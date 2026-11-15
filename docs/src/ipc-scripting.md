# CLI & IPC Scripting Reference

WinScroller 2D exposes a high-performance IPC interface over a local Windows Named Pipe (`\\.\pipe\winscroller2d-ipc`). You can query state, automate window layouts, or bind external controllers (stream decks, macros, AutoHotkey) using `winscroller2d msg`.

---

## 💻 Essential CLI Commands

```powershell
# Focus and Move in 2D
winscroller2d msg focus <left|right|up|down>
winscroller2d msg move <left|right|up|down>

# Viewport & Camera
winscroller2d msg pan <left|right|up|down>
winscroller2d msg center

# Window Sizing & Stacking
winscroller2d msg width <grow|shrink>
winscroller2d msg height <grow|shrink>
winscroller2d msg consume <left|right|up|down>
winscroller2d msg expel <left|right|up|down>
winscroller2d msg tabbed

# Window States
winscroller2d msg maximize
winscroller2d msg fullscreen
winscroller2d msg float
winscroller2d msg overview
winscroller2d msg close

# Workspaces & Monitors
winscroller2d msg workspace <1..9>
winscroller2d msg move-to-workspace <1..9>
winscroller2d msg focus-monitor <left|right>
winscroller2d msg move-to-monitor <left|right>

# Diagnostics & System
winscroller2d msg ping
winscroller2d msg state
winscroller2d msg retile
winscroller2d msg reload
winscroller2d msg pause
winscroller2d msg quit
```
