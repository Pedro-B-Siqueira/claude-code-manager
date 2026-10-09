# Claude Code Manager

Um app de macOS e Linux para quem roda várias sessões do [Claude Code](https://code.claude.com) ao mesmo tempo. Ele mostra o que cada sessão está fazendo, o que ela alterou e o que está esperando de você, e deixa retomar conversas antigas sem procurar o id no terminal.

Não é uma IDE. Os terminais são reais (cada sessão é o `claude` de sempre, rodando no seu shell), e o app fica em volta deles organizando.

![Sessões ativas em grade, tema escuro](docs/images/grade-escuro.png)

## Download

Baixe a versão mais recente em **[Releases](https://github.com/Pedro-B-Siqueira/claude-code-manager/releases/latest)**.

**macOS 14 ou superior** (Apple Silicon e Intel): abra o `.dmg` e arraste o app para Aplicativos. O app não tem assinatura da Apple, então na primeira vez o macOS bloqueia. Libere em **Ajustes → Privacidade e Segurança → Abrir mesmo assim**, ou rode:

```bash
xattr -dr com.apple.quarantine "/Applications/Claude Code Manager.app"
```

**Linux x86_64 e arm64** (experimental, ainda sem teste numa máquina real): prefira o `.deb` (Ubuntu, Debian e derivados), que já instala as dependências: `sudo apt install ./Claude*.deb`. Ou use o AppImage: `chmod +x ./Claude*.AppImage` e execute; no Ubuntu 24.04 ou superior ele precisa do FUSE 2 (`sudo apt install libfuse2t64`). Para colar imagens no Claude Code, instale `xclip` (X11) ou `wl-clipboard` (Wayland). O ícone na bandeja depende do painel; no GNOME, só aparece com a extensão AppIndicator. Achou algo estranho? Abra uma issue.

## Por que existe

Com quatro ou cinco sessões abertas em abas de terminal, fica difícil saber qual está pedindo permissão, qual terminou e qual ainda está trabalhando. Também é fácil perder uma conversa boa de dias atrás. E cada processo do Claude Code ocupa algumas centenas de megabytes de memória, então deixar tudo aberto "por via das dúvidas" pesa num notebook de 16 GB.

O app resolve isso lendo o que o próprio Claude Code já grava em disco, sem mudar nada na sua configuração e sem gastar um token.

## O que ele faz

**Sessões em grade.** Cada card mostra status, repositório, branch, o que o Claude está fazendo no terminal, os arquivos alterados (+/−), o quanto da janela de contexto já foi usado, os tokens que o Claude escreveu e a memória. Os filtros separam as que precisam de você das que estão trabalhando ou já terminaram, e os cards podem ser reordenados arrastando.

**Por que mudou isso?** Ao passar o mouse (ou focar com o teclado) num arquivo, aparece o diff daquela edição e o texto que o assistente escreveu logo antes de fazê-la. Tudo é lido do transcript local.

![Diff de uma edição com o motivo](docs/images/diff.png)

**Modo foco.** O terminal da sessão em tamanho grande, com abas para alternar e um painel com o resumo e a linha do tempo da atividade.

![Modo foco com terminal e resumo](docs/images/foco.png)

**Imagens e links no terminal.** Cole um print com ⌘V ou arraste imagens para o terminal: elas aparecem numa bandeja com miniaturas, saem com um clique no X e vão junto no próximo Enter. Os links aparecem em azul, e ⌘+clique (Ctrl+clique no Linux) abre: páginas no navegador, imagens no visualizador e arquivos no VS Code.

**Retomar sessões.** O histórico inteiro, com busca local (título, conversa, arquivo, branch, tag), agrupado em fixadas, recentes e por projeto. Cada sessão tem um resumo montado sem LLM: o pedido inicial, onde parou, os arquivos tocados, a duração e os tokens escritos. Dá para renomear, fixar, pôr tags e categoria, e retomar no terminal com um clique.

![Histórico com busca e resumo](docs/images/retomar.png)

**Worktrees.** Uma sessão nova pode nascer num worktree isolado, com branch própria. A pasta segue o padrão dos worktrees que você já tem. Antes de criar, o app mostra o comando exato e pede confirmação.

<img src="docs/images/worktree.png" alt="Nova sessão em um worktree" width="560">

**Busca rápida (⌘K).** Sessões abertas, histórico, arquivos, branches e ações, num lugar só.

![Paleta de busca](docs/images/paleta.png)

**Status de verdade.** Nas sessões abertas pelo app, o status vem dos hooks do Claude Code: trabalhando, pedindo permissão (com o comando pedido), esperando você, concluída ou ociosa. Sessões abertas em outros terminais aparecem também, marcadas como externas.

**Barra de menus.** Um ícone mostra quantas sessões precisam de você, com uma lista para pular direto para qualquer uma. Fechar a janela só a esconde; as sessões continuam rodando. (No Linux, fechar a janela encerra o app, porque o ícone da bandeja não aparece em todos os painéis.)

**Tema claro e escuro.** O terminal fica escuro nos dois, porque o Claude Code desenha os diffs e o texto esmaecido pensando em fundo escuro.

![Tema claro](docs/images/grade-claro.png)

## O que ele não faz

- Não edita código, não tem extensões e não substitui o seu editor (o botão abre o VS Code).
- Não mostra cota de uso nem custo. Para isso existe o [ClaudeGauge](https://github.com/PedroHenriqueGazola/ClaudeGauge), e os dois convivem bem.
- Não sincroniza nada com nuvem.

## Garantias

- **`~/.claude/` e `~/.claude.json` são somente leitura.** Toda escrita do app passa por uma checagem que recusa esses caminhos, e um teste de integração exercita o núcleo do app (banco, indexação, hooks e registro de sessões) com `~/.claude` sem permissão de escrita, conferindo que nenhum arquivo ali muda.
- **Sua configuração do Claude Code não muda.** Os hooks do app entram só nas sessões que ele abre, pela flag `claude --settings`, e se somam aos seus. O arquivo de settings não tem segredo: o token de cada sessão vai por variável de ambiente.
- **Zero tokens.** O app nunca chama modelo nenhum nem a API da Anthropic, e não lê o token de login.
- **Git só com confirmação.** As únicas operações que escrevem são criar e remover worktree. A remoção recusa worktrees com alterações pendentes. Nada de `checkout`, `reset`, `stash` ou `--force`.
- **Dados locais.** Nomes, tags, ordem dos cards, índice de busca, configurações e as imagens anexadas (apagadas depois de 3 dias) ficam em `~/Library/Application Support/ClaudeCodeManager/` (no Linux, `~/.local/share/ClaudeCodeManager/`).

## Compilar a partir do código

Requisitos: macOS 14 ou superior (com as Command Line Tools do Xcode) ou Linux com WebKitGTK 4.1, Node 20.19+ e [Rust](https://rustup.rs) estável. No Linux, instale antes as dependências de sistema:

```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf build-essential libssl-dev libxdo-dev
```

```bash
git clone https://github.com/Pedro-B-Siqueira/claude-code-manager.git
cd claude-code-manager
npm install
npm run generate
```

Os instaladores ficam em `src-tauri/target/release/bundle/` (`.app` e `.dmg` no macOS, AppImage e `.deb` no Linux).

Se o Rust foi instalado com `rustup --no-modify-path`, não tem problema: os scripts já incluem `~/.cargo/bin`.

## Uso

| Ação | macOS | Linux |
|---|---|---|
| Busca rápida | ⌘K | Ctrl+Shift+K |
| Nova sessão | ⌘N | Ctrl+Shift+N |
| Ir para a sessão N | ⌘1…9 | Ctrl+Shift+1…9 |
| Configurações | ⌘, | Ctrl+Shift+, |
| Copiar e colar no terminal | ⌘C / ⌘V | Ctrl+Shift+C / Ctrl+Shift+V |
| Abrir link no terminal | ⌘+clique | Ctrl+clique |
| Sair | ⌘Q | Fechar a janela, ou "Sair" no ícone da bandeja |

Sair pede confirmação se alguma sessão estiver trabalhando. No Linux, Ctrl+C e Ctrl+V vão direto para o Claude Code (o Ctrl+V é como ele cola imagens lá).

Sem login no Claude Code, o histórico e a busca funcionam normalmente. Ao abrir uma sessão, o próprio Claude Code pede o login no terminal embutido, como faria em qualquer terminal.

Nas configurações dá para trocar o tema, a pasta e os prefixos de branch dos worktrees, o scrollback, as notificações, a hibernação e o caminho do `claude`.

## Como funciona

- **Histórico.** Os transcripts em `~/.claude/projects` são lidos de forma incremental (só o que foi acrescentado) e indexados num SQLite com busca FTS5. O sistema avisa quando algo muda (FSEvents no macOS, inotify no Linux), sem polling.
- **Sessões vivas.** As do app rodam num PTY pelo seu shell de login, então herdam o mesmo PATH e ambiente do seu terminal. As externas vêm do registro de sessões que o Claude Code mantém em `~/.claude/sessions`.
- **Status.** Um servidor HTTP local, só em `127.0.0.1`, recebe os hooks das sessões do app. Cada sessão tem um token próprio, e o servidor só observa: nunca responde uma decisão no lugar do Claude Code.
- **Tokens.** O card mostra o que o Claude escreveu, somando cada resposta uma vez só (o transcript repete a mesma resposta em várias linhas). O total processado não aparece: cerca de 99% dele é o contexto relido do cache a cada chamada, um número enorme que não diz quanto trabalho foi feito.

Os detalhes estão em [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

## Memória

Medido num Mac Apple Silicon com 16 GB e macOS 26.6, usando `scripts/measure-memory.sh`:

| Cenário | App | WebKit | Terminais | Total |
|---|---|---|---|---|
| Release, 0 sessões | 98 MB | 112 MB | — | **210 MB** |
| Dev, 0 sessões | 131 MB | 163 MB | — | 294 MB |
| Dev, 6 sessões (`fake-claude`) | 133 MB | 183 MB | 57 MB | 373 MB |

O que o app gasta por sessão é pouco: cerca de 0,3 MB no processo principal, uns 3 MB na interface e uns 8 MB para o shell de login. O peso de verdade é o próprio `claude`, que ocupou de 500 a 610 MB por processo na mesma máquina.

Por isso a hibernação vem ligada. Depois de 30 minutos ociosa (configurável), uma sessão aberta pelo app tem o processo encerrado e volta com `claude --resume` quando você a abre. Sessões trabalhando, pedindo permissão ou esperando você nunca hibernam. Além disso, só a sessão em foco tem um terminal desenhado (a grade usa uma prévia em texto), e as prévias param quando a janela está escondida.

## Desenvolvimento

| Comando | O que faz |
|---|---|
| `npm run dev` | App em modo de desenvolvimento |
| `npm run generate` | Build de produção do sistema atual |
| `npm test` | Testes Rust, checagem de tipos e testes do frontend |
| `scripts/measure-memory.sh` | Mede a memória do app aberto (macOS) |

A cada push, o GitHub Actions roda `npm test` no macOS, no Ubuntu x64 e no Ubuntu arm64. Uma tag `vX.Y.Z` gera a Release, em rascunho, com o `.dmg` universal, o AppImage e o `.deb`.

Os testes nunca rodam o `claude` de verdade. Eles usam o `fake-claude` (`src-tauri/crates/fake-claude`), um binário que imita a saída no terminal e chama os hooks declarados no `--settings`, apontado pela variável `CCM_CLAUDE_BIN`. Para ver a cadeia inteira funcionando em dev:

```bash
CCM_E2E=pty-smoke CCM_E2E_CWD=/tmp CCM_CLAUDE_BIN=$PWD/src-tauri/target/debug/fake-claude npm run dev
```

Builds de release ignoram essas variáveis.

## Versões validadas

| | Versão |
|---|---|
| Claude Code | 2.1.295 |
| Tauri | 2.11 |
| Svelte | 5.57 |
| Rust | 1.99 |

O formato dos transcripts e o registro de sessões não são API oficial do Claude Code e podem mudar entre versões. Cada formato fica isolado em um módulo próprio, com testes, e o app mostra "indisponível" em vez de quebrar.

## Créditos

- [**ClaudeGauge**](https://github.com/PedroHenriqueGazola/ClaudeGauge), de Pedro Henrique Gazola (MIT): referência para a leitura do uso nos transcripts e a detecção de sessões vivas. A lógica foi reescrita em Rust, e cada resposta é contada uma vez só.
- [**Geist** e **Geist Mono**](https://vercel.com/font), da Vercel (SIL Open Font License 1.1), embutidas no app.
- Feito com [Tauri](https://tauri.app), [Svelte](https://svelte.dev), [xterm.js](https://xtermjs.org) e [svelte-dnd-action](https://github.com/isaacHagoel/svelte-dnd-action).
