# UniComp: Roadmap de Desenvolvimento

Este documento mapeia os estágios de maturação da engine, divididos por objetivos técnicos e experiência do usuário.

---

## 📍 Fase 1: Fundação do Core & Protocolo (Concluída ✅)
O objetivo desta fase foi validar a física do sistema e demonstrar que a compilação e execução remota funcionam com latência sub-segundo:

- [x] Binário único simétrico (`unicomp daemon` e `unicomp run`).
- [x] Engine de Content-Addressable Storage (CAS) com hashes SHA-256 e Merkle Tree.
- [x] Compressão de blobs via Gzip/Zstd no envio de arquivos.
- [x] Protocolo de transporte assíncrono em Rust sobre Tokio (`LengthDelimited` / Bincode framing).
- [x] Especificação universal em Protocol Buffers v3 ([`proto/unicomp.proto`](proto/unicomp.proto)).
- [x] Heurística de detecção automática para Python, Node.js, JVM e Rust.
- [x] Streaming em tempo real de `stdout` e `stderr` com flush contínuo.
- [x] Validação prática com benchmarks em 4 ecossistemas ([`examples/`](examples/)).

---

## 📍 Fase 2: Pareamento e Descoberta Sem Atrito (Próximo Marco 🎯)
O objetivo desta fase é eliminar a necessidade de passar parâmetros manuais como `--host 192.168.X.X`:

- [ ] **Arquivo de Configuração do Projeto (`.unicomp.toml`):**
  - Configurar host padrão, timeouts, pastas a sincronizar e variáveis de ambiente no próprio repositório.
- [ ] **Descoberta Local Automática (mDNS / Zeroconf / UDP Beacon):**
  - O daemon anuncia sua disponibilidade na rede local.
  - O cliente detecta os nós disponíveis e seleciona o mais adequado sem digitação manual de IP.
- [ ] **Mecanismo de Pareamento Seguro (Pairing Token & mTLS):**
  - Comando `unicomp pair <token>` para autenticação mútua e criptografia ponta a ponta.
- [ ] **Sincronização Reversa de Artefatos:**
  - Baixar automaticamente arquivos gerados durante a execução no host (ex: pasta `dist/`, arquivos `.png`, modelos `.pt`, relatórios) de volta para o cliente.

---

## 📍 Fase 3: Auto-Hydration e Sandboxing Robusto
O objetivo desta fase é garantir que o nó host nunca seja poluído ou comprometido por execuções de terceiros:

- [ ] **Integração de Package Managers de Alta Velocidade no Host:**
  - Python: Uso de `uv` nativo para resolver lockfiles em < 100ms sem poluir o sistema operacional do host.
  - Node.js: Uso de `pnpm` com store central compartilhada por hardlinks.
  - Rust: Configuração automática de `sccache` compartilhado para builds incrementais instantâneos.
- [ ] **Isolamento de Processos (Sandbox no Host):**
  - Linux: Suporte a `bubblewrap` (`bwrap`) e `cgroups v2` (limites de memória e cotas de CPU).
  - Windows: Uso de `Windows Job Objects` para impor limites estritos de RAM/CPU.
- [ ] **Suporte Completo a PTY (Pseudo-Terminal):**
  - Alocação de PTY master/slave no host para repasse de sequências ANSI completas, modo raw do teclado e redimensionamento dinâmico (`SIGWINCH`).

---

## 📍 Fase 4: Integração com IDEs & Shims Transparentes
O objetivo desta fase é integrar o UniComp no fluxo diário sem que o desenvolvedor precise sair de seu editor:

- [ ] **Extensão para VS Code / Cursor:**
  - Botão de execução direta com seleção de nó do cluster.
  - Painel de status de nós online, métricas de CPU/RAM em tempo real.
- [ ] **Shims Transparentes de Linha de Comando:**
  - Wrappers configuráveis para que comandos como `unicomp-python` ou `unicomp-cargo` sejam configurados como interpretadores nativos em IDEs como PyCharm, IntelliJ e VS Code.
- [ ] **Repasse de Sinais de Depuração (Remote Debugging Adapter):**
  - Túnel transparente para anexar depuradores locais (ex: `debugpy`, `lldb`) ao processo em execução remota.

---

## 📍 Fase 5: Pool de Execução e Balanceamento Inteligente
O objetivo desta fase é transformar múltiplos computadores em uma malha elástica de computação:

- [ ] **Agendador de Cargas de Trabalho (Scheduler):**
  - Seletor inteligente por perfil: tarefas de IA/PyTorch vão para máquinas com GPU; compilações pesadas de C++/Rust vão para máquinas com mais núcleos de CPU.
- [ ] **Fila de Execução e Prioridades:**
  - Suporte a fila de execuções concorrentes quando múltiplos desenvolvedores ou tarefas disputarem o mesmo host.
