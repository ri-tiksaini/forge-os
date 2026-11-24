#!/usr/bin/env bash
# Forge OS Whole-OS Theme Engine
# Applies selected theme (accent, highlight, bg) to GTK4, GTK3, Waybar, labwc, foot,
# and ~/.config/forge/active-theme.env so EVERY window and button border matches!
set -euo pipefail

THEME_NAME="${1:-Dark Cherry}"
ACCENT="${2:-#9B1126}"
ACCENT_HI="${3:-#C42B42}"
BG="${4:-#070709}"
FG="#ECECEF"

mkdir -p "$HOME/.config/forge" "$HOME/.config/gtk-4.0" "$HOME/.config/gtk-3.0" \
         "$HOME/.config/waybar" "$HOME/.themes/Forge-Cherry/openbox-3" "$HOME/.config/foot"

# 1. Shared OS-wide active theme state (read by forge-settings, forge-shell, forge-lab)
cat > "$HOME/.config/forge/active-theme.env" <<EOF
THEME_NAME=$THEME_NAME
ACCENT=$ACCENT
ACCENT_HI=$ACCENT_HI
BG=$BG
EOF

# 2. Global GTK4 & GTK3 CSS (applies theme borders to EVERY button, entry, card, slider & switch)
GTK_CSS="/* Forge OS Global Theme: $THEME_NAME */
@define-color window_bg_color $BG;
@define-color window_fg_color $FG;
@define-color view_bg_color $BG;
@define-color view_fg_color $FG;
@define-color headerbar_bg_color $BG;
@define-color headerbar_fg_color $FG;
@define-color card_bg_color #121217;
@define-color accent_bg_color $ACCENT;
@define-color accent_color $ACCENT_HI;
@define-color accent_fg_color #FFFFFF;

window, dialog, popover > contents {
    background-color: $BG;
    color: $FG;
}
button {
    border: 1px solid $ACCENT;
    border-radius: 8px;
}
button:hover, button:focus {
    border-color: $ACCENT_HI;
}
button.suggested-action, button.destructive-action {
    background-color: $ACCENT;
    border-color: $ACCENT_HI;
    color: #FFFFFF;
}
entry, searchentry, spinbutton, combobox button {
    border: 1px solid $ACCENT;
    border-radius: 8px;
}
entry:focus-within, searchentry:focus-within {
    border-color: $ACCENT_HI;
}
scale highlight, progressbar progress {
    background-color: $ACCENT;
}
scale slider {
    border: 2px solid $ACCENT;
}
switch {
    border: 1px solid $ACCENT;
}
switch:checked {
    background-color: $ACCENT;
    border-color: $ACCENT_HI;
}
scrollbar slider:hover {
    background-color: $ACCENT;
}
"
printf '%s\n' "$GTK_CSS" > "$HOME/.config/gtk-4.0/gtk.css"
printf '%s\n' "$GTK_CSS" > "$HOME/.config/gtk-3.0/gtk.css"

# 3. Waybar Bottom Taskbar Theme (live reload)
WAYBAR_SRC="/usr/share/forge/waybar/style.css"
if [ -f "$WAYBAR_SRC" ]; then
    sed -e "s/#9B1126/$ACCENT/gI" \
        -e "s/#C42B42/$ACCENT_HI/gI" \
        -e "s/#070709/$BG/gI" \
        "$WAYBAR_SRC" > "$HOME/.config/waybar/style.css"
    cat >> "$HOME/.config/waybar/style.css" <<EOF

/* Selected OS Theme Button Borders */
button, #taskbar button, #custom-duck, #custom-quick, #custom-clean, #custom-power, #clock, #cpu, #memory {
    border: 1px solid $ACCENT;
}
button:hover, #taskbar button.active {
    border-color: $ACCENT_HI;
}
EOF
    if pgrep -x waybar >/dev/null 2>&1; then
        pkill -x waybar || true
        (nohup waybar -c /usr/share/forge/waybar/config -s "$HOME/.config/waybar/style.css" >/dev/null 2>&1 &)
    fi
fi

# 4. labwc Window Titlebar & Active Window Border Theme (live reload)
THEMERC_SRC="/usr/share/themes/Forge-Cherry/openbox-3/themerc"
if [ -f "$THEMERC_SRC" ]; then
    sed -e "s/#9B1126/$ACCENT/gI" \
        -e "s/#C42B42/$ACCENT_HI/gI" \
        -e "s/#070709/$BG/gI" \
        "$THEMERC_SRC" > "$HOME/.themes/Forge-Cherry/openbox-3/themerc"
    command -v labwc >/dev/null 2>&1 && labwc --reconfigure >/dev/null 2>&1 || true
fi

# 5. foot Terminal Theme
FOOT_CFG="$HOME/.config/foot/foot.ini"
if [ -f "$FOOT_CFG" ]; then
    BG_NOHASH="${BG#\#}"
    ACC_NOHASH="${ACCENT#\#}"
    sed -i -e "s/^background=.*/background=$BG_NOHASH/" \
           -e "s/^selection-background=.*/selection-background=$ACC_NOHASH/" \
           "$FOOT_CFG" || true
fi

echo "[theme] Applied '$THEME_NAME' ($ACCENT / $ACCENT_HI on $BG) across GTK4, GTK3, Waybar, labwc & foot."
