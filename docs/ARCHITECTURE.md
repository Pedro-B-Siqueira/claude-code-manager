# Arquitetura

Este documento descreve como o Claude Code Manager é organizado: estrutura de pastas, módulos, contratos entre backend e frontend, modelo de dados e a ordem em que as partes são construídas.

## Princípios

- **Somente leitura no Claude Code.** `~/.claude/` e `~/.claude.json` nunca são escritos. Toda escrita do app passa por `AppPaths::ensure_writable`, que recusa esses caminhos.
- **Configuração global intocada.** Hooks entram só nas sessões abertas pelo app, via `claude --settings <arquivo>`.
- **Zero tokens.** O app não chama modelo nenhum, não envia prompts às sessões e não faz chamadas à API da Anthropic.
- **Dados próprios isolados.** Tudo que o app guarda fica em `~/Library/Application Support/ClaudeCodeManager/`. A janela usa armazenamento não persistente do WebView, para que nada seja gravado em `~/Library/WebKit`.
- **Git só com ação explícita.** Nenhum `checkout`, `reset` ou `stash` automático. Criar ou remover worktree exige confirmação.
- **O app se adapta ao usuário, não o contrário.** Projetos, pastas de worktree, branch principal e convenções são inferidos do que já existe na máquina (transcripts, `git worktree list`, configuração do repositório). Tudo pode ser ajustado nas configurações, que ficam só na máquina do usuário.
- **Testes nunca rodam o `claude` real.** O binário falso `fake-claude` (selecionado por `CCM_CLAUDE_BIN`) simula a saída do terminal, as linhas de transcript e as chamadas de hook.

## Decisões de projeto

| Tema | Decisão |
|---|---|
| Worktrees | Raiz configurável. Sem configuração, o app infere a raiz a partir dos worktrees que já existem nos repositórios conhecidos; sem nenhum, usa `~/worktrees`. Estrutura plana `<raiz>/<repo>-<slug>`. Os prefixos de branch são configuráveis (padrão `feat-` e `fix-`), e a branch nova sai da branch principal local do repositório (detectada como `main` ou `master`), sem `fetch` automático. |
| Hibernação | Ligada por padrão, depois de 30 min de ociosidade. Sessões em Trabalhando ou Pedindo permissão nunca hibernam. O card mostra a memória (RSS) do processo `claude`. |
| Cota de uso | Fora do escopo. O app não lê token OAuth nem acessa o Keychain. O custo equivalente por sessão é calculado localmente a partir do `usage` dos transcripts. |
| Terminal na grade | Os cards mostram uma prévia em texto gerada no backend. O xterm.js só é montado na sessão em foco. |

## Formatos do Claude Code

Validado com o Claude Code 2.1.295. Nada disso é API oficial, então cada formato fica isolado em um módulo próprio, com testes, e o app degrada para "indisponível" em vez de quebrar.

**Transcripts** ficam em `~/.claude/projects/<cwd-codificado>/<sessionId>.jsonl`, um evento JSON por linha.

