# Forge OS Local Developer Error Tracker (0 MB RAM, local-only, zero telemetry)
if [ -n "${BASH_VERSION:-}" ] && [[ $- == *i* ]]; then
    export FORGE_SESSION_ID="${FORGE_SESSION_ID:-sess-$$-$(date +%H%M%S)}"
    __forge_track_hook() {
        local rc=$?
        [ "$rc" -eq 0 ] && return 0
        local logdir="${HOME}/.local/share/forge"
        local logfile="${logdir}/session-errors.log"
        local cmd
        cmd="$(HISTTIMEFORMAT='' history 1 | sed 's/^[ ]*[0-9]\+[ ]*//')"
        [ -n "$cmd" ] || return 0
        mkdir -p "$logdir"
        printf '%s\t%s\texit=%d\t%s\t%s\n' \
            "$(date '+%Y-%m-%d %H:%M:%S')" \
            "$FORGE_SESSION_ID" \
            "$rc" \
            "$PWD" \
            "$cmd" >> "$logfile"
        if [ "$(wc -l < "$logfile" 2>/dev/null || echo 0)" -gt 500 ]; then
            tail -n 500 "$logfile" > "${logfile}.tmp" && mv "${logfile}.tmp" "$logfile"
        fi
    }
    case "${PROMPT_COMMAND:-}" in
        *__forge_track_hook*) ;;
        "") PROMPT_COMMAND="__forge_track_hook" ;;
        *) PROMPT_COMMAND="__forge_track_hook; ${PROMPT_COMMAND}" ;;
    esac
fi
