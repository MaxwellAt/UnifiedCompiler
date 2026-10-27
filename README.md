# UniComp (Compilador Único / Universal Remote Execution Engine)

> **Ambiente Unificado de Compilação e Execução Remota em Clusters e Desktops Distribuídos.**  
> Inspirado na filosofia do *Databricks Connect*, *Bazel Remote Execution API (REAPI)* e *Modal*. O código e o controle permanecem no seu computador; o poder de processamento, compilação pesada e memória residem no cluster/host remoto, com streaming de I/O em tempo real diretamente para o seu terminal local.

---

## 🚀 Como Funciona o UniComp

O **UniComp** é distribuído como um binário nativo único (`unicomp`). Ele pode atuar em dois modos simétricos:

1. **Modo Host / Daemon (`unicomp daemon`)**:
   - Instalado nas máquinas do seu cluster, servidores, desktops secundários ou instâncias na nuvem.
   - Gerencia o **Content-Addressable Storage (CAS)** com deduplicação de código por hash SHA-256 e compressão zstd/gzip.
   - Mantém as toolchains nativas do ambiente (Rust/Cargo, Python/uv, Node.js/pnpm, Java/JVM).
   - Executa a compilação ou inicialização de dependências diretamente na arquitetura nativa do host (evitando quebras de libc, ABI e C-extensions).
   - Transmite `stdout`, `stderr`, telemetria de CPU/RAM e códigos de saída em streaming assíncrono contínuo.

2. **Modo Master / Client (`unicomp run`)**:
   - Instalado no seu computador de desenvolvimento (Windows, macOS ou Linux).
   - Substitui a execução local de `python`, `node`, `cargo run` ou `java`.
   - Lê a workspace local (respeitando `.gitignore` e `.unicompignore`), calcula a árvore de Merkle dos arquivos.
   - Sincroniza **apenas os arquivos modificados (delta zero após o primeiro upload)**.
   - Recebe a saída do cluster e a exibe no seu terminal em tempo real como se estivesse rodando na sua própria máquina (`localhost`).

---

## 🛠️ Linguagens Suportadas Nativa e Automaticamente

| Linguagem / Ecossistema | Detecção Automática | Estratégia de Build / Execução no Host |
| :--- | :--- | :--- |
| **Rust / C / C++** | `Cargo.toml` | Compilação remota via `cargo run --release` com reaproveitamento de target cache e `sccache`. |
| **Python** | `pyproject.toml`, `requirements.txt`, `main.py` | Execução isolada com Python/uv; hidratação rápida de dependências. |
| **Node.js / TypeScript** | `package.json`, `index.js` | Execução via `node` / `npm start` / `pnpm` com store global compartilhado. |
| **JVM (Java / Kotlin)** | `pom.xml`, `build.gradle`, `*.java` | Compilação e execução via JVM 17/21/24 (`javac` + `java` ou Gradle/Maven). |
| **Comando Personalizado** | Qualquer arquivo / script | Flag `-- <comando>` para executar qualquer binário ou pipeline arbitrário. |

---

## 📦 Guia Rápido de Instalação e Uso

### 1. Compilar o Binário
```bash
cargo build --release
```
O executável compilado estará em `./target/release/unicomp` (ou `unicomp.exe` no Windows).

---

### 2. Iniciar o Nó Host (No Cluster / Servidor / Desktop Remoto)
Na máquina que possui CPU/GPU e memória para processamento:
```bash
unicomp daemon --bind 0.0.0.0:9876
```

---

### 3. Verificar a Saúde do Cluster (Na sua Máquina Master)
Antes de rodar qualquer código, você pode inspecionar o nó remoto:
```bash
unicomp check --host 192.168.1.50:9876
```
Saída esperada:
```text
============================================================
  UniComp Host Status: ONLINE
============================================================
  Hostname:         cluster-node-01
  Operating System: linux (x86_64)
  CPU Cores:        32
  Memory:           65536 MB
  Daemon Version:   0.1.0
------------------------------------------------------------
  Installed Toolchains (Ready for Remote Execution):
    * rust     : rustc 1.99.1 [rustc]
    * python   : Python 3.12.10 [python]
    * node     : v20.19.5 [node]
    * jvm      : openjdk 21.0.2 [java]
============================================================
```

