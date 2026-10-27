# UniComp: Visão e Manifesto do Projeto

> **"A ergonomia de programar no seu notebook local, com a força bruta da sua máquina dedicada — sem SSH, sem Git push/pull, sem configuração manual."**

---

## 1. A Dor Real: A Falsa Promessa do "Cluster Pessoal"

Muitos desenvolvedores possuem hoje um cenário muito comum:
* Um **notebook** leve e portátil para trabalhar no dia a dia.
* Um **desktop dedicado**, uma **workstation com GPU**, ou um **servidor antigo/homelab** na mesma rede com dezenas de gigabytes de RAM e muitos núcleos de CPU.

No entanto, transformar essa segunda máquina em uma extensão útil do seu fluxo de desenvolvimento é uma experiência cheia de atrito.

### O Fluxo Tradicional e seus Gargalos:
1. **O Ciclo do Git:** Para testar uma alteração simples de 2 linhas, você precisa commitar (`git commit`), subir (`git push`), conectar via SSH na outra máquina, e puxar (`git pull`).
2. **O Pesadelo da Sincronização de Ambiente:**
   * O Python da máquina remota não tem o mesmo pacote ou tem versão de biblioteca diferente.
   * O Node.js quebra por causa de módulos compilados na plataforma errada.
   * O Rust precisa compilar dependências inteiras do zero toda vez.
   * O Java/JVM tem mismatch de versão (`Java 17` vs `Java 21`).
3. **A Perda de Foco (Context Switching):**
   * Você precisa abrir o Remote Desktop (VNC/RDP) ou múltiplos terminais SSH.
   * Os logs ficam na outra máquina; se o script gerar um gráfico ou arquivo, você precisa copiar via SCP/SFTP de volta.

**Resultado prático:** O desenvolvedor desiste de usar a máquina potente para tarefas cotidianas e continua sobrecarregando o próprio notebook.

---

## 2. A Inspiração: O Modelo "Databricks Connect"

Plataformas modernas de Big Data como o **Databricks** resolveram isso de uma forma elegante através do *Databricks Connect*:
* O código permanece e é editado **100% no seu computador**.
* O driver/cliente intercepta a intenção de execução e transmite o plano para o cluster remoto via gRPC.
* O cluster executa a carga pesada e faz streaming dos resultados de volta para o seu terminal local.

**O UniComp nasce para levar essa exata filosofia para qualquer desenvolvedor comum, unificando as principais linguagens do mercado (Rust, Python, Node.js, JVM e C/C++).**

---

## 3. Os Princípios Fundamentais do UniComp

### Princípio 1: Zero-Configuração Remota (O Host se Auto-Configura)
O desenvolvedor nunca deve precisar abrir uma sessão SSH para instalar pacotes ou preparar pastas na máquina host. O daemon remoto analisa os manifestos recebidos (`requirements.txt`, `package.json`, `Cargo.lock`, etc.) e gerencia os ambientes de forma autônoma e descartável.

### Princípio 2: Local Ergonomics, Remote Compute
O comando executado no cliente deve se comportar de forma indistinguível de um comando local:
* Mesmas saídas no terminal (`stdout` e `stderr` separados).
* Cores ANSI e barras de progresso preservadas.
* Cancelamento imediato via `Ctrl+C` repassado diretamente ao processo no host.
* Mesmo código de saída (`exit code`).

### Princípio 3: Content-Addressable Storage (CAS) e Delta Zero
Não transferimos repositórios nem arquivos repetidos. Usando uma árvore de Merkle (SHA-256):
* Na primeira execução, apenas o código fonte puro é compactado e enviado.
* A partir da segunda execução, **o tráfego de rede tende a zero bytes** para arquivos inalterados.
* Pastas pesadas (`node_modules`, `.venv`, `target`, `.git`) são bloqueadas na origem e reconstruídas/cacheadas no destino.

### Princípio 4: Simetria Binária
Um único executável (`unicomp`) resolve os dois lados da moeda:
* `unicomp daemon`: Executa no nó de processamento.
* `unicomp run`: Executa no notebook de desenvolvimento.

---

## 4. Onde Queremos Chegar (A Experiência Alvo)

Imagine abrir o terminal no seu notebook em um projeto Python com Machine Learning ou um projeto pesado em Rust:

```bash
# O UniComp detecta sua máquina dedicada na LAN via broadcast
$ unicomp run

[UniComp] Nó detectado na rede: "Desktop-Ryzen-RTX" (12 cores, 32GB RAM, CUDA pronto)
[UniComp] Sincronizando deltas (0 arquivos modificados)...
[UniComp] Executando remotamente em Desktop-Ryzen-RTX...
============================================================
  Treinando modelo com GPU remota...
  Época 1/50 [====================] 100% - perda: 0.124
============================================================
[UniComp] Finalizado em 4.2s com código 0.
```

Você não digitou IP. Você não abriu SSH. Você não configurou ambiente remoto.  
**O código rodou lá, mas a sensação foi de rodar aqui.**
