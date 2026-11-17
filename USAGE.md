# Usage & Installation Guide

## Installation & First-Time Setup

### 1. Prerequisites (Clean Windows 10 / 11 Machine)
Windows does not include Git or Rust by default. In an Administrator PowerShell prompt, install the required toolchains:
```powershell
# 1. Install Git for Windows (Enables 'git clone')
winget install --id Git.Git --accept-package-agreements --accept-source-agreements

# 2. Install Rust toolchain (Rustup)
winget install --id Rustlang.Rustup --accept-package-agreements --accept-source-agreements

# 3. Install MSVC C++ Build Tools (Required to link native Windows Win32/DWM APIs)
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```
*Restart your PowerShell terminal after installation so that `git`, `cargo`, and `rustc` are available on your PATH.*

### 2. Download the Source Code

#### Option A: Clone with Git (Recommended)
```powershell
git clone https://github.com/fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager.git
cd "Rust-Window-Tiling-manager"
```

*If the repository requires GitHub authentication, clone with GitHub CLI or SSH:*
```powershell
# Using GitHub CLI:
winget install --id GitHub.cli
gh auth login
gh repo clone fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager
cd "Rust-Window-Tiling-manager"

# Or using SSH:
git clone git@github.com:fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager.git
cd "Rust-Window-Tiling-manager"
```

#### Option B: Download ZIP (No Git Required)
If you do not have Git installed or cannot use `git clone`, download directly in PowerShell:
```powershell
Invoke-WebRequest -Uri "https://github.com/fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager/archive/refs/heads/main.zip" -OutFile "$HOME\Downloads\winscroller.zip"
Expand-Archive -Path "$HOME\Downloads\winscroller.zip" -DestinationPath "$HOME\Projects"
cd "$HOME\Projects\Rust-Window-Tiling-manager-main"
```

### 3. Build Release Binary
```powershell
cargo build --release
```

### 4. Initialize & Launch
```powershell
# Create default configuration at %APPDATA%\WinScroller2D\winscroller2d.toml
.\target\release\winscroller2d.exe --init-config

# Validate configuration syntax
.\target\release\winscroller2d.exe --validate

# Launch (triggers Windows UAC elevation prompt automatically)
.\target\release\winscroller2d.exe
```

> [!NOTE]
> **Why Administrator Elevation is Needed**:
> WinScroller 2D automatically requests Administrator elevation via UAC. This allows its low-level keyboard hooks (`WH_KEYBOARD_LL`) to navigate across and manage elevated windows (such as Administrator PowerShell and Task Manager) without being silenced by Windows UIPI.

### 5. Running Automatically on Windows Startup
To launch WinScroller 2D automatically upon logging in:
```powershell
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\WinScroller2D.lnk")
$Shortcut.TargetPath = "$PWD\target\release\winscroller2d.exe"
$Shortcut.WorkingDirectory = "$PWD"
$Shortcut.Save()
```

---

## Updating to a New Version

When updating WinScroller 2D, the running executable file is locked by the operating system. You must exit the active instance before compiling:

### Step 1: Gracefully Stop the Active Instance
* Press **`Win + Shift + Escape`**
* OR run via terminal:
  ```powershell
  .\target\release\winscroller2d.exe msg quit
  ```

### Step 2: Pull Latest Changes & Rebuild
```powershell
git pull origin main
cargo build --release
```

### Step 3: Restart with Replacement
```powershell
.\target\release\winscroller2d.exe --replace
```

### Step 4: Refresh Workspace Layout (If Migrating)
```powershell
# Reset all windows to default clean geometry
.\target\release\winscroller2d.exe msg retile
```

---

## Command-Line Interface (CLI)

```text
winscroller2d [OPTIONS]
winscroller2d msg <COMMAND> [ARGS...]
```

### Options

