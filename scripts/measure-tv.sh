#!/usr/bin/env bash
#
# Mede o Lume numa TV real (adb) contra os orçamentos da spec resource-efficiency:
#   - Private Dirty do TOTAL em primeiro plano (até 32 MB) e em segundo plano (até 10 MB);
#   - quadros de 33 ms ou mais na navegação do dock e da grade de Apps (até 1%);
#   - PSS, só como referência.
#
# Uso:
#   scripts/measure-tv.sh <serial-adb> <apk-release> [pasta-de-saída]
#   ex.: scripts/measure-tv.sh 192.168.0.9:5555 dist/lume-<versão>-release-debugsigned.apk
#
# Pré-requisitos (não verificados pelo script):
#   - o dock precisa ter os favoritos desejados (13 itens para a tarefa 5.2); confira em home.png;
#   - a TV precisa estar acordada. O script abre o Lume, mas não troca o launcher da TV de volta.
#
# O script desinstala o app antes (a chave de assinatura muda a cada build), instala o APK, define o
# Lume como launcher e grava, na pasta de saída: summary.txt (resultados e vereditos), home.png,
# grid.png e os timestamps brutos de cada rodada (frames-*.txt).
#
# Método: em cada fase (dock e grade) há uma passada de aquecimento, sem contagem (carga de ícones
# logo após a instalação), e depois ROUNDS rodadas medidas de KEYS_PER_ROUND teclas, com o buffer
# de quadros zerado antes de cada uma.
set -uo pipefail

SERIAL="${1:-}"
APK="${2:-}"
if [ -z "$SERIAL" ] || [ -z "$APK" ]; then
    sed -n '2,25p' "$0" | sed 's/^# \{0,1\}//'
    exit 2
fi
if [ ! -f "$APK" ]; then
    echo "APK não encontrado: $APK" >&2
    exit 2
fi

PKG="app.lume.tv"
LAYER="app.lume.tv/app.lume.tv.LumeActivity#0"
STAMP="$(date +%Y%m%d-%H%M%S)"
OUT="${3:-dist/tv-measure/${STAMP}-${SERIAL//[:.]/_}}"
mkdir -p "$OUT"
SUMMARY="$OUT/summary.txt"
: > "$SUMMARY"
ADB=(adb -s "$SERIAL")

KEY_UP=19
KEY_DOWN=20
KEY_LEFT=21
KEY_RIGHT=22
ROUNDS=4
KEYS_PER_ROUND=12
KEY_GAP=0.4
FRAME_MS=33
LIMIT_FRAMES_PCT=1

say() { echo "$*" | tee -a "$SUMMARY"; }
tap_key() { "${ADB[@]}" shell input keyevent "$1" >/dev/null 2>&1; sleep "$KEY_GAP"; }

# Linha TOTAL do dumpsys meminfo: $2 = PSS, $3 = Private Dirty (KB).
meminfo_total() {
    "${ADB[@]}" shell dumpsys meminfo "$PKG" 2>/dev/null | tr -d '\r' \
        | awk '/^ *TOTAL +[0-9]/ {print $2, $3; exit}'
}

# A camada do Lume muda depois de reinstalar: escolhe a que tem apresentações reais.
find_layer() {
    local cand n best="" best_n=-1
    while read -r cand; do
        n=$("${ADB[@]}" shell dumpsys SurfaceFlinger --latency "$cand" 2>/dev/null </dev/null | tr -d '\r' \
            | awk 'NR > 1 && $2 > 0 && $2 < 9e18 { n++ } END { print n + 0 }')
        # Empate (app parado: todas com 0) fica com a camada de maior número, a mais recente.
        if [ "$n" -ge "$best_n" ]; then best="$cand"; best_n="$n"; fi
    done < <("${ADB[@]}" shell dumpsys SurfaceFlinger --list 2>/dev/null | tr -d '\r' \
        | grep -o 'app\.lume\.tv/app\.lume\.tv\.LumeActivity[^ ]*' | sort -t'#' -k2 -n)
    if [ -n "$best" ]; then LAYER="$best"; fi
}

