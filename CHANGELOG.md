# Changelog

All notable changes to **WinScroller 2D** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-12

### Added
- **2D Grid & Tiling Core (`winscroller2d-core`)**:
  - Sparse infinite `(col, row)` grid workspace (`GridWorkspace`) with configurable edge peek, gaps, and discrete width/height preset steps.
  - Five placement strategies (`right_of_focus`, `below_focus`, `rightmost_in_row`, `stack_in_current_cell`, `neighbor_4way`), optional hole preservation (`preserve_holes`), and optional row/column compaction (`auto_compact_empty`).
  - Niri-style per-cell stacking (`Win+Comma` consume, `Win+Period` expel, `Win+W` vertical split vs. tabbed mode) with offscreen/inactive DWM cloaking.
  - Underdamped spring animation system (`CameraAnim`, `Spring1D`) and optional `auto_expand_focused` mode.
  - Persistent snap & workspace memory (`PersistedState` in `state.json`) preserving window coordinates, cell snap presets/custom fractions, camera positions, and workspace lattice toggles across restarts and sibling snaps.
- **Native Win32 Daemon & UI (`winscroller2d`)**:
  - Multi-monitor & Windows virtual-desktop isolation (`VirtualDesktopTracker` via COM `IVirtualDesktopManager` and Registry watcher).
  - Cross-monitor edge clipping (`clip_cross_monitor_edges`) and Windows 11 `DWMWA_BORDER_COLOR` active/inactive window borders.
  - Dynamic **Win+O** overview with live DWM thumbnails, Gaussian wallpaper blur (`pure_blur` / `acrylic_clear`), centered bottom-middle app icons, front/back focus transition animations, drag-and-drop grid reordering (`Shift+Drag` to stack), and a spring-animated red top-right close button (`✕`) active only on the highlighted card.
  - Low-level keyboard hook (`WH_KEYBOARD_LL`) with Start Menu suppression and reserved `Win+L` passthrough.
  - Precision Touchpad raw HID gesture support (3-finger continuous 2D pan + inertia commit, 4-finger swipe up/down for overview).
  - Debounced hover-focus for edge-peeking windows (`HoverFocusTracker`).
  - Live-editable graphical Settings window (`Win+Alt+S`), system tray icon/menu, and automatic `winscroller2d.toml` live reload.
  - CLI (`--config`, `--init-config`, `--validate`, `-V/--version`, `-h/--help`) and named-pipe IPC (`winscroller2d msg <command>`).
- **Documentation & GitHub Setup**:
  - Added `README.md`, `KEYBINDINGS.md`, `RELEASE_NOTES.md`, `USAGE.md`, `CONFIGURATION.md`, `CONTRIBUTING.md`, `LICENSE`, mdBook site (`docs/`), and GitHub Actions workflows (CI & GitHub Pages).