| Flag | Description |
|------|-------------|
| `-c`, `--config <PATH>` | Use a custom `winscroller2d.toml` path instead of `%APPDATA%\WinScroller2D\winscroller2d.toml` |
| `-r`, `--replace` | Replace running instance if already active |
| `--no-elevate` | Run non-elevated (skip automatic Administrator elevation) |
| `--init-config` | Write the default `winscroller2d.toml` (if missing) and exit |
| `--validate` | Parse and normalize the configuration file, print validation result, and exit |
| `-V`, `--version` | Print version (`WinScroller 2D (winscroller2d) 0.1.0`) |
| `-h`, `--help` | Show CLI help |

---

## IPC Control (`winscroller2d msg ...`)

While a WinScroller 2D daemon is running, single-instance enforcement prevents launching a second daemon. Use `winscroller2d msg` over the named pipe to inspect or control the running instance:

| Command | Arguments | Description |
|---------|-----------|-------------|
| `ping` | — | Health check (`pong`) |
| `state` | — | Print JSON snapshot of monitors, workspaces, windows, and coordinates |
| `focus` | `<left\|right\|up\|down>` | Focus adjacent cell in the 2D grid |
| `move` | `<left\|right\|up\|down>` | Move or swap focused window in the 2D grid |
| `pan` | `<left\|right\|up\|down>` | Pan camera by one cell |
| `pan-pixels` | `<dx> <dy>` | Pan camera continuously by pixel deltas |
| `pan-commit` | `<vx> <vy>` | Commit continuous pan with velocity |
| `center` | — | Center camera on focused cell |
| `width` | `<grow\|shrink\|+1\|-1>` | Step focused cell width preset |
| `height` | `<grow\|shrink\|+1\|-1>` | Step focused cell height preset |
| `consume` | `[left\|right\|up\|down]` | Consume neighboring window into focused cell |
| `expel` | `[left\|right\|up\|down]` | Expel active window from focused cell |
| `tabbed` / `toggle-stack` | — | Toggle split vs. tabbed stacking in focused cell |
| `maximize` | — | Toggle cell maximize |
| `fullscreen` | — | Toggle borderless fullscreen |
| `float` | — | Toggle floating mode for focused window |
| `overview` | — | Toggle the **Win+O** overview |
| `close` | — | Close the focused window |
| `workspace` | `<1..N>` | Switch to 1-based workspace index |
| `move-to-workspace` | `<1..N>` | Move focused window to 1-based workspace index |
| `focus-monitor` | `<left\|right\|up\|down>` | Focus monitor in direction |
| `move-to-monitor` | `<left\|right\|up\|down>` | Move focused window to monitor in direction |
| `retile` | — | Retile and resync managed windows |
| `pause` | — | Toggle pause/resume tiling |
| `layout-toggle` | — | Toggle 2D lattice tiling vs. floating for current workspace |
| `settings` | — | Open the Settings window |
| `reload` | — | Reload `winscroller2d.toml` immediately |
| `quit` | — | Gracefully exit and restore managed windows |

---

## Live Settings Window (`Win+Alt+S`)

Press `Win+Alt+S` (or right-click tray icon → **Settings...**) to open the interactive settings overlay. You can click stepper buttons or click numeric fields directly to edit:

- `peek_x`, `peek_y`, `gap_x`/`gap_y`
- `new_window_placement`, `auto_compact_empty`, `auto_expand_focused`
- `stiffness`, `damping_ratio`
- `blur_mode`, `overview_thumbnails`, `overview_min_width`
- `hover_focus_peeks`, `hover_focus_delay_ms`, `center_cursor_on_peek`
- `three_finger_pan`, `persist_state`

Changes apply live immediately and can also be reset to defaults.

---

## System Files & Logs

| File | Location | Purpose |
|------|----------|---------|
| Configuration | `%APPDATA%\WinScroller2D\winscroller2d.toml` | Main TOML configuration (auto-reloaded on save) |
| Persisted State | `<config-dir>\state.json` | Saved window grid coordinates, snap sizes & workspace toggles |
| Crash Log | `%APPDATA%\WinScroller2D\crash.log` | Panic details and stack backtraces |

---

## Keyboard, Mouse & Touchpad Controls

See **[KEYBINDINGS.md](KEYBINDINGS.md)** for the full shortcuts, overview controls, and touchpad gesture table.
