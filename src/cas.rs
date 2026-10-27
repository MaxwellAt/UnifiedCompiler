use crate::protocol::{Digest, FileNode, WorkspaceManifest};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use ignore::WalkBuilder;
use sha2::{Digest as ShaDigest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub struct CasStore {
    root_dir: PathBuf,
}

impl CasStore {
    pub fn new<P: AsRef<Path>>(root_dir: P) -> Result<Self> {
        let root = root_dir.as_ref().to_path_buf();
        fs::create_dir_all(&root).context("Failed to create CAS root directory")?;
        Ok(Self { root_dir: root })
    }

    pub fn blob_path(&self, digest: &Digest) -> PathBuf {
        let prefix = if digest.sha256.len() >= 2 {
            &digest.sha256[..2]
        } else {
            "xx"
        };
        self.root_dir.join(prefix).join(&digest.sha256)
    }

    pub fn has_blob(&self, digest: &Digest) -> bool {
        self.blob_path(digest).is_file()
    }

    pub fn store_blob(&self, digest: &Digest, raw_data: &[u8]) -> Result<()> {
        let dest = self.blob_path(digest);
        if dest.is_file() {
            return Ok(());
        }

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        // Temporary file write then atomic rename to prevent partial writes
        let tmp_path = dest.with_extension("tmp");
        fs::write(&tmp_path, raw_data)?;
        fs::rename(tmp_path, dest)?;
        Ok(())
    }

    pub fn read_blob(&self, digest: &Digest) -> Result<Vec<u8>> {
        let path = self.blob_path(digest);
        fs::read(&path).with_context(|| format!("Blob not found: {}", digest.sha256))
    }

    pub fn materialize_workspace(
        &self,
        manifest: &WorkspaceManifest,
        target_dir: &Path,
    ) -> Result<()> {
        fs::create_dir_all(target_dir)?;

        for dir in &manifest.directories {
            fs::create_dir_all(target_dir.join(dir))?;
        }

        for node in &manifest.files {
            let file_target = target_dir.join(&node.relative_path);
            if let Some(parent) = file_target.parent() {
                fs::create_dir_all(parent)?;
            }

            let blob = self.read_blob(&node.digest)?;
            fs::write(&file_target, blob)?;

            #[cfg(unix)]
            if node.is_executable {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&file_target)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&file_target, perms)?;
            }
        }

        Ok(())
    }
}

pub fn hash_bytes(data: &[u8]) -> Digest {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hex::encode(hasher.finalize());
    Digest {
        sha256: hash,
        size_bytes: data.len() as u64,
    }
}

pub fn compress_data(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

pub fn decompress_data(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

/// Builds a workspace manifest from a local directory, respecting gitignore and custom exclusions.
pub fn scan_workspace<P: AsRef<Path>>(root: P) -> Result<WorkspaceManifest> {
    let root = root.as_ref();
    let mut files = Vec::new();
    let mut directories = Vec::new();

    let walker = WalkBuilder::new(root)
        .standard_filters(true)
        .add_custom_ignore_filename(".unicompignore")
        .hidden(true)
        .parents(true)
        .build();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        if path == root {
            continue;
        }

        let relative = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");

        // Skip heavy directories that should never be sent over the wire
        if relative.starts_with(".git/")
            || relative.starts_with("target/")
            || relative.starts_with("node_modules/")
            || relative.starts_with(".venv/")
            || relative.starts_with("__pycache__/")
            || relative.starts_with(".gradle/")
            || relative.starts_with("build/")
        {
            continue;
        }

        let file_type = entry.file_type().context("Missing file type")?;
        if file_type.is_dir() {
            directories.push(relative);
        } else if file_type.is_file() {
            let data = fs::read(path)?;
            let digest = hash_bytes(&data);

            let is_executable = {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let perms = fs::metadata(path)?.permissions();
                    perms.mode() & 0o111 != 0
                }
                #[cfg(not(unix))]
                {
                    relative.ends_with(".exe") || relative.ends_with(".sh") || relative.ends_with(".bat")
                }
            };

            files.push(FileNode {
                relative_path: relative,
                digest,
                is_executable,
            });
        }
    }

    Ok(WorkspaceManifest { files, directories })
}
