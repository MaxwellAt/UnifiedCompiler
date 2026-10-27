# UniComp (Universal Distributed Compiler & Remote Execution Engine)
## Arquitetura de Baixo Nível e Especificação Técnica

### 1. Resumo Executivo
O **UniComp** é um sistema unificado de compilação, resolução de dependências e execução remota com streaming bidirecional em tempo real. Projetado para proporcionar ao desenvolvedor a experiência exata de execução local (`localhost`), ele abstrai diferenças de hardware (CPU arch), sistema operacional (Windows/macOS vs. Linux), topologia de rede e cadeias de dependências de linguagens como Rust, Python, Node.js e JVM.

---

### 2. Análise de Armadilhas Técnicas e Decisões de Engenharia

#### 2.1. Diferença de Arquitetura e SO (Master vs. Host)
* **Cenário Típico:** Desenvolvedor em Windows x86_64 ou macOS Apple Silicon (ARM64) submetendo tarefas para nós em Linux x86_64 ou Linux ARM64 (Graviton/Ampere).
* **Armadilha da Cross-Compilação Local:**
  - Compilar binários C/Rust localmente para rodar no host falha miseravelmente quando há dependências C nativas (`bindgen`, `cc`, `openssl-sys`, `libtorch`), incompatibilidade de `glibc` (ex: host com `GLIBC_2.35` e master compilando com `GLIBC_2.38`), ou flags de CPU (`avx2`, `avx512`, `neon`).
  - No Python/Node, compilar wheels nativas ou addons N-API no Mac ARM64 gera binários Mach-O incompatíveis com o Linux ELF.
* **Armadilha da Compilação Remota Ingênua:**
  - Transferir todo o código via rsync/tarball e rodar `cargo build` ou `npm install` do zero no nó worker gera latência inaceitável (cold starts de minutos).
* **Solução UniComp (Hybrid CAS + Remote Deterministic Build):**
  - **Content-Addressable Storage (CAS):** O master decompõe a workspace em um grafo de Merkle (árvore de hashes SHA-256). Apenas os blobs delta modificados são transferidos (compactados via Zstandard).
  - **Remote Incremental Compilation:** A compilação ou resolução de dependências pesadas ocorre diretamente no host, mas montada sobre volumes persistentes de cache de compilação (`sccache`, `uv-cache`, `pnpm-store`, `gradle-cache`) isolados por namespace de projeto.

#### 2.2. Dependências de Runtime e Linguagens
| Linguagem | Armadilha de Baixo Nível | Solução Arquitetural UniComp |
| :--- | :--- | :--- |
| **Python** | Virtualenvs são vinculados a caminhos absolutos e SO (shebangs em `bin/`, libs em `site-packages`). Impossível sincronizar `.venv` do Windows para Linux. | Envio de manifesto (`pyproject.toml` / `uv.lock`). O host utiliza `uv` nativo para instanciar/atualizar o ambiente isolado em milissegundos (< 80ms) a partir de um cache local de wheels Linux. |
| **Node.js** | Módulos com bindings nativos C++ (`node-gyp`, Sharp, SQLite3, Canvas) falham se copiados de OS diferente; `node_modules` com symlinks quebram entre Windows e Linux. | Sincroniza apenas código e lockfile (`pnpm-lock.yaml` / `package-lock.json`). Host utiliza `pnpm` com store global via hardlinks. Addons nativos compilados para a libc do host. |
| **Rust / C** | Mismatch de ABI C (`glibc` vs `musl`), flags de CPU (`march=native`), ferramentas de linker (`lld`, `mold`, `binutils`). | Worker executa `cargo check` / `cargo build` reutilizando cache persistente compartilhado (`sccache` + target cache com overlay) com compilação paralela distribuída. |
| **JVM (Java/Kotlin)** | Bytecode `.class` é agnóstico a SO, mas JNI nativo (ex: RocksDB, Netty Epoll) requer shared libraries específicas (.so Linux). Java Major Version mismatch (ex: runtime local Java 21 vs host Java 17). | Handshake de compatibilidade de JVM target version. Sincronização de classpath / fat-jar ou build remoto via Gradle Daemon persistente no worker. |

#### 2.3. Comunicação e Streaming de I/O em Tempo Real
* **Requisitos:**
  - Streaming em sub-milissegundos de `stdout` e `stderr` separados (para preservar redirects `> out.txt 2> err.txt`).
  - Suporte completo a PTY (Pseudo-Terminal): sequências ANSI, cores, curses, redimensionamento dinâmico de janela (`SIGWINCH`), modo raw do terminal para comandos interativos (ex: `read -s`, shells, repls).
  - Repasse imediato de sinais POSIX/Windows (`SIGINT` / Ctrl+C, `SIGTERM`, `SIGKILL`).
* **Comparativo de Protocolos:**
  - **WebSocket:** Não possui multiplexação de canais nativa (necessitaria framing ad-hoc manual) nem tipagem estrita de schema.
  - **gRPC Bidirecional (HTTP/2):** Padrão de ouro industrial (usado por Bazel Remote Execution, Kubernetes CRI e Ray). Garante contratos estritos via Protocol Buffers v3, controle de fluxo por stream (window size) e suporte a mTLS nativo.
  - **QUIC / HTTP/3:** Superior para mobilidade de IP e ausência de Head-of-Line blocking em redes instáveis, porém com restrições corporativas de portas UDP.
