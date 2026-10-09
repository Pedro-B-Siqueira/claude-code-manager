# Claude Code Manager

Gerenciador leve de sessões do Claude Code para macOS, com terminais embutidos. Não é uma IDE: serve para acompanhar várias sessões ao mesmo tempo, ver o que cada uma alterou e retomar sessões antigas.

> **Status:** etapa 2 de 10 (biblioteca de sessões a partir dos transcripts). A arquitetura completa está em [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Garantias

- `~/.claude/` e `~/.claude.json` são **somente leitura**. Toda escrita passa por `AppPaths::ensure_writable`, que recusa esses caminhos, e um teste de integração roda o app com `~/.claude` em modo somente leitura.
- **Zero tokens:** o app nunca chama modelo nenhum nem a API da Anthropic.
- Hooks só entram nas sessões abertas pelo app, via `claude --settings <arquivo>`. A configuração global não muda.
- Os dados do app ficam só em `~/Library/Application Support/ClaudeCodeManager/` (banco SQLite e logs).
- Testes automatizados nunca rodam o `claude` real. Eles usam o binário falso `fake-claude` (`CCM_CLAUDE_BIN`).

## Requisitos

- macOS 14 ou superior, Apple Silicon.
- Node 20.19+ (validado com 25.8.1) e npm.
- Rust estável via rustup (validado com 1.99). Se o rustup foi instalado com `--no-modify-path`, os scripts npm já incluem `~/.cargo/bin` no PATH.
- Xcode Command Line Tools.

## Instalação

```bash
npm install
```

## Uso

| Comando | O que faz |
|---|---|
| `npm run dev` | Abre o app em modo de desenvolvimento (Vite + Tauri). |
| `npm run generate` | Build de produção: gera `.app` e `.dmg` em `src-tauri/target/release/bundle/`. |
| `npm test` | Testes Rust (`cargo test --workspace`), checagem de tipos (`svelte-check`) e testes do frontend (`vitest`). |
| `npm run check` | Só a checagem de tipos do frontend. |
| `cargo run --release --example index_benchmark -- <projects> <db>` | Mede a indexação de um diretório de transcripts num banco descartável (em `src-tauri/`). |

O build é local e não é assinado nem notarizado. Na primeira abertura do `.app`, use botão direito → **Abrir**.

## Estrutura

```
src/                 frontend Svelte 5 + TypeScript
src-tauri/           backend Rust (Tauri 2)
src-tauri/crates/    fake-claude (binário falso para testes)
docs/                arquitetura e decisões
```

## Versões validadas

| Item | Versão |
|---|---|
| Claude Code | 2.1.295 |
| Tauri | 2.11 |
| Svelte | 5.57 |
| Rust | 1.99 |

O formato dos transcripts e o registro de sessões vivas não são API oficial e podem mudar entre versões do Claude Code.

## Créditos

- **ClaudeGauge**, de Pedro Henrique Gazola ([github.com/PedroHenriqueGazola/ClaudeGauge](https://github.com/PedroHenriqueGazola/ClaudeGauge), licença MIT): referência para o cálculo de custo equivalente (estrutura da tabela de preços e divisão da escrita de cache por TTL) e para a detecção de sessões vivas. A lógica foi portada para Rust, sem cópia do código Swift. Duas diferenças: o uso é contado uma vez por resposta (`message.id`), já que o transcript repete o `usage` em várias linhas, e a tabela de preços foi atualizada com os valores oficiais dos modelos atuais.
- **Geist** e **Geist Mono**, da Vercel (SIL Open Font License 1.1): fontes embutidas no app. A licença está em `src/lib/assets/fonts/OFL.txt`.
