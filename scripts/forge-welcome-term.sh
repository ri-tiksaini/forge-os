#!/usr/bin/env bash
RED='\033[38;2;155;17;38m'
WHITE='\033[38;2;244;244;245m'
DIM='\033[38;2;161;161;170m'
BOLD='\033[1m'
RST='\033[0m'

clear
printf "${RED}${BOLD}"
cat <<'DUCK'
       __
     <(o )___     FORGE OS 1.0  //  CRIMSON DUCK
      ( ._> /     Developer Workstation (Debian 13 + Rust)
       `---'
DUCK
printf "${RST}\n"
printf " ${WHITE}Quick Commands:${RST}\n"
printf "   ${RED}dev search <name>${RST}   Search software catalog (5,000-item engine)\n"
printf "   ${RED}dev add postgres${RST}    Install zero-RAM parked PostgreSQL\n"
printf "   ${RED}dev doctor${RST}          Hardware tier & daemon health\n"
printf "   ${RED}mousepad <file>${RST}     Open Forge Code Editor (Notepad)\n"
printf "   ${RED}forge-settings${RST}      Open Control Center & Wi-Fi\n\n"
printf " ${DIM}Tip: Ctrl+V pastes | Ctrl+Shift+C copies | Click bottom bar for all apps.${RST}\n\n"
exec bash -l