* **Decisão UniComp:** gRPC bidirecional sobre HTTP/2 com mTLS e fallback/suporte futuro a QUIC (via `quiche` / `s2n-quic`).

#### 2.4. Isolamento e Segurança no Host
* **Ameaças:** Código arbitrário enviado por clientes pode conter loops infinitos, fork bombs, consumo de toda a memória RAM, leitura de segredos locais do host ou tráfego malicioso na VPC.
* **Matriz de Sandbox:**
  1. **Nível 1 (Dev Trusted - Single Tenant):** `cgroups v2` + Linux Namespaces (`bubblewrap` / `bwrap`). Latência de início < 5ms. Isola PID, Mount, Network (veth ou loopback), IPC e define limites de memória/CPU.
  2. **Nível 2 (Multi-Tenant Container):** `containerd` / `runc` com rootfs customizado pré-aquecido e volumes OverlayFS descartáveis. Latência: 80-150ms.
  3. **Nível 3 (Multi-Tenant Untrusted - Zero Trust):** MicroVMs baseadas em KVM (`Firecracker` ou `Cloud-Hypervisor`). Isolamento completo em nível de kernel com boot < 25ms.

---

### 3. Especificação da Arquitetura do Sistema

```
                  +-----------------------------------+
                  |        UniComp Client (CLI)       |
                  |     (Windows / macOS / Linux)     |
                  +-----------------+-----------------+
                                    |
          1. Workspace Merkle Walk  | 2. Manifest & Env Probe
          3. Missing Blobs Upload   | 4. Bidirectional Stream (gRPC/mTLS)
                                    v
                  +-----------------------------------+
                  |      UniComp Daemon (Worker)      |
                  |           (Linux Host)            |
                  +-----------------+-----------------+
                                    |
     +------------------------------+------------------------------+
     |                              |                              |
     v                              v                              v
+------------+              +---------------+              +---------------+
| Content    |              |  Execution    |              |  Ephemeral    |
| Addressed  |              |  Supervisor   |              |  Sandbox      |
| Store(CAS) |              | (PTY & Stats) |              | (cgroup/bwrap)|
+------------+              +---------------+              +---------------+
```

#### 3.1. Pipeline de Execução Passo a Passo
1. **Probe Local & Merkle Tree:**
   - O cliente lê `.unicompignore` e `.gitignore`.
   - Gera um grafo de arquivos e hashes SHA-256 dos conteúdos locais.
   - Analisa o manifesto da linguagem (`Cargo.toml`, `pyproject.toml`, `package.json`, etc.) e variáveis de ambiente relevantes.
2. **Handshake e Sessão:**
   - Cliente conecta ao daemon via gRPC sobre mTLS.
   - Host responde com capacidades: arquitetura (`x86_64`, `aarch64`), toolchains instaladas (`rustc 1.82`, `python 3.12`, `node 22`), GPU disponível (CUDA/ROCm), e cotas de cgroup.
3. **Sincronização Diferencial (CAS Sync):**
   - Cliente envia a lista de hashes para o Host (`CheckBlobsRequest`).
   - Host responde com os hashes que não possui em cache.
   - Cliente transmite apenas os blobs ausentes compactados com Zstandard nível 3 (`UploadBlobsRequest`).
4. **Montagem do Ambiente no Host (OverlayFS):**
   - Camada Base: Rootfs imutável do sistema / toolchains compartilhadas.
   - Camada CAS: Workspace do projeto montada via hardlinks / read-only bind mount.
   - Camada Superior (Upper/Workdir): Camada efêmera Copy-On-Write (RAM-backed `tmpfs` ou SSD NVMe).
5. **Resolução de Dependências & Build:**
   - Host aciona gerenciador de pacote correspondente apontando para caches globais no host.
   - Python: `uv sync --frozen` montado sobre o cache global.
   - Rust: `cargo build` com `CARGO_TARGET_DIR` indexado e cache compartilhado via `sccache`.
   - Node: `pnpm install --frozen-lockfile` utilizando o content-addressable store local do host.
6. **Spawn do Processo e Streaming Bidirecional:**
   - Se interativo (TTY): Host aloca par PTY (`openpty`), configura modo raw e repassa janelas e sinais.
   - Stream de saída: Host envia chunks binários de `STDOUT`, `STDERR`, e `METRICS` (CPU%, RAM RSS, I/O bytes).
   - Stream de entrada: Cliente transmite `STDIN`, `RESIZE_TTY(cols, rows)`, e `SIGNAL(SIGINT/SIGTERM)`.
7. **Término e Teardown:**
   - Host aguarda término (`waitpid`), envia `ExecutionExit(code, signal, runtime_metrics)`.
   - Camada efêmera é destruída ou preservada conforme flag de persistência/artefatos.
   - Cliente restaura o terminal local e encerra com o mesmo código de saída emitido pelo processo remoto.