---

### 4. Executar Seus Projetos Remotamente

#### Exemplo em Python (Monte Carlo Pi)
```bash
cd examples/python_job
unicomp run --host 192.168.1.50:9876
```
*Na primeira execução: envia o arquivo do script.*  
*Na segunda execução:*
```text
[UniComp] Connecting to cluster host at 192.168.1.50:9876...
[UniComp] Connected to host 'cluster-node-01' (linux x86_64, 32 cores, 65536 MB RAM)
[UniComp] Scanning workspace and building Merkle tree... found 1 files (0 dirs).
[UniComp] All workspace blobs already cached in cluster! (Delta = 0 bytes)
[UniComp] Starting execution on cluster [Language: Python] -> ["python", "main.py"]
[UniComp Phase: SynchronizingWorkspace] Materializing workspace for job job_1793069783021...
[UniComp Phase: Running] Executing command: ["python", "main.py"]
============================================================
  [PYTHON CLUSTER JOB] Running on Remote Host: cluster-node-01
  ...
```

#### Exemplo em Node.js
```bash
cd examples/node_job
unicomp run --host 192.168.1.50:9876 -- node index.js
```

#### Exemplo em Java / JVM
```bash
cd examples/java_job
unicomp run --host 192.168.1.50:9876
```

#### Exemplo em Rust (Heavy Compute)
```bash
cd examples/rust_job
unicomp run --host 192.168.1.50:9876
```

---

## 🏛️ Estrutura do Repositório

```text
D:\UnifiedCompiler\
├── Cargo.toml                    # Configuração de dependências ultra-lean
├── proto/
│   └── unicomp.proto             # Especificação universal Protocol Buffers v3 (REAPI compliant)
├── UNICOMP_ARCHITECTURE_SPEC.md  # Especificação técnica detalhada e decisões de engenharia
├── TECHNICAL_DEEP_DIVE.md        # Diagramas de sequência Mermaid e matrizes de trade-off
├── src/
│   ├── main.rs                   # Ponto de entrada CLI (daemon, run, check)
│   ├── protocol.rs               # Contratos de dados de streaming bidirecional
│   ├── framing.rs                # Codificação e decodificação assíncrona de quadros
│   ├── cas.rs                    # Content-Addressable Storage & Merkle Workspace Scanner
│   ├── language.rs               # Módulo de detecção e heurística de projetos
│   ├── server.rs                 # Daemon assíncrono do Worker com isolamento e streaming
│   └── client.rs                 # Cliente master com cálculo de deltas e streaming de I/O
└── examples/
    ├── python_job/               # Job de cálculo intensivo em Python
    ├── node_job/                 # Job assíncrono concorrente em Node.js
    ├── java_job/                 # Job de processamento numérico em JVM (Java)
    └── rust_job/                 # Job de computação de alta performance em Rust
```

---

## 🔬 Especificações Técnicas de Baixo Nível

Consulte os documentos dedicados para aprofundamento técnico:
- [UNICOMP_ARCHITECTURE_SPEC.md](UNICOMP_ARCHITECTURE_SPEC.md): Análise sobre mismatch de libc, virtualenvs, JNI nativo e sandboxing em Linux com `bwrap` e `cgroups v2`.
- [TECHNICAL_DEEP_DIVE.md](TECHNICAL_DEEP_DIVE.md): Sequência de chamadas entre Master, CAS, Sandbox e processo PTY.
- [unicomp.proto](proto/unicomp.proto): Esquema universal de gRPC/Protobuf para interoperabilidade com outros nós ou linguagens.
