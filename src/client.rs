use crate::cas::{compress_data, scan_workspace};
use crate::config::UnicompConfig;
use crate::framing::{recv_frame, send_frame};
use crate::language::detect_project;
use crate::protocol::*;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use tokio::net::TcpStream;

pub struct ClientRunner {
    host_addr: String,
    workdir: std::path::PathBuf,
    explicit_command: Vec<String>,
    project_cfg: Option<UnicompConfig>,
}

impl ClientRunner {
    pub fn new(
        host_addr: &str,
        workdir: &Path,
        explicit_command: Vec<String>,
        project_cfg: Option<UnicompConfig>,
    ) -> Self {
        Self {
            host_addr: host_addr.to_string(),
            workdir: workdir.to_path_buf(),
            explicit_command,
            project_cfg,
        }
    }

    pub async fn run(&self) -> Result<i32> {
        println!("[UniComp] Connecting to cluster host at {}...", self.host_addr);
        let mut stream = TcpStream::connect(&self.host_addr).await
            .with_context(|| format!("Failed to connect to UniComp host at {}", self.host_addr))?;

        let (mut reader, mut writer) = stream.split();

        // 1. Handshake
        send_frame(
            &mut writer,
            &ClientMessage::Handshake {
                client_version: "0.1.0".to_string(),
                auth_token: None,
            },
        ).await?;

        let ack: HostMessage = recv_frame(&mut reader).await?
            .context("Server disconnected during handshake")?;

        match ack {
            HostMessage::HandshakeAck { capabilities } => {
                println!(
                    "[UniComp] Connected to host '{}' ({} {}, {} cores, {} MB RAM)",
                    capabilities.hostname,
                    capabilities.os,
                    capabilities.arch,
                    capabilities.cpu_cores,
                    capabilities.total_memory_mb,
                );
            }
            HostMessage::Error { message } => {
                anyhow::bail!("Host rejected connection: {}", message);
            }
            other => anyhow::bail!("Unexpected handshake response: {:?}", other),
        }

        // 2. Scan Local Workspace and build Merkle Tree
        print!("[UniComp] Scanning workspace and building Merkle tree... ");
        io::stdout().flush()?;
        let manifest = scan_workspace(&self.workdir)?;
        println!("found {} files ({} dirs).", manifest.files.len(), manifest.directories.len());

        // 3. Query Missing Blobs on Host
        let all_digests: Vec<Digest> = manifest.files.iter().map(|f| f.digest.clone()).collect();
        send_frame(
            &mut writer,
            &ClientMessage::QueryMissingBlobs { digests: all_digests },
        ).await?;

        let missing_resp: HostMessage = recv_frame(&mut reader).await?
            .context("Server disconnected while checking missing blobs")?;

        let missing_digests = match missing_resp {
            HostMessage::MissingBlobs { digests } => digests,
            other => anyhow::bail!("Unexpected response to QueryMissingBlobs: {:?}", other),
        };

        if !missing_digests.is_empty() {
            println!(
                "[UniComp] Synchronizing {} missing blobs to cluster...",
                missing_digests.len()
            );

            // Upload each missing blob
            for (i, d) in missing_digests.iter().enumerate() {
                // Find local file corresponding to this digest
                if let Some(file_node) = manifest.files.iter().find(|f| f.digest == *d) {
                    let file_path = self.workdir.join(&file_node.relative_path);
                    let raw_bytes = std::fs::read(&file_path)?;
                    let compressed = compress_data(&raw_bytes)?;

                    send_frame(
                        &mut writer,
                        &ClientMessage::UploadBlobChunk {
                            digest: d.clone(),
                            data: compressed,
                            is_compressed: true,
                        },
                    ).await?;

                    // Await Ack
                    let _ack: HostMessage = recv_frame(&mut reader).await?
                        .context("Server disconnected during blob upload")?;

                    print!("\r[UniComp] Uploaded {}/{} blobs", i + 1, missing_digests.len());
                    io::stdout().flush()?;
                }
            }
            println!();
        } else {
            println!("[UniComp] All workspace blobs already cached in cluster! (Delta = 0 bytes)");
        }

        // 4. Determine Project & Execution Command
        let detected = detect_project(&self.workdir);
        let command = if !self.explicit_command.is_empty() {
            self.explicit_command.clone()
        } else {
            detected.default_command.clone()
        };

        let job_id = format!("job_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis());

        let project_name = self.workdir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".to_string());

        let mut env_vars = HashMap::new();
        let mut sync_artifacts = Vec::new();

        if let Some(ref pcfg) = self.project_cfg {
            if let Some(ref env) = pcfg.env {
                env_vars.extend(env.clone());
            }
            if let Some(ref sync) = pcfg.sync {
                if let Some(ref arts) = sync.artifacts {
                    sync_artifacts.extend(arts.clone());
                }
            }
        }

        let config = ExecutionConfig {
            job_id,
            project_name,
            language: detected.language,
            command,
            env_vars,
            sync_artifacts,
            pty_mode: false,
            term_cols: 80,
            term_rows: 24,
        };

        println!(
            "[UniComp] Starting execution on cluster [Language: {:?}] -> {:?}",
            config.language, config.command
        );

        send_frame(
            &mut writer,
            &ClientMessage::StartExecution {
                config,
                manifest,
            },
        ).await?;

        // 5. Stream Results in Real Time
        let mut final_exit_code = 0;
        let mut stdout_handle = io::stdout();
        let mut stderr_handle = io::stderr();

        while let Some(msg) = recv_frame::<_, HostMessage>(&mut reader).await? {
            match msg {
                HostMessage::PhaseUpdate { phase, message } => {
                    println!("[UniComp Phase: {:?}] {}", phase, message);
                }
                HostMessage::StdoutChunk { data } => {
                    stdout_handle.write_all(&data)?;
                    stdout_handle.flush()?;
                }
                HostMessage::StderrChunk { data } => {
                    stderr_handle.write_all(&data)?;
                    stderr_handle.flush()?;
                }
                HostMessage::ExecutionFinished {
                    exit_code,
                    duration_ms,
                    error_message,
                } => {
                    println!(
                        "\n[UniComp] Job completed on host in {:.2}s with exit code {}",
                        (duration_ms as f64) / 1000.0,
                        exit_code
                    );
                    if let Some(err) = error_message {
                        eprintln!("[UniComp Error] {}", err);
                    }
                    final_exit_code = exit_code;
                    break;
                }
                HostMessage::Error { message } => {
                    eprintln!("[UniComp Remote Error] {}", message);
                    final_exit_code = 1;
                    break;
                }
                _ => {}
            }
        }

        Ok(final_exit_code)
    }
}
