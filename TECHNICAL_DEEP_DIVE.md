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

## 2. Diagrama de Camadas da Arquitetura do Cluster

```mermaid
graph TD
    subgraph "Master / Client (Windows / macOS / Linux)"
        CLI["UniComp CLI (Binary Nativo Rust)"]
        Scanner["Merkle Scanner & Ignore Parser"]
        Scheduler["Cluster Scheduler & Node Router"]
        RawTerm["Terminal Raw Mode & Signal Interceptor"]
        GrpcClient["Tonic gRPC Client (mTLS)"]
        CLI --> Scanner
        CLI --> Scheduler
        Scheduler --> GrpcClient
        CLI --> RawTerm
    end

    subgraph "Rede de Alta Velocidade (gRPC / mTLS Mesh)"
        StreamPipe["HTTP/2 Multiplexed Bidirectional Stream<br/>(Protobuf: Stdin, Stdout, Stderr, PTY, Metrics)"]
        GrpcClient <===> StreamPipe
    end

    subgraph "Pool de Execução Distribuída (Cluster Hosts)"
        subgraph "Nó 1: Worker GPU (Desktop RTX / CUDA)"
            Daemon1["UniComp Daemon"]
            Cas1["CAS Local (Zstd)"]
            Proc1["Workload GPU (PyTorch / CUDA)"]
            Daemon1 --> Cas1 --> Proc1
        end

        subgraph "Nó 2: Worker High-Core (Servidor 32+ Cores)"
            Daemon2["UniComp Daemon"]
            Cas2["CAS Local (Zstd)"]
            Proc2["Builds Pesados (Rust / C++)"]
            Daemon2 --> Cas2 --> Proc2
        end

        subgraph "Nó 3: Worker Geral / Edge (Homelab / Mini-PC)"
            Daemon3["UniComp Daemon"]
            Cas3["CAS Local (Zstd)"]
            Proc3["Pipelines (Node.js / JVM / Python)"]
            Daemon3 --> Cas3 --> Proc3
        end
    end

    StreamPipe <===> Daemon1
    StreamPipe <===> Daemon2
    StreamPipe <===> Daemon3
```
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
