# WinScroller 2D

**A native, lightweight 2D scrolling and tiling window manager for Windows 10/11 written in Rust.**

WinScroller 2D merges **Niri-style** scrolling columns and cell stacks with **Hyprscroll2D** infinite 2D grids and an interactive **Win+O dynamic overview** — powered directly by native Win32, DWM, and HID APIs.

## Highlights

- **Infinite 2D Sparse Grid** — Place, swap, and scroll across `(col, row)` coordinates with neighbor edge peeking and five placement strategies.
- **Niri Cell Stacking** — Consume (`Win+Comma`), expel (`Win+Period`), and toggle vertical split vs. tabbed stacks (`Win+W`).
- **Dynamic `Win+O` Overview** — Automatically fits all active windows into the display view with live DWM thumbnails, Gaussian-blurred wallpaper, bottom-centered app icons, smooth front/back depth transitions, drag-and-drop reordering, and a spring-animated red top-right `✕` close button on the highlighted window.
- **Per-Monitor & Per-Desktop Instances** — Separate scrolling workspace state per monitor and per Windows virtual desktop.
- **Persistent Snap Memory** — Saves grid coordinates, custom/preset snap proportions, and workspace layout modes in `state.json`.
- **Precision Touchpad & Hover Focus** — 3-finger continuous 2D panning, 4-finger swipe overview toggle, and debounced hover-focus on peeking edge windows.
- **Live Settings & IPC CLI** — Configure via `%APPDATA%\WinScroller2D\winscroller2d.toml` (hot-reloaded), `Win+Alt+S` Settings overlay, or `winscroller2d msg <command>`.

## Quick Links

| Guide | Description |
|-------|-------------|
| [Getting Started](./getting-started.md) | Build, initialize config, and launch |
| [Keybindings & Gestures](./keyboard-shortcuts.md) | Default hotkeys, overview mouse/key controls, touchpad & tray actions |
| [Overview Mode](./overview-mode.md) | How the dynamic **Win+O** overview and drag-drop work |
| [Configuration](./configuration.md) | Complete `winscroller2d.toml` reference |
| [Release Notes (v0.1.0)](./release-notes.md) | Full v0.1.0 feature summary & upgrade notes |
| [Architecture](./architecture.md) | Workspace crates and Win32 modules |
