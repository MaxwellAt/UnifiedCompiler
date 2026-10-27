# UniComp (Compilador Único / Universal Remote Execution Engine)

<p align="center">
  <b>A força de processamento do seu cluster ou máquina dedicada com a ergonomia do seu notebook local.</b><br>
  <i>Sem SSH. Sem Git push/pull manual. Sem dualidade de ambientes.</i>
</p>

<p align="center">
  <a href="VISION.md"><b>Manifesto & Visão</b></a> •
  <a href="ROADMAP.md"><b>Roadmap</b></a> •
  <a href="CONTRIBUTING.md"><b>Como Contribuir</b></a> •
  <a href="UNICOMP_ARCHITECTURE_SPEC.md"><b>Especificação Técnica</b></a>
</p>

---

## 💡 A Grande Ideia (Por que o UniComp existe?)

Frequentemente, desenvolvedores possuem uma máquina potente disponível na rede (um desktop gamer com GPU, uma workstation com muitos núcleos ou um servidor homelab), mas utilizam um notebook no dia a dia.

Hoje, usar essa máquina potente para rodar seu código exige um ritual cheio de atrito:
1. Dar `git commit` e `git push` no notebook.
2. Acessar a outra máquina via SSH ou Remote Desktop.
3. Dar `git pull`.
4. Resolver dependências e versões que quebram no ambiente remoto (`pip`, `npm`, `rustup`, `java`).
5. Executar o comando e tentar inspecionar os logs longe do seu editor.

O **UniComp** resolve essa lacuna. Inspirado na arquitetura do **Databricks Connect** e do **Google Bazel Remote Execution API (REAPI)**, ele transforma qualquer computador em um nó de execução transparente:

* O seu código permanece **100% no seu notebook**.
* O comando de execução é interceptado e redirecionado para a máquina dedicada.
* Apenas os arquivos modificados (**delta zero**) são enviados via rede usando Content-Addressable Storage (CAS).
* A compilação e a execução pesada acontecem no host remoto.
* O `stdout`, `stderr`, códigos de saída e métricas voltam para o seu terminal em tempo real como se o processo estivesse rodando na sua própria máquina (`localhost`).

Para ler a motivação e os princípios completos, veja o nosso [**Manifesto de Visão (`VISION.md`)**](VISION.md).

---

## 🏛️ Como Funciona o Sistema

```
                  +-----------------------------------+
                  |        UniComp Client (CLI)       |
                  |     (Seu Notebook / Workstation)  |
                  +-----------------+-----------------+
                                    |
          1. Workspace Merkle Walk  | 2. Manifest & Env Probe
          3. Upload de Deltas (CAS) | 4. Streaming Bidirecional em Tempo Real
                                    v
                  +-----------------------------------+
                  |      UniComp Daemon (Worker)      |
                  |     (Máquina Dedicada / Host)     |
                  +-----------------+-----------------+
                                    |
     +------------------------------+------------------------------+
     |                              |                              |
     v                              v                              v
+------------+              +---------------+              +---------------+
| Content    |              |  Auto-Env     |              |  Execution    |
| Addressed  |              |  Hydration    |              |  Supervisor   |
| Store(CAS) |              | (uv/pnpm/etc) |              | (Stdout/Err)  |
+------------+              +---------------+              +---------------+
```

---

## ⚡ Linguagens e Runtimes Suportados Nativa e Automaticamente

| Linguagem / Ecossistema | Detecção Automática | Estratégia de Build / Execução no Host |
| :--- | :--- | :--- |
| **Rust / C / C++** | `Cargo.toml` | Compilação remota nativa via `cargo run --release` com reaproveitamento de cache. |
| **Python** | `pyproject.toml`, `requirements.txt`, `main.py` | Execução com Python/uv isolado e hidratação rápida de dependências. |
| **Node.js / TypeScript** | `package.json`, `index.js` | Execução via `node` / `npm start` / `pnpm` com store compartilhada. |
| **JVM (Java / Kotlin)** | `pom.xml`, `build.gradle`, `*.java` | Compilação e execução via JVM (`javac` + `java` ou Gradle). |
| **Comando Personalizado** | Qualquer script ou binário | Flag `-- <comando>` para executar qualquer pipeline arbitrário. |

---

## 🚀 Guia Rápido de Instalação e Teste

### 1. Compilar o Binário Único (Rust)
```bash
cargo build --release
```
O executável compilado estará em `./target/release/unicomp` (ou `unicomp.exe` no Windows).

### 2. Iniciar o Host (Na Máquina Potente / Cluster)
```bash
unicomp daemon --bind 0.0.0.0:9876
```

### 3. Verificar o Host (No seu Notebook)
```bash
unicomp check --host 192.168.1.50:9876
```

### 4. Executar Seus Projetos sem Subir nada Manualmente
```bash
# Executar projeto Python (exemplo de Monte Carlo)
unicomp run --host 192.168.1.50:9876 --dir examples/python_job

# Executar projeto Rust pesado
unicomp run --host 192.168.1.50:9876 --dir examples/rust_job

# Executar projeto Node.js
unicomp run --host 192.168.1.50:9876 --dir examples/node_job -- node index.js

# Executar projeto Java / JVM
unicomp run --host 192.168.1.50:9876 --dir examples/java_job
```

---

## 🗺️ Roadmap de Evolução

Nosso objetivo é transformar o UniComp em uma experiência 100% invisível (zero configuração manual de IPs, descoberta local mDNS automática, shims para VS Code e pooling de recursos).  
Consulte o [**ROADMAP.md**](ROADMAP.md) para detalhes de cada fase.

---

## 🤝 Como Contribuir

Contribuições em código, documentação e sugestões de arquitetura são muito bem-vindas!  
Consulte o [**Guia de Contribuição (`CONTRIBUTING.md`)**](CONTRIBUTING.md) para entender como configurar o ambiente e quais áreas estão abertas para desenvolvimento.

---

## 📄 Licença

Distribuído sob licença aberta (MIT / Apache 2.0). Veja o arquivo `LICENSE` para mais detalhes.