- **Tipos de linha:** `user`, `assistant`, `attachment`, `ai-title` (`aiTitle`), `last-prompt` (`lastPrompt`), `mode`, `permission-mode`, `file-history-snapshot`, entre outros.
- **Campos comuns:** `cwd`, `gitBranch`, `timestamp`, `sessionId`, `uuid`, `parentUuid`, `isSidechain` e `version`.
- **Mensagem do assistente:** `message.{id, model, content[], usage, stop_reason}`. O `usage` traz `input_tokens`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens` e `cache_creation.{ephemeral_5m_input_tokens, ephemeral_1h_input_tokens}`.
- **Edições:** `tool_use` Edit (`file_path`, `old_string`, `new_string`, `replace_all`) e Write (`file_path`, `content`). O `toolUseResult` da linha `user` seguinte traz `structuredPatch[{oldStart, oldLines, newStart, newLines, lines}]`, que dá a faixa de linhas. MultiEdit é aceito pelo parser, mas não aparece nas versões recentes.
- **Subagentes:** `<sessionId>/subagents/agent-*.jsonl`. As edições feitas por eles entram nos arquivos da sessão principal, com um selo.

**Sessões vivas.** O Claude Code mantém `~/.claude/sessions/<pid>.json` com `{pid, sessionId, cwd, status: busy|idle, statusUpdatedAt, name, kind, version}`. Essa é a fonte principal para sessões abertas fora do app. O fallback é `ps` + `lsof`, e o PID sempre é validado com `kill(pid, 0)`.

**CLI.**

- `--session-id <uuid>`: o app gera o id antes de abrir a sessão, então terminal e transcript ficam ligados desde o início.
- `--resume <id>`: retoma uma sessão.
- `--settings <arquivo>`: injeta os hooks do app.
- `-n/--name` não é usado, porque o nome dado no app existe só no app.

**Hooks.**

- **Tipo `http`:** `{type: "http", url, headers, allowedEnvVars, timeout}`. O Claude Code faz POST do JSON do evento.
- **Fontes somadas:** hooks de fontes diferentes se somam; os do usuário continuam rodando.
- **Eventos usados:** `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PermissionRequest`, `Notification` (`permission_prompt`, `idle_prompt`), `Stop` e `SessionEnd`.
- **Campos comuns:** `session_id`, `transcript_path`, `cwd` e `hook_event_name`.

**ClaudeGauge.** Quando o `settings.json` do usuário contém um hook cujo comando inclui `claude-notify.sh`, o app desliga as próprias notificações de sistema por padrão, para não duplicar avisos.

## Estrutura de pastas

```
claude-code-manager/
├─ package.json            # dev / generate / test
├─ vite.config.ts  svelte.config.js  tsconfig.json  vitest.config.ts
├─ docs/ARCHITECTURE.md
├─ src/                    # frontend Svelte 5 + TypeScript
│  ├─ main.ts  App.svelte
│  └─ lib/
│     ├─ api/        commands.ts  types.ts  logger.ts   # wrappers tipados de invoke/listen/Channel
│     ├─ stores/     live  library  settings  ui (.svelte.ts)
│     ├─ sessions/   regras de status e filtros
│     ├─ components/
│     │  ├─ layout/    TopBar  Sidebar  FilterChips  ViewToggle  ResizablePanes
│     │  ├─ sessions/  SessionGrid  SessionCard  StatusBadge  ContextMeter  FileChangeList
│     │  │             DiffPopover  ActivityFeed  SummaryPanel  TerminalPreview
│     │  ├─ focus/     FocusView
│     │  ├─ terminal/  XtermView
│     │  ├─ library/   ResumeModal  HistoryRow  TagEditor
│     │  ├─ palette/   CommandPalette
│     │  └─ dialogs/   NewSessionDialog  ConfirmDialog  SettingsDialog
│     ├─ theme/      tokens, movimento, fontes
│     └─ assets/fonts/  Geist e Geist Mono (OFL)
└─ src-tauri/
   ├─ Cargo.toml           # workspace: app + crates/fake-claude
   ├─ tauri.conf.json  capabilities/  icons/
   ├─ src/                 # módulos abaixo
   ├─ crates/fake-claude/  # binário falso para testes
   └─ tests/
      ├─ fixtures/transcripts/   # sintéticas, no formato real
      ├─ golden_rules.rs         # HOME temporário com ~/.claude somente leitura
      └─ session_flow.rs         # fluxo de sessão com o fake-claude
