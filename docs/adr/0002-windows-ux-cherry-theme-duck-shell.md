# ADR 0002: Windows-Familiar Navigation, Cherry-Black Theme & Duck Taskbar Shell
- Context: Live QEMU testing showed a blank desktop with only a top pill was unintuitive for mouse-first users, and Windows-familiar shortcuts (Alt+Tab, Alt+F4, Ctrl+V in terminal) are needed.
- Decision:
  1. Adopt Cherry Red (#D91E36) + Obsidian Black (#09090B) palette in branding/brand.toml.
  2. Add solid black duck SVG logo (branding/logo.svg).
  3. Switch keymap to Windows-familiar bindings (Alt+Tab, Alt+F4, Super/Win launcher, Super+E/I/L).
  4. Replace minimal pill with a sleek GTK4 taskbar featuring the Duck Start button (animated app burst grid), pinned app icons, live CPU/RAM/GPU/Disk/Net/Battery/Sound telemetry, and a Quick Control Center popover.
  5. Expand forge-settings with dedicated Network, Keyboard, Mouse, Display, and live System Monitor pages.
- Impact on 1.3 budgets: +~12 MiB PSS for taskbar + icon theme; Tier 0 total idle PSS remains ~130-155 MiB (well below 300 MiB budget).
