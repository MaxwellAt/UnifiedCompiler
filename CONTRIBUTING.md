# Guia de Contribuição: UniComp

Obrigado pelo interesse em contribuir com o **UniComp**! Este projeto é construído em código aberto com a missão de transformar qualquer computador em uma extensão transparente da sua máquina de desenvolvimento.

---

## 🏛️ Filosofia de Design da Base de Código

Antes de escrever código, considere estes quatro pilares:

1. **Simplicidade Operacional para o Usuário Final:** O usuário nunca deve precisar rodar comandos complexos no servidor remoto. O daemon deve ser o mais autônomo possível.
2. **Desempenho e Eficiência de Recursos:** O UniComp deve iniciar em milissegundos e usar o mínimo de memória possível. Evitamos frameworks pesados ou runtimes lentos.
3. **Segurança no Host:** O host deve se proteger contra execuções destrutivas, loops infinitos e vazamento de recursos.
4. **Resiliência a Falhas de Rede:** Conexões remotas podem oscilar. Streams devem tratar `EOF`, desconexões e reconexões graciosamente.

---

## 🛠️ Como Configurar o Ambiente Local

### Pré-requisitos:
* **Rust 1.80+** (com `cargo` e `rustc`).
* Pelo menos um runtime instalado para testes locais:
  * Python 3.10+
  * Node.js 18+
  * Java / JDK 17+

### 1. Clonar e Compilar
```bash
git clone https://github.com/MaxwellAt/UnifiedCompiler.git
cd UnifiedCompiler

# Compilar o binário em modo debug
cargo build

# O binário estará pronto em:
./target/debug/unicomp
```

### 2. Testar o Ciclo Completo (Host + Cliente na Mesma Máquina)

Abra dois terminais:

**Terminal 1 (Host Daemon):**
```bash
./target/debug/unicomp daemon --bind 127.0.0.1:9876
```

**Terminal 2 (Cliente / Master):**
```bash
# 1. Inspecionar o daemon
./target/debug/unicomp check --host 127.0.0.1:9876

# 2. Executar um dos projetos de exemplo
./target/debug/unicomp run --host 127.0.0.1:9876 --dir examples/python_job
./target/debug/unicomp run --host 127.0.0.1:9876 --dir examples/node_job -- node index.js
./target/debug/unicomp run --host 127.0.0.1:9876 --dir examples/rust_job
./target/debug/unicomp run --host 127.0.0.1:9876 --dir examples/java_job
```

---

## 📂 Arquitetura dos Módulos

```text
src/
├── main.rs       # Entrada CLI e roteamento de comandos (daemon, run, check)
├── protocol.rs   # Tipos e structs compartilhadas (Digest, Manifest, Mensagens)
├── framing.rs    # Serialização assíncrona com bincode e prefixo de tamanho
├── cas.rs        # Content-Addressable Storage (hashing SHA-256 e Merkle Tree)
├── language.rs   # Detecção automática de tipos de projeto e comandos padrão
├── server.rs     # Daemon do host (recebe conexões, materializa e executa)
└── client.rs     # Cliente local (calcula deltas, faz upload e lê streaming de saída)
```

---

## 🤝 Áreas Prioritárias para Contribuição

Confira o nosso [`ROADMAP.md`](ROADMAP.md) para a lista completa. Algumas tarefas de alto impacto:

1. **Presets de Novas Linguagens e Runtimes:**
   * Adicionar suporte a Go (`go run`), C++ (`CMakeLists.txt`), Kotlin, .NET/C#, etc. em [`src/language.rs`](src/language.rs).
2. **Suporte a Arquivo `.unicomp.toml`:**
   * Permitir que o cliente leia um arquivo de configuração no diretório do projeto para definir o host padrão.
3. **Descoberta via mDNS / Zeroconf:**
   * Implementar o broadcast de presença para eliminar a necessidade de passar `--host <IP>`.
4. **Sandboxing e Limites de Recursos no Host:**
   * Implementar isolamento no Linux com Bubblewrap (`bwrap`) e cgroups v2.
   * Implementar limites de memória no Windows usando `Windows Job Objects`.
5. **Sincronização Reversa de Arquivos Gerados:**
   * Sincronizar arquivos criados durante o job de volta para o cliente.

---

## 📝 Padrão de Commits e Licenciamento

Adotamos o padrão **Conventional Commits**:
* `feat: ...` para novas funcionalidades.
* `fix: ...` para correção de bugs.
* `docs: ...` para melhorias de documentação.
* `refactor: ...` para refatorações que não alteram comportamento.
* `perf: ...` para melhorias de desempenho.

Todas as contribuições submetidas ao projeto são licenciadas sob a **GNU Affero General Public License v3 (AGPL-3.0)**. Ao enviar um Pull Request, você confirma que seu código pode ser compartilhado sob estes termos.
