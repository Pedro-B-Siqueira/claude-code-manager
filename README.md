# Claude Code Manager

Gerenciador leve de sessões do Claude Code para macOS, com terminais embutidos. Não é uma IDE: serve para acompanhar várias sessões ao mesmo tempo, ver o que cada uma alterou e retomar sessões antigas.

> **Status:** etapa 1 de 10 (esqueleto com dados falsos). A arquitetura completa está em [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

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

- **ClaudeGauge**, de Pedro Henrique Gazola ([github.com/PedroHenriqueGazola/ClaudeGauge](https://github.com/PedroHenriqueGazola/ClaudeGauge), licença MIT): referência para a tabela de preços de modelos (custo equivalente por sessão) e para a detecção de sessões vivas. A lógica foi portada para Rust, sem cópia do código Swift.
- **Geist** e **Geist Mono**, da Vercel (SIL Open Font License 1.1): fontes embutidas no app. A licença está em `src/lib/assets/fonts/OFL.txt`.
