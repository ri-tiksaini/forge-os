#!/usr/bin/env bash
# Generates an Ed25519 SSH key if missing and copies the public key to Wayland clipboard.
set -euo pipefail

KEY="${HOME}/.ssh/id_ed25519"
mkdir -p "${HOME}/.ssh"
chmod 700 "${HOME}/.ssh"

if [ ! -f "$KEY" ]; then
    ssh-keygen -t ed25519 -C "${USER}@$(hostname)" -f "$KEY" -N ""
    echo "[OK] Generated new Ed25519 SSH key at ${KEY}"
else
    echo "[OK] Existing SSH key found at ${KEY}"
fi

if command -v wl-copy >/dev/null 2>&1 && [ -n "${WAYLAND_DISPLAY:-}" ]; then
    wl-copy < "${KEY}.pub"
    echo "[OK] Copied public key to clipboard (ready to paste into GitHub/GitLab)."
fi
echo ""
cat "${KEY}.pub"
