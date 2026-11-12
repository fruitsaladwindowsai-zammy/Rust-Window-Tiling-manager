# Usage Guide

## Running WinScroller 2D

```powershell
# Build release binary
cargo build --release

# Launch (run as Administrator if you also need to tile/intercept elevated UAC windows)
.\target\release\winscroller2d.exe
```

On first startup, WinScroller 2D automatically creates `%APPDATA%\WinScroller2D\winscroller2d.toml` from the default template if it does not already exist, places a tray icon in the Windows notification area, and tiles existing windows on each monitor and Windows virtual desktop.

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
