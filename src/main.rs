mod cas;
mod client;
mod framing;
mod language;
mod protocol;
mod server;

use anyhow::Result;
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    match args[1].as_str() {
        "daemon" | "host" | "server" => {
            let mut bind_addr = "0.0.0.0:9876".to_string();
            let mut data_dir = default_data_dir();

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--bind" | "-b" => {
                        if i + 1 < args.len() {
                            bind_addr = args[i + 1].clone();
                            i += 2;
                            continue;
                        }
                    }
                    "--data-dir" | "-d" => {
                        if i + 1 < args.len() {
                            data_dir = PathBuf::from(&args[i + 1]);
                            i += 2;
                            continue;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }

            let srv = server::DaemonServer::new(&bind_addr, &data_dir)?;
            srv.run().await?;
        }

        "run" | "exec" => {
            let mut host_addr = "127.0.0.1:9876".to_string();
            let mut workdir = env::current_dir()?;
            let mut explicit_command = Vec::new();

            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--host" | "-h" => {
                        if i + 1 < args.len() {
                            host_addr = args[i + 1].clone();
                            i += 2;
                            continue;
                        }
                    }
                    "--dir" | "-C" => {
                        if i + 1 < args.len() {
                            workdir = PathBuf::from(&args[i + 1]);
                            i += 2;
                            continue;
                        }
                    }
                    "--" => {
                        explicit_command = args[i + 1..].to_vec();
                        break;
                    }
                    arg if !arg.starts_with('-') && explicit_command.is_empty() => {
                        explicit_command = args[i..].to_vec();
                        break;
                    }
                    _ => {}
                }
                i += 1;
            }

            let client = client::ClientRunner::new(&host_addr, &workdir, explicit_command);
            let exit_code = client.run().await?;
            std::process::exit(exit_code);
        }

        "check" | "status" | "info" => {
            let mut host_addr = "127.0.0.1:9876".to_string();
            if args.len() >= 3 && !args[2].starts_with('-') {
                host_addr = args[2].clone();
            } else if let Some(pos) = args.iter().position(|a| a == "--host" || a == "-h") {
                if pos + 1 < args.len() {
                    host_addr = args[pos + 1].clone();
                }
            }

            run_check(&host_addr).await?;
        }

        "help" | "--help" | "-h" => {
            print_help();
        }

        other => {
            eprintln!("[UniComp] Unknown command '{}'", other);
            print_help();
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn run_check(host_addr: &str) -> Result<()> {
    use framing::{recv_frame, send_frame};
    use protocol::{ClientMessage, HostMessage};
    use tokio::net::TcpStream;

    println!("[UniComp Check] Probing cluster host at {}...", host_addr);
    let mut stream = TcpStream::connect(host_addr).await?;
    let (mut reader, mut writer) = stream.split();

    send_frame(
        &mut writer,
        &ClientMessage::Handshake {
            client_version: "0.1.0".to_string(),
            auth_token: None,
        },
    ).await?;

    let ack: HostMessage = recv_frame(&mut reader).await?
        .ok_or_else(|| anyhow::anyhow!("Host disconnected"))?;

    match ack {
        HostMessage::HandshakeAck { capabilities } => {
            println!("============================================================");
            println!("  UniComp Host Status: ONLINE");
            println!("============================================================");
            println!("  Hostname:         {}", capabilities.hostname);
            println!("  Operating System: {} ({})", capabilities.os, capabilities.arch);
            println!("  CPU Cores:        {}", capabilities.cpu_cores);
            println!("  Memory:           {} MB", capabilities.total_memory_mb);
            println!("  Daemon Version:   {}", capabilities.server_version);
            println!("------------------------------------------------------------");
            println!("  Installed Toolchains (Ready for Remote Execution):");
            if capabilities.toolchains.is_empty() {
                println!("    (None detected in system PATH)");
            } else {
                for t in &capabilities.toolchains {
                    println!("    * {:<8} : {} [{}]", t.name, t.version, t.path);
                }
            }
            println!("============================================================");
        }
        HostMessage::Error { message } => {
            eprintln!("[UniComp Error] {}", message);
        }
        other => {
            eprintln!("[UniComp] Unexpected response: {:?}", other);
        }
    }

    Ok(())
}

fn default_data_dir() -> PathBuf {
    if let Ok(home) = env::var("USERPROFILE").or_else(|_| env::var("HOME")) {
        PathBuf::from(home).join(".unicomp")
    } else {
        PathBuf::from(".unicomp")
    }
}

fn print_help() {
    println!(
        r#"UniComp - Universal Distributed Compiler and Remote Execution Engine
Version 0.1.0

USAGE:
    unicomp <COMMAND> [OPTIONS]

COMMANDS:
    daemon, host, server   Starts the UniComp worker daemon on a cluster host / remote desktop
    run, exec              Executes the current project on a remote UniComp host
    check, status          Inspects remote cluster capabilities, toolchains, and hardware specs
    help                   Prints this help message

OPTIONS (run):
    --host, -h <ADDR>      Remote cluster host address (default: 127.0.0.1:9876)
    --dir, -C <PATH>       Path to local workspace (default: current directory)
    -- <CMD...>            Explicit command override (e.g. -- cargo test, -- python custom.py)

OPTIONS (daemon):
    --bind, -b <ADDR>      Listen address and port (default: 0.0.0.0:9876)
    --data-dir, -d <PATH>  Storage directory for Content-Addressed Storage & Workspaces

EXAMPLES:
    # 1. On Cluster / Host machine:
    unicomp daemon --bind 0.0.0.0:9876

    # 2. On Developer / Master machine:
    unicomp check --host 192.168.1.50:9876
    unicomp run --host 192.168.1.50:9876
    unicomp run --host 192.168.1.50:9876 -- python heavy_training.py
    unicomp run --host 192.168.1.50:9876 -- cargo run --release
"#
    );
}
