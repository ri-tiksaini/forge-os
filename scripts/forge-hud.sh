#!/usr/bin/env bash
# Zero-flicker Dark Cherry Live Telemetry HUD
set -u

RED=$'\033[38;2;155;17;38m'
PINK=$'\033[38;2;196;43;66m'
WHITE=$'\033[38;2;244;244;245m'
DIM=$'\033[38;2;161;161;170m'
BOLD=$'\033[1m'
RST=$'\033[0m'
EOL=$'\033[K'

prev_total=0
prev_idle=0
prev_rx=0
prev_tx=0

make_bar() {
    local pct=$1 width=20
    local filled=$(( (pct * width) / 100 ))
    local empty=$(( width - filled ))
    local out="${RED}"
    for ((i=0; i<filled; i++)); do out+="█"; done
    out+="${DIM}"
    for ((i=0; i<empty; i++)); do out+="░"; done
    printf "%s%s" "$out" "$RST"
}

printf '\033[?25l\033[2J\033[H'
trap 'printf "\033[?25h"; exit 0' INT TERM EXIT

while true; do
    read -r _ u n s id io irq sirq _ < /proc/stat
    total=$((u + n + s + id + io + irq + sirq))
    idle=$((id + io))
    dt=$((total - prev_total))
    di=$((idle - prev_idle))
    cpu=0
    [ "$dt" -gt 0 ] && cpu=$(( (100 * (dt - di)) / dt ))
    prev_total=$total
    prev_idle=$idle

    mem_tot=$(awk '/^MemTotal:/ {print int($2/1024)}' /proc/meminfo)
    mem_avail=$(awk '/^MemAvailable:/ {print int($2/1024)}' /proc/meminfo)
    mem_used=$((mem_tot - mem_avail))
    mem_pct=0
    [ "$mem_tot" -gt 0 ] && mem_pct=$(( (mem_used * 100) / mem_tot ))

    disk_pct=$(df -P / | awk 'NR==2 {gsub("%",""); print $5}')

    rx_now=$(awk -F'[: ]+' 'NR>2 && $2!="lo" {r+=$3} END{print int(r)}' /proc/net/dev)
    tx_now=$(awk -F'[: ]+' 'NR>2 && $2!="lo" {t+=$11} END{print int(t)}' /proc/net/dev)
    rx_k=0; tx_k=0
    [ "$prev_rx" -gt 0 ] && rx_k=$(( (rx_now - prev_rx) / 1024 ))
    [ "$prev_tx" -gt 0 ] && tx_k=$(( (tx_now - prev_tx) / 1024 ))
    prev_rx=$rx_now; prev_tx=$tx_now

    gpu=$(basename "$(readlink -f /sys/class/drm/card0/device/driver 2>/dev/null)" 2>/dev/null || echo "sw")
    bat="AC / Desktop"
    for b in /sys/class/power_supply/BAT*/capacity; do
        [ -r "$b" ] && bat="$(cat "$b")%" && break
    done

    pg_state=$(systemctl is-active postgresql 2>/dev/null || echo "not-installed")
    forged_state=$(systemctl is-active forged 2>/dev/null || echo "idle")

    cpu_bar=$(make_bar "$cpu")
    ram_bar=$(make_bar "$mem_pct")
    disk_bar=$(make_bar "$disk_pct")

    frame=$(printf '\033[H'
        printf '%s%s┌──────────────────────────────────────────────┐%s%s\n' "$RED" "$BOLD" "$RST" "$EOL"
        printf '%s%s│  FORGE OS 1.0  //  CRIMSON TELEMETRY HUD     │%s%s\n' "$RED" "$BOLD" "$RST" "$EOL"
        printf '%s%s└──────────────────────────────────────────────┘%s%s\n%s\n' "$RED" "$BOLD" "$RST" "$EOL" "$EOL"
        printf ' %sCPU  %s %s %s%3d%%%s%s\n' "$WHITE" "$RST" "$cpu_bar" "$PINK" "$cpu" "$RST" "$EOL"
        printf ' %sRAM  %s %s %s%3d%%%s %s(%d/%d MiB)%s%s\n' "$WHITE" "$RST" "$ram_bar" "$PINK" "$mem_pct" "$RST" "$DIM" "$mem_used" "$mem_tot" "$RST" "$EOL"
        printf ' %sDISK %s %s %s%3d%%%s%s\n%s\n' "$WHITE" "$RST" "$disk_bar" "$PINK" "$disk_pct" "$RST" "$EOL" "$EOL"
        printf ' %sGPU      :%s %s%s%s%s\n' "$DIM" "$RST" "$WHITE" "$gpu" "$RST" "$EOL"
        printf ' %sNETWORK  :%s %s↓ %d KB/s%s  %s↑ %d KB/s%s%s\n' "$DIM" "$RST" "$PINK" "$rx_k" "$RST" "$WHITE" "$tx_k" "$RST" "$EOL"
        printf ' %sPOWER    :%s %s%s%s%s\n' "$DIM" "$RST" "$WHITE" "$bat" "$RST" "$EOL"
        printf ' %sDAEMON   :%s forged=%s%s%s  postgres=%s%s%s%s\n' "$DIM" "$RST" "$PINK" "$forged_state" "$RST" "$WHITE" "$pg_state" "$RST" "$EOL"
        printf ' %sTIME     :%s %s%s%s%s\n%s\n' "$DIM" "$RST" "$WHITE" "$(date '+%Y-%m-%d %H:%M:%S')" "$RST" "$EOL" "$EOL"
        printf ' %sShortcuts: [Win] Apps | [Alt+Tab] Switch | [Alt+F4] Close%s%s\033[J' "$DIM" "$RST" "$EOL"
    )
    printf '%s' "$frame"
    sleep 1
done
