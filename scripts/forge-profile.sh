#!/usr/bin/env bash
# Export or import your Forge OS developer profile (installed catalog list + dotfiles).
set -euo pipefail

CMD="${1:-help}"
ARG="${2:-}"

case "$CMD" in
    export)
        OUT="${ARG:-${HOME}/forge-profile-$(date +%Y%m%d).tar.gz}"
        TMP="$(mktemp -d)"
        trap 'rm -rf "$TMP"' EXIT
        dev list --json > "$TMP/dev-state.json" 2>/dev/null || echo '{}' > "$TMP/dev-state.json"
        mkdir -p "$TMP/home/.config"
        for sub in labwc sway gtk-4.0 foot forge git mise; do
            [ -e "${HOME}/.config/${sub}" ] && cp -a "${HOME}/.config/${sub}" "$TMP/home/.config/"
        done
        for f in .bashrc .profile .gitconfig; do
            [ -e "${HOME}/${f}" ] && cp -a "${HOME}/${f}" "$TMP/home/"
        done
        tar -czf "$OUT" -C "$TMP" .
        echo "Exported Forge OS profile to: $OUT"
        ;;
    import)
        [ -f "$ARG" ] || { echo "Usage: forge-profile import <forge-profile.tar.gz>"; exit 2; }
        TMP="$(mktemp -d)"
        trap 'rm -rf "$TMP"' EXIT
        tar -xzf "$ARG" -C "$TMP"
        [ -d "$TMP/home" ] && cp -a "$TMP/home/." "${HOME}/"
        echo "Restored dotfiles into ${HOME}."
        if [ -f "$TMP/dev-state.json" ]; then
            echo "Saved catalog state from profile:"
            cat "$TMP/dev-state.json"
        fi
        ;;
    *)
        echo "Usage:"
        echo "  forge-profile export [output.tar.gz]"
        echo "  forge-profile import <profile.tar.gz>"
        ;;
esac
