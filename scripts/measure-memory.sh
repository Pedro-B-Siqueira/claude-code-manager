#!/bin/bash
# Mede a memória do Claude Code Manager: processo do app, processos do WebKit que ele criou
# e a árvore de processos dos terminais (shell + claude). Só lê `ps`; não altera nada.
#
# Uso:
#   scripts/measure-memory.sh baseline   # antes de abrir o app: anota os processos WebKit existentes
#   scripts/measure-memory.sh report     # com o app aberto: mostra o consumo atual
set -euo pipefail

BASELINE_FILE="${TMPDIR:-/tmp}/ccm-webkit-baseline.txt"
APP_PATTERN='[Cc]laude[ -][Cc]ode[ -][Mm]anager.app/Contents/MacOS|target/(debug|release)/claude-code-manager$'

webkit_pids() {
  ps -axo pid=,comm= | awk '/com\.apple\.WebKit\./ {print $1}' | sort -n
}

rss_kb() {
  ps -o rss= -p "$1" 2>/dev/null | tr -d ' ' || echo 0
}

descendants() {
  local parent="$1"
  for child in $(ps -axo pid=,ppid= | awk -v p="$parent" '$2 == p {print $1}'); do
    echo "$child"
    descendants "$child"
  done
}

to_mb() {
  awk -v kb="$1" 'BEGIN { printf "%.0f MB", kb / 1024 }'
}

case "${1:-report}" in
  baseline)
    webkit_pids > "$BASELINE_FILE"
    echo "Processos WebKit anotados em $BASELINE_FILE. Abra o app e rode: $0 report"
    ;;
  report)
    app_pid=$(pgrep -f "$APP_PATTERN" | head -1 || true)
    if [[ -z "$app_pid" ]]; then
      echo "O Claude Code Manager não está aberto." >&2
      exit 1
    fi
    app_kb=$(rss_kb "$app_pid")
    webkit_kb=0
    if [[ -f "$BASELINE_FILE" ]]; then
      for pid in $(comm -13 "$BASELINE_FILE" <(webkit_pids)); do
        webkit_kb=$((webkit_kb + $(rss_kb "$pid")))
      done
    fi
    sessions_kb=0
    sessions=0
    for child in $(ps -axo pid=,ppid= | awk -v p="$app_pid" '$2 == p {print $1}'); do
      sessions=$((sessions + 1))
      tree_kb=$(rss_kb "$child")
      for pid in $(descendants "$child"); do
        tree_kb=$((tree_kb + $(rss_kb "$pid")))
      done
      sessions_kb=$((sessions_kb + tree_kb))
    done
    echo "App (processo principal):      $(to_mb "$app_kb")"
    if [[ -f "$BASELINE_FILE" ]]; then
      echo "WebKit (processos do app):     $(to_mb "$webkit_kb")"
    else
      echo "WebKit: rode '$0 baseline' antes de abrir o app para medir"
    fi
    echo "Terminais ($sessions processos filhos): $(to_mb "$sessions_kb")"
    echo "Total:                         $(to_mb $((app_kb + webkit_kb + sessions_kb)))"
    ;;
  *)
    echo "uso: $0 [baseline|report]" >&2
    exit 2
    ;;
esac