# Quadros do SurfaceFlinger: conta intervalos entre apresentações reais >= FRAME_MS.
# Imprime "perdidos total" e grava os timestamps brutos no arquivo $1.
frame_stats() {
    local raw="$1"
    "${ADB[@]}" shell dumpsys SurfaceFlinger --latency "$LAYER" 2>/dev/null | tr -d '\r' > "$raw"
    awk -v th="$FRAME_MS" '
        NR > 1 && NF == 3 && $2 > 0 && $2 < 9e18 { t[++n] = $2 }
        END {
            over = 0
            for (i = 2; i <= n; i++) if ((t[i] - t[i - 1]) / 1e6 >= th) over++
            print over + 0, (n > 0 ? n - 1 : 0)
        }' "$raw"
}

# Passada sem contagem: aquece ícones e a grade antes de medir.
warm_pass() {
    local keys=("$@")
    local k
    for k in $(seq 1 "$KEYS_PER_ROUND"); do
        tap_key "${keys[$(( (k - 1) % ${#keys[@]} ))]}"
    done
}

# Uma fase medida: ROUNDS rodadas de KEYS_PER_ROUND teclas. Imprime o resultado em TOTAL_PCT_<name>.
# $1 = nome da fase; demais argumentos = sequência de teclas (uma rodada), repetida em ciclo.
run_stage() {
    local name="$1"; shift
    local keys=("$@")
    local over_total=0 frames_total=0 r k over frames pct
    for r in $(seq 1 "$ROUNDS"); do
        "${ADB[@]}" shell dumpsys SurfaceFlinger --latency-clear "$LAYER" >/dev/null 2>&1
        for k in $(seq 1 "$KEYS_PER_ROUND"); do
            tap_key "${keys[$(( (k - 1) % ${#keys[@]} ))]}"
        done
        sleep 1
        read -r over frames < <(frame_stats "$OUT/frames-${name}-r${r}.txt")
        say "  $name rodada $r: $over de $frames quadros de ${FRAME_MS} ms ou mais"
        over_total=$(( over_total + over ))
        frames_total=$(( frames_total + frames ))
    done
    if [ "$frames_total" -gt 0 ]; then
        pct=$(awk -v a="$over_total" -v b="$frames_total" 'BEGIN { printf "%.2f", 100 * a / b }')
        say "  $name total: $over_total de $frames_total (${pct}%)"
        eval "TOTAL_PCT_${name}=${pct}"
    else
        say "  $name total: sem quadros medidos (camada $LAYER)"
        eval "TOTAL_PCT_${name}=nan"
    fi
}

# Vai da tela inicial (foco no dock) para a grade de Apps: sobe à barra, abre a aba Apps e desce.
goto_grid() {
    "${ADB[@]}" shell input keyevent "$KEY_UP" >/dev/null 2>&1; sleep "$KEY_GAP"
    "${ADB[@]}" shell input keyevent "$KEY_UP" >/dev/null 2>&1; sleep "$KEY_GAP"
    "${ADB[@]}" shell input keyevent "$KEY_RIGHT" >/dev/null 2>&1; sleep 1
    "${ADB[@]}" shell input keyevent "$KEY_DOWN" >/dev/null 2>&1; sleep 1
}

verdict() { # $1 = nome, $2 = valor medido, $3 = limite, $4 = unidade de $2
    local v
    if [ "$2" = "nan" ] || [ -z "$2" ]; then
        v="SEM MEDIDA"
    else
        v=$(awk -v a="$2" -v b="$3" 'BEGIN { print (a <= b) ? "PASSA" : "NÃO PASSA" }')
    fi
    say "  $1: $2 $4 (limite $3 $4): $v"
}

say "== $SERIAL ($(date '+%Y-%m-%d %H:%M'))"
say "Modelo: $("${ADB[@]}" shell getprop ro.product.model | tr -d '\r') | Android $("${ADB[@]}" shell getprop ro.build.version.release | tr -d '\r') | $("${ADB[@]}" shell wm size | tr -d '\r')"
say "APK: $APK"

# 1. Instalação limpa.
"${ADB[@]}" uninstall "$PKG" >/dev/null 2>&1
install_out="$("${ADB[@]}" install -r "$APK" 2>&1 | tr -d '\r' | tail -n 1)"
say "Instalação: $install_out"
if [ "$install_out" != "Success" ]; then
    say "Falha na instalação; medição interrompida."
    exit 1
