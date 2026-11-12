# Release Notes — WinScroller 2D v0.1.0

**Release Date:** 2026-11-12  
**Platform:** Windows 10 / Windows 11 (64-bit)  
**Edition:** Rust 2024

## Highlights

- **Infinite 2D Sparse Grid & Niri Cell Stacking**: Place, move, swap, consume (`Win+Comma`), expel (`Win+Period`), and tab/split-stack (`Win+W`) windows across unbounded `(col, row)` coordinates with DWM cloaking for offscreen and inactive stacked windows.
- **Dynamic `Win+O` Overview**: Fit-to-display live DWM thumbnails over Gaussian-blurred wallpaper, frameless previews with bottom-centered app icons, smooth front/back transitions, drag-and-drop grid reordering (`Shift+Drag` to stack), and a spring-animated red top-right `✕` close button active only on highlighted cards.
- **Per-Monitor & Per-Virtual-Desktop Workspaces**: Tracks Windows Virtual Desktop GUIDs (`IVirtualDesktopManager` + Registry notifications) so every monitor and virtual desktop keeps an independent scrolling workspace instance.
- **Persistent Snap Memory**: Saves window coordinates, custom/preset snap proportions, camera positions, and workspace lattice toggles to `%APPDATA%\WinScroller2D\state.json`.
- **Precision Touchpad & Hover Focus**: Raw HID 3-finger continuous 2D panning, 4-finger swipe overview toggle, and debounced hover-focus over edge peeks.
- **Live Settings Overlay (`Win+Alt+S`), Hot-Reload & IPC**: Graphical live-settings editor, automatic `winscroller2d.toml` reload, system tray menu, crash backtrace logger (`crash.log`), and scriptable `winscroller2d msg ...` named-pipe IPC.

## Verification

- `cargo test --workspace`: **22 tests passing** (`6` unit tests in `winscroller2d`, `16` integration/layout tests in `winscroller2d-core`).

## Known Operational Notes

- Run `winscroller2d.exe` elevated if you also want hotkeys/tiling to control UAC-elevated Administrator windows.
- Stop a running `winscroller2d.exe` (`Win+Shift+Escape`) before rebuilding `--release`.
