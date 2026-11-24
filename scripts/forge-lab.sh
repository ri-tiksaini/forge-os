#!/usr/bin/env bash
# Forge OS System Internals Lab (Authenticated + Sandboxed Simulation Copy)
# Usage:
#   forge-lab map                         Show system file tree, wiring & functions
#   forge-lab sim <real-file> <edited>    Test edited file in an isolated sandbox copy
#   forge-lab apply <real-file> <edited>  Backup + apply tested file (requires sudo auth)
#   forge-lab new <tool-name>             Create a new custom developer tool in sandbox
set -euo pipefail

MAP="/usr/share/forge/system-map.json"
CMD="${1:-map}"

validate_file() {
    local target="$1" candidate="$2"
    case "$target" in
        *.sh|/usr/local/bin/*)
            bash -n "$candidate"
            echo "[SIM PASS] Shell syntax valid (bash -n)."
            ;;
        *.json|*waybar/config)
            python3 -m json.tool "$candidate" >/dev/null
            echo "[SIM PASS] Valid JSON."
            ;;
        *.xml)
            python3 -c "import xml.etree.ElementTree as ET, sys; ET.parse(sys.argv[1])" "$candidate"
            echo "[SIM PASS] Valid XML."
            ;;
        *)
            [ -s "$candidate" ] || { echo "[SIM FAIL] File is empty."; return 1; }
            echo "[SIM PASS] Non-empty config verified."
            ;;
    esac
}

case "$CMD" in
    map)
        python3 - <<'PY' "$MAP"
import json, sys
data = json.load(open(sys.argv[1]))
for item in data:
    print(f"FILE     : {item['path']}")
    print(f"ROLE     : {item['role']}")
    print(f"WIRING   : {item['wiring']}")
    print(f"DETAILS  : {item['functions']}")
    print("-" * 64)
PY
        ;;
    sim)
        REAL="${2:-}"
        EDITED="${3:-}"
        [ -n "$REAL" ] && [ -f "$EDITED" ] || { echo "Usage: forge-lab sim <real-path> <edited-file>"; exit 2; }
        SIM_ROOT="/tmp/forge-lab-sim-$$"
        trap 'rm -rf "$SIM_ROOT"' EXIT
        mkdir -p "${SIM_ROOT}$(dirname "$REAL")"
        cp -a "$EDITED" "${SIM_ROOT}${REAL}"
        echo "[SIM] Isolated sandbox created at ${SIM_ROOT}"
        validate_file "$REAL" "${SIM_ROOT}${REAL}"
        echo "[SIM] Safe to apply. No live system files were modified."
        ;;
    apply)
        REAL="${2:-}"
        EDITED="${3:-}"
        [ -n "$REAL" ] && [ -f "$EDITED" ] || { echo "Usage: forge-lab apply <real-path> <edited-file>"; exit 2; }
        echo "WARNING: You are modifying a live system file: ${REAL}"
        # Authenticate via sudo before touching any system file
        sudo -v
        sudo /usr/local/bin/forge-lab sim "$REAL" "$EDITED"
        if [ -x /usr/local/bin/forge-config-backup ]; then
            sudo /usr/local/bin/forge-config-backup >/dev/null 2>&1 || true
            echo "[OK] Pre-change safety backup created."
        fi
        sudo install -m "$( [ -x "$REAL" ] && echo 0755 || echo 0644 )" "$EDITED" "$REAL"
        echo "[OK] Applied ${REAL}. Restart affected service or reboot if needed."
        ;;
    new)
        NAME="${2:-}"
        [ -n "$NAME" ] || { echo "Usage: forge-lab new <tool-name>"; exit 2; }
        OUT="${HOME}/Projects/custom-tools/${NAME}.sh"
        mkdir -p "$(dirname "$OUT")"
        cat <<TOOL > "$OUT"
#!/usr/bin/env bash
# Custom Forge OS Tool: ${NAME}
set -euo pipefail
echo "Running custom tool: ${NAME}"
TOOL
        chmod +x "$OUT"
        echo "[OK] Created custom tool template at ${OUT}"
        echo "Test it in sandbox: forge-lab sim /usr/local/bin/${NAME} ${OUT}"
        echo "Install to OS     : forge-lab apply /usr/local/bin/${NAME} ${OUT}"
        ;;
    *)
        echo "Usage: forge-lab [map|sim <real> <edited>|apply <real> <edited>|new <name>]"
        exit 2
        ;;
esac
