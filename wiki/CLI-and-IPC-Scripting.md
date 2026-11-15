# CLI & IPC Scripting Reference

WinScroller 2D exposes a high-performance IPC interface over a local Windows Named Pipe (`\\.\pipe\winscroller2d-ipc`). You can query state, automate window layouts, or bind external controllers (stream decks, macros, AutoHotkey) using `winscroller2d msg`.

---

## 💻 CLI Command Reference

Execute commands directly from PowerShell, CMD, or Windows Terminal:

```powershell
winscroller2d msg <COMMAND> [ARGS...]
```

### Navigation & Focus
```powershell
# Focus adjacent cell
winscroller2d msg focus left
winscroller2d msg focus right
winscroller2d msg focus up
winscroller2d msg focus down

# Move focused window in the 2D grid
winscroller2d msg move left
winscroller2d msg move right
winscroller2d msg move up
winscroller2d msg move down
```

### Camera Panning & Viewport
```powershell
# Pan camera viewport without shifting window focus
winscroller2d msg pan left
winscroller2d msg pan right

# Smooth continuous pixel pan (used by touchpads and scripts)
winscroller2d msg pan-pixels 120.0 0.0

# Commit kinetic pan velocity
winscroller2d msg pan-commit 450.0 0.0

# Re-center camera on the active cell
winscroller2d msg center
```

### Sizing & Stacking
```powershell
# Step cell width preset (grow or shrink)
winscroller2d msg width grow
winscroller2d msg width shrink

# Step cell height preset
winscroller2d msg height grow
winscroller2d msg height shrink

# Pull adjacent neighbor into current cell (create split / stack)
winscroller2d msg consume left
winscroller2d msg consume right

# Expel top window out of cell into neighbor
winscroller2d msg expel left
winscroller2d msg expel right

# Toggle between horizontal split and tabbed stack
winscroller2d msg tabbed
```

### Window State & Overview
```powershell
# Toggle maximize cell
winscroller2d msg maximize

# Toggle borderless fullscreen
winscroller2d msg fullscreen

# Toggle floating mode
winscroller2d msg float

# Toggle live 2D Overview mode
winscroller2d msg overview

# Close active window
winscroller2d msg close
```

### Workspaces & Displays
```powershell
# Switch to workspace 1..9
winscroller2d msg workspace 2

# Move active window to workspace 1..9
winscroller2d msg move-to-workspace 3

# Focus adjacent monitor
winscroller2d msg focus-monitor right

# Move window to adjacent monitor
winscroller2d msg move-to-monitor right
```

### Daemon Control
```powershell
# Health check ping (returns 'pong')
winscroller2d msg ping

# Inspect complete manager state in JSON format
winscroller2d msg state

# Reset all cells to clean default presets
winscroller2d msg retile

# Reload configuration file from disk
winscroller2d msg reload

# Pause or resume window management
winscroller2d msg pause

# Gracefully terminate daemon and restore all windows
winscroller2d msg quit
```

---

## 📜 Scripting Examples

### 1. PowerShell: Query Focused Window ID and Process Name
```powershell
$state = winscroller2d msg state | ConvertFrom-Json
$focusedId = $state.focused_window_id
$currentWs = $state.workspaces | Where-Object { $_.windows.id -contains $focusedId }
$focusedWin = $currentWs.windows | Where-Object { $_.id -eq $focusedId }

Write-Host "Focused: $($focusedWin.title) ($($focusedWin.process_name)) at Col $($focusedWin.col), Row $($focusedWin.row)"
```

### 2. AutoHotkey v2 Integration
```ahk
; Bind special mouse buttons or external macro keys to WinScroller 2D actions
XButton1::Run("winscroller2d.exe msg focus left",, "Hide")
XButton2::Run("winscroller2d.exe msg focus right",, "Hide")
^!o::Run("winscroller2d.exe msg overview",, "Hide")
```
