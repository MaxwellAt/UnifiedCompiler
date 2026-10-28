use crate::cas::{decompress_data, CasStore};
use crate::framing::{recv_frame, send_frame};
use crate::protocol::*;
use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::process::Command;

pub struct DaemonServer {
    bind_addr: String,
    cas: Arc<CasStore>,
    work_dir: PathBuf,
}

impl DaemonServer {
    pub fn new(bind_addr: &str, data_dir: &Path) -> Result<Self> {
        let cas_dir = data_dir.join("cas");
        let work_dir = data_dir.join("workspaces");
        std::fs::create_dir_all(&work_dir)?;

        Ok(Self {
            bind_addr: bind_addr.to_string(),
            cas: Arc::new(CasStore::new(cas_dir)?),
            work_dir,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.bind_addr).await
            .with_context(|| format!("Failed to bind to {}", self.bind_addr))?;

        println!("[UniComp Daemon] Listening on {}", self.bind_addr);
        println!("[UniComp Daemon] Host capabilities probed:");
        let caps = probe_capabilities();
        println!("  - OS: {} ({})", caps.os, caps.arch);
        println!("  - Cores: {}, Memory: {} MB", caps.cpu_cores, caps.total_memory_mb);
        for t in &caps.toolchains {
            println!("  - Toolchain: {} ({}) -> {}", t.name, t.version, t.path);
        }

        loop {
            match listener.accept().await {
                Ok((socket, remote_addr)) => {
                    println!("[UniComp Daemon] New connection from {}", remote_addr);
                    let cas = Arc::clone(&self.cas);
                    let work_dir = self.work_dir.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_client(socket, cas, work_dir).await {
                            eprintln!("[UniComp Daemon] Error handling client {}: {:?}", remote_addr, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("[UniComp Daemon] Accept error: {:?}", e);
                }
            }
        }
    }
}

pub fn probe_capabilities() -> NodeCapabilities {
    let os = env::consts::OS.to_string();
    let arch = env::consts::ARCH.to_string();
    let hostname = hostname_string();
    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let mut toolchains = Vec::new();

    // Check rustc / cargo
    if let Ok(out) = std::process::Command::new("rustc").arg("--version").output() {
        if out.status.success() {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            toolchains.push(ToolchainInfo {
                name: "rust".to_string(),
                version: ver,
                path: "rustc".to_string(),
            });
        }
    }

    // Check python
    if let Ok(out) = std::process::Command::new("python").arg("--version").output() {
        if out.status.success() {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            toolchains.push(ToolchainInfo {
                name: "python".to_string(),
                version: ver,
                path: "python".to_string(),
            });
        }
    }

    // Check node
    if let Ok(out) = std::process::Command::new("node").arg("--version").output() {
        if out.status.success() {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            toolchains.push(ToolchainInfo {
                name: "node".to_string(),
                version: ver,
                path: "node".to_string(),
            });
        }
    }

    // Check java
    if let Ok(out) = std::process::Command::new("java").arg("-version").output() {
        let err_or_out = if out.status.success() {
            String::from_utf8_lossy(&out.stderr).to_string()
        } else {
            String::new()
        };
        let first_line = err_or_out.lines().next().unwrap_or("Java installed").to_string();
        toolchains.push(ToolchainInfo {
            name: "jvm".to_string(),
            version: first_line,
            path: "java".to_string(),
        });
    }

    // Check go
    if let Ok(out) = std::process::Command::new("go").arg("version").output() {
        if out.status.success() {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            toolchains.push(ToolchainInfo {
                name: "go".to_string(),
                version: ver,
                path: "go".to_string(),
            });
        }
    }

    NodeCapabilities {
        os,
        arch,
        hostname,
        cpu_cores,
        total_memory_mb: 16384,
        toolchains,
        server_version: "0.1.0".to_string(),
    }
}

fn hostname_string() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unicomp-host".to_string())
}

async fn handle_client(
    mut socket: TcpStream,
    cas: Arc<CasStore>,
    work_dir: PathBuf,
) -> Result<()> {
    let (mut reader, mut writer) = socket.split();

    // 1. Initial Handshake
    let handshake: Option<ClientMessage> = recv_frame(&mut reader).await?;
    match handshake {
        Some(ClientMessage::Handshake { client_version, .. }) => {
            println!("[Daemon] Client connected (client_version: {})", client_version);
            let ack = HostMessage::HandshakeAck {
                capabilities: probe_capabilities(),
            };
            send_frame(&mut writer, &ack).await?;
        }
        other => {
            anyhow::bail!("Expected Handshake message, got: {:?}", other);
        }
    }

    // 2. Command / Execution Loop


    while let Some(msg) = recv_frame::<_, ClientMessage>(&mut reader).await? {
        match msg {
            ClientMessage::QueryMissingBlobs { digests } => {
                let mut missing = Vec::new();
                for d in digests {
                    if !cas.has_blob(&d) {
                        missing.push(d);
                    }
                }
                send_frame(&mut writer, &HostMessage::MissingBlobs { digests: missing }).await?;
            }

            ClientMessage::UploadBlobChunk {
                digest,
                data,
                is_compressed,
            } => {
                let raw_bytes = if is_compressed {
                    decompress_data(&data)?
                } else {
                    data
                };
                cas.store_blob(&digest, &raw_bytes)?;
                send_frame(&mut writer, &HostMessage::BlobsAck { stored_count: 1 }).await?;
            }

            ClientMessage::StartExecution { config, manifest } => {
                let job_dir = work_dir.join(&config.job_id);


                send_frame(
                    &mut writer,
                    &HostMessage::PhaseUpdate {
                        phase: ExecutionPhase::SynchronizingWorkspace,
                        message: format!("Materializing workspace for job {}...", config.job_id),
                    },
                ).await?;

                cas.materialize_workspace(&manifest, &job_dir)?;

                send_frame(
                    &mut writer,
                    &HostMessage::PhaseUpdate {
                        phase: ExecutionPhase::Running,
                        message: format!("Executing command: {:?}", config.command),
                    },
                ).await?;

                let start_time = Instant::now();
                let exit_code = execute_job(&config, &job_dir, &mut writer).await?;
                let duration_ms = start_time.elapsed().as_millis() as u64;

                send_frame(
                    &mut writer,
                    &HostMessage::ExecutionFinished {
                        exit_code,
                        duration_ms,
                        error_message: None,
                    },
                ).await?;

                // Cleanup workspace
                let _ = std::fs::remove_dir_all(&job_dir);
                break;
            }

            ClientMessage::CancelExecution => {
                println!("[Daemon] Client requested cancellation");
                break;
            }

            _ => {}
        }
    }

    Ok(())
}

async fn execute_job<W>(
    config: &ExecutionConfig,
    job_dir: &Path,
    writer: &mut W,
) -> Result<i32>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    if config.command.is_empty() {
        return Ok(0);
    }

    #[cfg(windows)]
    let (real_prog, real_args): (String, Vec<String>) = {
        let raw = &config.command[0];
        let raw_lower = raw.to_lowercase();
        if raw_lower.ends_with(".cmd")
            || raw_lower.ends_with(".bat")
            || raw_lower == "npm"
            || raw_lower == "yarn"
            || raw_lower == "pnpm"
            || raw_lower == "npx"
            || raw_lower == "pip"
        {
            ("cmd.exe".to_string(), {
                let mut v = vec!["/C".to_string(), raw.clone()];
                v.extend_from_slice(&config.command[1..]);
                v
            })
        } else {
            (raw.clone(), config.command[1..].to_vec())
        }
    };

    #[cfg(not(windows))]
    let (real_prog, real_args): (String, Vec<String>) = {
        (config.command[0].clone(), config.command[1..].to_vec())
    };

    let mut cmd = Command::new(&real_prog);
    cmd.args(&real_args);
    cmd.current_dir(job_dir);
    cmd.envs(&config.env_vars);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().with_context(|| format!("Failed to spawn {} (command: {:?})", real_prog, config.command))?;

    let mut child_stdout = child.stdout.take().context("Missing child stdout")?;
    let mut child_stderr = child.stderr.take().context("Missing child stderr")?;

    // Streaming stdout and stderr asynchronously
    let mut stdout_buf = [0u8; 4096];
    let mut stderr_buf = [0u8; 4096];

    loop {
        tokio::select! {
            res = child_stdout.read(&mut stdout_buf) => {
                match res {
                    Ok(0) => break, // stdout closed
                    Ok(n) => {
                        let chunk = stdout_buf[..n].to_vec();
                        send_frame(writer, &HostMessage::StdoutChunk { data: chunk }).await?;
                    }
                    Err(e) => {
                        eprintln!("[Daemon] Stdout read error: {:?}", e);
                        break;
                    }
                }
            }
            res = child_stderr.read(&mut stderr_buf) => {
                match res {
                    Ok(0) => {},
                    Ok(n) => {
                        let chunk = stderr_buf[..n].to_vec();
                        send_frame(writer, &HostMessage::StderrChunk { data: chunk }).await?;
                    }
                    Err(_) => break,
                }
            }
            status = child.wait() => {
                let exit_status = status?;
                return Ok(exit_status.code().unwrap_or(0));
            }
        }
    }

    let status = child.wait().await?;
    Ok(status.code().unwrap_or(0))
}
