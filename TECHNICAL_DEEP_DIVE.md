# UniComp: Análise Técnica e Arquitetura de Baixo Nível

## 1. Topologia e Fluxo de Dados Bidirecional

```mermaid
sequenceDiagram
    autonumber
    actor Dev as Desenvolvedor (CLI)
    participant Client as UniComp Client (Local)
    participant Host as UniComp Daemon (Worker)
    participant CAS as Content-Addressed Storage
    participant Sandbox as Sandbox / OCI / MicroVM
    participant Process as Processo Alvo (PTY)

    Dev->>Client: unicomp run cargo run --release
    Note over Client: 1. Hash Merkle da Workspace<br/>2. Detecção de Toolchain & Manifests
    Client->>Host: RPC GetNodeCapabilities()
    Host-->>Client: Capabilities(OS, Arch, Toolchains, Driver: bwrap)
    Client->>Host: RPC FindMissingBlobs(digests)
    Host-->>Client: MissingBlobs([sha256_hash_1, ...])
    Client->>Host: RPC UploadBlobStream(zstd chunks)
    Host->>CAS: Salva blobs em /var/lib/unicomp/cas

    Client->>Host: RPC Execute(Stream Init: Config, PTY, Limits)
    Host->>Sandbox: Monta OverlayFS (Lower=CAS, Upper=Tmpfs CoW)
    Host->>Sandbox: Aplica Cgroups v2 (CPU, Mem, PIDs)
    Host->>Sandbox: Hydration de Dependências (uv / pnpm / sccache)
    Host->>Process: Spawn Processo via openpty()
    
    par Streaming Bidirecional em Tempo Real
        Process-->>Host: stdout / stderr (ANSI bytes)
        Host-->>Client: HostStreamMessage(stdout_chunk, metrics)
        Client-->>Dev: Renderiza no Terminal Local (Raw Mode)
        Dev->>Client: Input do Usuário (Teclas / Caracteres)
        Client->>Host: ClientStreamMessage(stdin_chunk)
        Host->>Process: Escreve no PTY Master
    and Sinais e Redimensionamento
        Dev->>Client: Redimensiona Janela / Ctrl+C
        Client->>Host: ClientStreamMessage(resize_pty / SIGINT)
        Host->>Process: ioctl(TIOCSWINSZ) / kill(-PID, SIGINT)
    end

    Process-->>Host: Process Exited (exit_code=0)
    Host->>Sandbox: Limpeza de Upperdir efêmero
    Host-->>Client: HostStreamMessage(ExecutionExit)
    Client-->>Dev: Encerra com exit code original
```

---

## 2. Diagrama de Camadas da Arquitetura

```mermaid
graph TD
    subgraph "Master / Client (Windows / macOS / Linux)"
        CLI["UniComp CLI (Binary Nativo Rust)"]
        Scanner["Merkle Scanner & Ignore Parser"]
        RawTerm["Terminal Raw Mode & Signal Interceptor"]
        GrpcClient["Tonic gRPC Client (mTLS)"]
        CLI --> Scanner
        CLI --> RawTerm
        CLI --> GrpcClient
    end

    subgraph "Rede de Alta Velocidade (gRPC / mTLS)"
        StreamPipe["HTTP/2 Multiplexed Bidirectional Stream<br/>(Protobuf: Stdin, Stdout, Stderr, PTY, Metrics)"]
        GrpcClient <===> StreamPipe
    end

    subgraph "Host / Cluster Worker (Linux x86_64 / ARM64)"
        Daemon["UniComp Daemon (Rust/Tokio)"]
        StreamPipe <===> Daemon
        CasEngine["Content-Addressed Storage (CAS Engine)<br/>Zstd Compression + Content Deduplication"]
        DepHydrator["Dependency Hydrator<br/>(uv cache / pnpm store / sccache / gradle daemon)"]
        SandboxMgr["Sandbox & Isolation Manager<br/>(Bubblewrap / Cgroups v2 / Namespaces)"]
        PtyMultiplexer["PTY Multiplexer & I/O Loop<br/>(nix::pty, termios, ioctl TIOCSWINSZ)"]
        
        Daemon --> CasEngine
        Daemon --> DepHydrator
        Daemon --> SandboxMgr
        Daemon --> PtyMultiplexer
        
        subgraph "Execução Isolada (Sandbox Jail)"
            Overlay["OverlayFS: Base Tools + Workspace Mount + Tmpfs Upper"]
            TargetProc["Processo de Execução Alvo<br/>(Python / Node / Rust / JVM / C)"]
            SandboxMgr --> Overlay
            PtyMultiplexer --> TargetProc
            Overlay -.-> TargetProc
        end
    end
```