```

## Módulos Rust

| Módulo | Responsabilidade |
|---|---|
| `lib.rs` | Builder do Tauri, plugins, `AppState` e registro dos comandos |
| `error.rs` | `AppError`, serializado para o frontend como `{kind, message}` |
| `paths.rs` | Resolve `claude_home` (respeita `HOME` e `CLAUDE_CONFIG_DIR`) e a pasta de dados do app, e recusa escritas em áreas do Claude Code |
| `settings.rs` | Configurações tipadas, com padrões e saneamento de valores |
| `db/` | SQLite (`rusqlite`, SQLite embutido com FTS5) e migrations por `user_version` |
| `shell_env.rs` | Lê o PATH do shell de login uma vez e localiza `claude`, `code`, `gh` e `git` |
| `transcript/` | Tipos tolerantes a campos novos, parser linha a linha, leitura incremental por offset, extração de edições (faixa + "Por quê"), resumo local e soma de `usage` deduplicada por `message.id` |
| `pricing.rs` | Preço por modelo para o custo equivalente (cache 5m = 1,25×, 1h = 2×, leitura = 0,1× da entrada) |
| `context.rs` | Tamanho da janela de contexto por modelo e porcentagem usada |
| `library/` | Varredura inicial em segundo plano e indexação incremental para o banco e o FTS |
| `watcher.rs` | `notify` (FSEvents) em `projects/` e `sessions/`, com debounce e sem polling |
| `live/` | Registro de sessões vivas, fallback por processos e RSS por PID |
| `pty/` | `portable-pty`, spawn pelo shell de login (`claude …; exec $SHELL -l -i`), ring buffer de scrollback, prévia via `vt100` e hibernação |
| `hooks/` | Servidor HTTP local em `127.0.0.1` com porta aleatória e o arquivo de settings gerado a cada execução. O token de cada sessão vai por variável de ambiente, nunca para o disco |
| `status.rs` | Máquina de estados das sessões |
| `git.rs` | Branch, diffstat e worktrees (listar, criar, remover se estiver limpo) |
| `integrations.rs` | VS Code, Finder e abertura de PR (`gh` ou URL de compare) |
| `claudegauge.rs` | Detecção do hook do ClaudeGauge (só leitura) |
| `notifications.rs` · `tray.rs` | Notificações de sistema e ícone na barra de menus |
| `commands/` | Handlers finos por domínio |

### Status das sessões

**Sessões abertas pelo app (via hooks).**

| Evento | Status |
|---|---|
| `UserPromptSubmit`, `PreToolUse`, `PostToolUse` | Trabalhando |
| `PermissionRequest`, `Notification:permission_prompt` | Pedindo permissão (com o comando pedido) |
| `Notification:idle_prompt` | Esperando você |
| `Stop` | Concluída (vira Ociosa depois de alguns minutos sem evento) |
| `SessionEnd` ou saída do processo | Encerrada |

**Sessões externas (via registro e transcript).**

| Sinal | Status |
|---|---|
| Registro em `busy` | Trabalhando |
| Registro em `idle`, com o último turno do assistente terminado | Concluída |
| `tool_use` sem `tool_result` por alguns segundos | Pedindo permissão (heurística) |
| Processo encerrado | Sai da lista de sessões vivas |

## Contratos backend ↔ frontend

**Comandos (`invoke`).**

- **Biblioteca:** `library_list`, `library_search`, `session_summary`, `session_rename`, `session_pin`, `session_set_tags`, `session_set_category`, `tags_list`.
- **Sessões vivas:** `live_list`, `session_new({cwd, worktree?})`, `session_resume(id)`, `session_close(key, force)`, `session_wake(key)`.
- **Terminal:** `pty_attach(key, channel)`, `pty_detach`, `pty_write`, `pty_resize`.
- **Detalhes:** `session_files`, `session_activity`, `edit_detail(editId)`. O diff é carregado sob demanda, relido do transcript pelo offset da linha.
- **Git:** `git_status`, `worktree_list`, `worktree_create`, `worktree_remove`, `open_vscode`, `open_finder`, `open_pr`.
- **App:** `settings_get`, `settings_update`, `recent_dirs`, `grid_order_set`, `app_info`.

**Eventos (`emit`).**

| Evento | Quando dispara |
|---|---|
| `live:changed` | Lista de sessões vivas ou RSS mudou |
| `session:status` | Status de uma sessão mudou |
| `session:delta` | Tokens, custo, contexto ou arquivos mudaram |
| `session:activity` | Nova atividade na sessão |
| `session:exited` | O processo da sessão saiu |
| `library:changed` | A biblioteca de sessões mudou |
| `needs-you:count` | Mudou o número de sessões que precisam de você |

A saída do terminal vai por `tauri::ipc::Channel` binário, e só enquanto o xterm está montado.

## Modelo de dados

SQLite em `~/Library/Application Support/ClaudeCodeManager/ccm.sqlite`. O cache derivado dos transcripts pode ser reconstruído a qualquer momento; os dados do usuário (nomes, tags, fixados, ordem e configurações) não.

| Tabela | Conteúdo |
|---|---|
| `settings` | Configurações em JSON |
| `transcript_files` | Estado da leitura incremental: caminho, inode, tamanho, offset e mtime |
| `sessions` | Cache derivado: cwd, branch, título, pedido inicial, última resposta, modelo, tokens, custo e contexto |
| `session_meta` | Nome personalizado, fixada, ordem de fixação e categoria |
| `tags` · `session_tags` | Tags e associações |
| `file_edits` | Edições com faixa de linhas, +/− e posição da linha no transcript (o diff completo não fica no banco) |
| `activity` | Feed de atividade por sessão |
| `session_fts` | Índice FTS5 com títulos, prompts, texto do assistente, arquivos e branch (sem `tool_result`, para manter o índice pequeno) |
| `app_sessions` | Sessões abertas pelo app, inclusive as hibernadas |
| `grid_order` | Ordem dos cards na grade |

## Etapas

1. **Esqueleto:** Tauri + Svelte, temas claro e escuro, layout base com dados falsos, scripts e README.
2. **Transcripts e biblioteca:** parser incremental, índice de busca, renomear, fixar, tags e resumo local.
3. **Terminais:** PTY com xterm.js, nova sessão, retomar, encerrar, confirmação ao fechar e PATH do shell de login.
4. **Status em tempo real:** servidor de hooks, `--settings`, sessões externas, notificações e detecção do ClaudeGauge.
5. **Cards e Foco:** prévia, arquivos, hover de diff, atividade, contexto, tokens, custo e memória.
6. **Git:** branch e diffstat, VS Code, Finder, PR e worktrees.
7. **Barra de menus:** ícone com contagem e lista rápida.
8. **Navegação:** `⌘K`, atalhos, filtros, drag-and-drop animado e painéis redimensionáveis.
9. **Economia:** pausa de renderização, scrollback, hibernação e medição de memória.
10. **Revisão final:** auditoria das garantias e da cobertura de testes.

## Verificação

- **`npm test`:** roda `cargo test --workspace`, `svelte-check` e `vitest`.
- **`golden_rules.rs`:** cria um HOME temporário com `~/.claude` populado e em modo somente leitura, exercita o app e confirma que nenhum arquivo ali mudou.
- **Testes de fluxo:** usam o `fake-claude` para cobrir nova sessão, retomada, status via hooks e encerramento.
