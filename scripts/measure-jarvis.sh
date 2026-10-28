#!/bin/bash
# Mede RAM e CPU do Modo Jarvis numa instância isolada do app: HOME temporário, então banco,
# ajustes e ~/.claude são próprios e vazios. O próprio app abre e fecha o Jarvis num timer
# (CCM_JARVIS_MEASURE, só em build de desenvolvimento) com as 4 sessões de teste do design
# (CCM_JARVIS_MOCK=1). Só lê `ps`; não toca no app instalado nem nos dados do usuário.
#
# Uso:
#   scripts/measure-jarvis.sh <binário> [ciclos=1] [pixel_ratio] [segundos_aberto=90] [segundos_fechado=30]
# O primeiro ciclo abre aos 20 s. pixel_ratio vazio ("") mantém a escala padrão.
# O binário vem de `npm run tauri build -- --debug --bundles app`:
#   "src-tauri/target/debug/bundle/macos/Claude Code Manager.app/Contents/MacOS/claude-code-manager"
# A janela do Jarvis cobre a tela durante a medição.
set -euo pipefail

binary="${1:?informe o binário do app}"
cycles="${2:-1}"
pixel_ratio="${3:-}"
first_open=20
open_seconds="${4:-90}"
cycle_seconds=$((open_seconds + ${5:-30}))

home=$(mktemp -d "${TMPDIR:-/tmp}/ccm-jarvis-measure.XXXXXX")
# Como numa instalação normal do Claude Code, que já tem essas pastas.
mkdir -p "$home/.claude/sessions" "$home/.claude/projects"
samples="$home/samples.tsv"

schedule=""
for ((cycle = 0; cycle < cycles; cycle++)); do
  opens=$((first_open + cycle * cycle_seconds))
  schedule+="${schedule:+,}$opens,$((opens + open_seconds))"
done
last_close=$((first_open + (cycles - 1) * cycle_seconds + open_seconds))

webkit_pids() {
  ps -axo pid=,comm= | awk '/com\.apple\.WebKit\./ {print $1}' | sort
}

before_launch=$(webkit_pids)
env HOME="$home" CCM_JARVIS_MOCK=1 CCM_JARVIS_MEASURE="$schedule" ${pixel_ratio:+CCM_JARVIS_PIXEL_RATIO=$pixel_ratio} "$binary" >"$home/app.log" 2>&1 &
app_pid=$!
trap 'kill "$app_pid" 2>/dev/null || true; wait "$app_pid" 2>/dev/null || true; rm -rf "$home"' EXIT

started=$(date +%s)
while true; do
  elapsed=$(($(date +%s) - started))
  ((elapsed > last_close + 12)) && break
  pids=$(printf '%s\n' "$app_pid" $(comm -13 <(echo "$before_launch") <(webkit_pids)) | paste -sd, -)
  # tempo, pid, RSS (KB), CPU acumulada (s), tipo
  ps -o pid=,rss=,time=,comm= -p "$pids" 2>/dev/null | awk -v t="$elapsed" '{
    n = split($3, part, ":"); cpu = (n == 3) ? part[1] * 3600 + part[2] * 60 + part[3] : part[1] * 60 + part[2]
    kind = "app"
    if ($4 ~ /WebContent/) kind = "webcontent"; else if ($4 ~ /GPU/) kind = "gpu"; else if ($4 ~ /Networking/) kind = "networking"
    printf "%d\t%d\t%d\t%.2f\t%s\n", t, $1, $2, cpu, kind
  }' >>"$samples"
  sleep 1
done

cp "$samples" "${TMPDIR:-/tmp}/ccm-jarvis-samples-${pixel_ratio:-default}.tsv"
awk -F'\t' -v schedule="$schedule" -v ratio="${pixel_ratio:-1,0 (padrão)}" '
  { rss[$1] += $3; process_rss[$1, $2] = $3; cpu[$1, $2] = $4; kind[$2] = $5; if (!($2 in first)) first[$2] = $1 }
  function mb(kb) { return kb / 1024 }
  function average_rss(from, to,    t, total, count) {
    for (t = from; t <= to; t++) if (t in rss) { total += rss[t]; count++ }
    return count ? total / count : 0
  }
  function cpu_percent(from, to,    pid, used) {
    for (pid in first) if (((from, pid) in cpu) && ((to, pid) in cpu)) used += cpu[to, pid] - cpu[from, pid]
    return (to > from) ? 100 * used / (to - from) : 0
  }
  function kind_rss(t, wanted,    pid, total) {
    for (pid in first) if (kind[pid] == wanted && ((t, pid) in process_rss)) total += process_rss[t, pid]
    return total
  }
  function nearest(t) { while (!(t in rss) && t > 0) t--; return t }
  END {
    count = split(schedule, moments, ",")
    base = average_rss(moments[1] - 10, moments[1] - 1)
    printf "Escala do canvas: %s\n", ratio
    printf "Jarvis fechado (baseline)  RSS app+WebKit: %6.1f MB   CPU: %4.1f%% de um núcleo   (app %.1f, GPU %.1f MB)\n", mb(base), cpu_percent(moments[1] - 10, moments[1] - 1), mb(kind_rss(moments[1] - 1, "app")), mb(kind_rss(moments[1] - 1, "gpu"))
    for (cycle = 1; cycle * 2 <= count; cycle++) {
      opened = moments[cycle * 2 - 1]; closed = moments[cycle * 2]
      live = average_rss(opened + 10, closed - 1)
      after = nearest(closed + 10)
      printf "Ciclo %d  aberto: %+6.1f MB, CPU %4.1f%%   |   10 s depois de sair: %+6.1f MB (app %+.1f, GPU %+.1f)\n", cycle, mb(live - base), cpu_percent(opened + 10, closed - 1), mb(rss[after] - base), mb(kind_rss(after, "app") - kind_rss(moments[1] - 1, "app")), mb(kind_rss(after, "gpu") - kind_rss(moments[1] - 1, "gpu"))
    }
  }' "$samples"