---

## 3. Matrizes de Comparação e Trade-Offs

### 3.1. Estratégia de Build: Local vs. Remoto vs. Híbrido CAS
| Critério | Cross-Compilação Local | Compilação Remota Simples | UniComp Híbrido (CAS + Remote Incremental) |
| :--- | :--- | :--- | :--- |
| **Portabilidade Master/Host** | Péssima (quebra com C-bindings, glibc e SO) | Excelente (compila nativo no Linux) | **Excelente (compila nativo no Linux)** |
| **Uso de Banda de Rede** | Baixo (envia apenas binário gerado) | Alto (envia arquivos e dependências sempre) | **Ultra-baixo (envia apenas deltas via Merkle CAS)** |
| **Tempo de Execução (Cold)** | Rápido no host, mas lento/impossível no local | Lento (recria ambientes `npm`/`pip`/`cargo`) | **Rápido (hydratation via caches pré-aquecidos)** |
| **Tempo de Execução (Hot)** | Médio (recompilação local constante) | Médio (locks e compilações completas) | **Sub-segundo (sccache + uv instantâneo)** |
| **Complexidade de Toolchain** | Altíssima (requer toolchains cross locais) | Baixa no client, média no host | **Mínima no client (client não precisa de toolchains)** |

### 3.2. Mecanismo de Sandbox e Isolamento
| Mecanismo | Overhead de CPU/Memória | Latência de Spawn | Segurança de Isolamento | Prós & Contras no UniComp |
| :--- | :--- | :--- | :--- | :--- |
| **Processo Direto (`fork/exec`)** | 0% | < 1ms | Nenhuma (perigoso) | Risco inaceitável de fork-bombs e invasão de host. |
| **Bubblewrap (`bwrap`) + cgroups v2** | < 0.1% | 2ms - 5ms | Alta (Namespaces + Seccomp + Cgroups) | **Recomendado para clusters de dev e times confiáveis.** |
| **Containers (`containerd` / `runc`)** | Baixo | 80ms - 200ms | Padrão OCI / Container | Ótimo para ambientes pré-customizados via Dockerfiles. |
| **MicroVMs (`Firecracker`)** | ~5MB RAM por VM | 15ms - 30ms | Máxima (Isolamento de Kernel KVM) | **Recomendado para nós compartilhados multi-tenant ou código não-confiável.** |

### 3.3. Protocolo de Streaming e Transporte
| Protocolo | Latência I/O | Multiplexação | Tipagem / Contrato | Decisão Arquitetural |
| :--- | :--- | :--- | :--- | :--- |
| **gRPC (HTTP/2) + mTLS** | < 1ms em LAN / < 25ms em WAN | Nativa (Streams bidirecionais por chamada) | Estrita (Protobuf v3) | **Padrão adotado no Core do UniComp.** |
| **WebSocket + TLS** | Baixa | Não-nativa (precisa de framing manual) | Não tipado (JSON/Raw) | Complexo para multiplexar stdin/stdout/resize/signals. |
| **QUIC / HTTP/3** | Mínima (0-RTT, sem TCP HoL) | Nativa em nível de pacote UDP | Alta | **Camada de transporte secundária para WAN/Wi-Fi instável.** |