fi
"${ADB[@]}" shell cmd package set-home-activity "$PKG/.LumeActivity" >/dev/null 2>&1
"${ADB[@]}" shell am start -n "$PKG/.LumeActivity" >/dev/null 2>&1

# 2. Primeiro plano logo após a abertura e depois de 30 s parado.
sleep 15
"${ADB[@]}" exec-out screencap -p > "$OUT/home.png" 2>/dev/null
find_layer
say "Camada de quadros: $LAYER"
read -r pss dirty < <(meminfo_total)
FG_DIRTY_OPEN="${dirty:-nan}"
say "Primeiro plano (15 s após abrir): Private Dirty ${dirty:-?} KB, PSS ${pss:-?} KB"
sleep 30
read -r pss dirty < <(meminfo_total)
FG_DIRTY="${dirty:-nan}"
say "Primeiro plano parado (45 s): Private Dirty ${dirty:-?} KB, PSS ${pss:-?} KB"

# 3. Dock (foco começa no dock): direita 6 vezes e volta 6. Aquece, depois mede.
say "Dock (aquecimento sem contagem, depois medido):"
find_layer
warm_pass "$KEY_RIGHT" "$KEY_RIGHT" "$KEY_RIGHT" "$KEY_RIGHT" "$KEY_RIGHT" "$KEY_RIGHT" \
    "$KEY_LEFT" "$KEY_LEFT" "$KEY_LEFT" "$KEY_LEFT" "$KEY_LEFT" "$KEY_LEFT"
find_layer
run_stage dock "$KEY_RIGHT" "$KEY_RIGHT" "$KEY_RIGHT" "$KEY_RIGHT" "$KEY_RIGHT" "$KEY_RIGHT" \
    "$KEY_LEFT" "$KEY_LEFT" "$KEY_LEFT" "$KEY_LEFT" "$KEY_LEFT" "$KEY_LEFT"

# 4. Grade de Apps: aquecimento, screenshot para conferir o foco, depois medido.
goto_grid
"${ADB[@]}" exec-out screencap -p > "$OUT/grid.png" 2>/dev/null
say "Grade (aquecimento sem contagem, depois medido; confira grid.png):"
find_layer
warm_pass "$KEY_RIGHT" "$KEY_RIGHT" "$KEY_DOWN" "$KEY_LEFT" "$KEY_LEFT" "$KEY_UP"
find_layer
run_stage grade "$KEY_RIGHT" "$KEY_RIGHT" "$KEY_DOWN" "$KEY_LEFT" "$KEY_LEFT" "$KEY_UP"

# 5. Segundo plano: abre as Configurações do Android e mede o Lume sem foco.
"${ADB[@]}" shell am start -a android.settings.SETTINGS >/dev/null 2>&1
sleep 8
read -r pss dirty < <(meminfo_total)
if [ -z "${dirty:-}" ]; then
    BG_DIRTY="nan"
    say "Segundo plano (8 s): processo do Lume encerrado pelo sistema (sem medida)"
else
    BG_DIRTY="$dirty"
    say "Segundo plano (8 s): Private Dirty ${dirty} KB, PSS ${pss} KB"
fi
"${ADB[@]}" shell am start -n "$PKG/.LumeActivity" >/dev/null 2>&1
sleep 3

# 6. Vereditos.
say "Vereditos (spec resource-efficiency):"
verdict "Private Dirty em primeiro plano (45 s parado)" "$(awk -v a="$FG_DIRTY" 'BEGIN{ if (a == "nan") print "nan"; else printf "%.1f", a/1024 }')" "32" "MB"
verdict "Private Dirty em segundo plano" "$(awk -v a="$BG_DIRTY" 'BEGIN{ if (a == "nan") print "nan"; else printf "%.1f", a/1024 }')" "10" "MB"
verdict "Dock, quadros de ${FRAME_MS} ms ou mais" "${TOTAL_PCT_dock}" "$LIMIT_FRAMES_PCT" "%"
verdict "Grade, quadros de ${FRAME_MS} ms ou mais" "${TOTAL_PCT_grade}" "$LIMIT_FRAMES_PCT" "%"
say "Não medido por este script: modo Espiada; número de favoritos do dock (ver home.png)."
say "Resumo salvo em $SUMMARY"
