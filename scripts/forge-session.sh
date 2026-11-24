#!/usr/bin/env bash
# Launches labwc (real minimize/maximize/close titlebars + click-and-drag move/resize)
# paired with native edge-pinned Waybar & wlr/taskbar.
set -euo pipefail

export XDG_CURRENT_DESKTOP=labwc
export GTK_THEME=Adwaita:dark
export GSK_RENDERER=cairo
export WLR_RENDERER="${WLR_RENDERER:-pixman}"
export WLR_NO_HARDWARE_CURSORS=1
exec labwc
