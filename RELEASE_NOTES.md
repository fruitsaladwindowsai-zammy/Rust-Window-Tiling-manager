# Release Notes — WinScroller 2D v0.2.0

**Release Date:** 2026-11-20  
**Target Platform:** Windows 10 / Windows 11 (64-bit)  
**Toolchain:** Rust Edition 2024 (`winscroller2d` & `winscroller2d-core`)

WinScroller 2D `v0.2.0` delivers major enhancements to navigation fluidness, multi-monitor boundary handling, `Win+O` live overview fidelity, memory management, and elevated Administrator process focus.

---

## What's New in v0.2.0

### 🚀 Fluid Bounce Animations (Static Window Geometry)
- **Static Window Dimensions**: Window sizes remain static during transitions (`animate_resize = false`), eliminating jarring window resizes during scroll.
- **Fluid Camera Spring Physics**: Camera panning features natural underdamped spring physics with gentle bounce deceleration.

### 🖥️ Multi-Monitor Boundary Enforcement
- **Zero Cross-Monitor Bleed**: Scrolled windows on Monitor 1 are clipped to display work areas and parked offscreen at `(-32000, -32000)` when off-camera, preventing apps from spilling over onto Monitor 2 in pan or `Win + Arrow` mode.

### 🖼️ Win+O Overview Overhaul & Live Preview Fixes
- **No More Blank / Black Cards**: Fixed DWM composition buffer evictions during overview display so Electron, Chrome, Edge, and WPF windows render preview cards reliably.
- **Crisp High-Res Icons for Console Apps**: PowerShell and command prompt windows that report tiny thumbnail sources are automatically rendered with crisp, centered high-res icons and soft drop shadows instead of distorted pixelated thumbnails.
- **Elevated Administrator Application Support**: Elevated windows (Task Manager, elevated PowerShell, Windows Terminal) are properly cataloged, assigned titles, and natively brought to the foreground with keystroke unlock upon selection in `Win+O`.

### ⚡ RAM & Resource Optimization
- Overview wallpaper blur buffers and DIB surfaces are released immediately when `Win+O` closes, maintaining a minimal idle memory footprint (~15–25 MB).

### 🛠️ Silent Background Execution & Auto-Setup
- WinScroller 2D runs silently in the background as a Windows GUI subsystem application without persistent terminal consoles.
- Automatic runtime prerequisite verification and installation.

---

# Release Notes — WinScroller 2D v0.1.0

**Release Date:** 2026-11-12  
**Target Platform:** Windows 10 / Windows 11 (64-bit)  
**Toolchain:** Rust Edition 2024 (`winscroller2d` & `winscroller2d-core`)

WinScroller 2D `v0.1.0` is the initial public release of our native Rust + Win32 infinite 2D scrolling and tiling window manager for Windows, combining **Niri-style** scrolling columns/stacks and **Hyprscroll2D** infinite 2D grid navigation with an interactive **Win+O** visual overview and zero third-party runtime dependencies beyond `windows-sys`, `serde`, `serde_json`, and `toml`.

---

## Highlights & Key Features

### 1. Infinite 2D Sparse Grid & Niri-Style Cell Stacking
- **Sparse 2D Grid Engine (`GridWorkspace`)**: Place, move, and swap windows across unbounded `(col, row)` coordinates with configurable `peek_x`/`peek_y`, `gap_x`/`gap_y`, and `stack_gap`.
- **Flexible Spawn & Compaction Modes**: Supports `right_of_focus`, `below_focus`, `rightmost_in_row`, `stack_in_current_cell`, and `neighbor_4way`, plus `preserve_holes` and optional `auto_compact_empty`.
- **Column/Cell Stacking**: Consume neighboring windows into a cell (`Win+Comma`), expel them (`Win+Period`), and switch between horizontal side-by-side split and tabbed stacking (`Win+W`) with automatic DWM cloaking of inactive tabs and offscreen windows.
- **Discrete Presets & Auto-Expand**: Cycle or step independent width and height presets (`width_steps`, `height_steps`) or enable `auto_expand_focused` to grow the active cell while shrinking unfocused neighbors back to their resting sizes.

### 2. Interactive `Win+O` Dynamic 2D Overview
- **Fit-to-Display Dynamic Scaling**: All active windows scale dynamically to fit cleanly inside the display viewport without scrolling clutter.
- **Live DWM Thumbnails & Clean App Framing**: Renders live DWM thumbnails directly over a Gaussian-blurred desktop wallpaper (`pure_blur` or `acrylic_clear`) with no outer white frame, displaying each app's icon centered at the bottom middle (plus stack indicator dots for multi-window cells).
- **Front/Back Depth Transitions**: Entering `Win+O` smoothly animates the active window backward into the overview; selecting a card or pressing `Enter` animates the highlighted window forward to the front.
- **Highlighted Top-Right Close Button**: Only the currently highlighted card displays a spring-animated red circular close button with a crisp white `✕` in its top-right corner to close apps directly from overview.
- **Drag-and-Drop Reordering & Snapping**: Dragging a card onto another window snaps both side-by-side horizontally (50% / 50%); `Shift+Drag` swaps their positions; `Ctrl+Drag` stacks the window into the target cell.

### 3. Per-Monitor & Per-Windows-Virtual-Desktop Isolation
- **Independent Scroll Instances**: Tracks Windows Explorer virtual desktop GUIDs via COM `IVirtualDesktopManager` and registry notifications (`VirtualDesktopTracker`), maintaining separate scroll/grid workspaces per `(monitor, virtual_desktop)` pair.
- **Cross-Monitor Edge Clipping**: Prevents edge-peeking windows from bleeding onto adjacent displays (`clip_cross_monitor_edges = true`).
- **Dynamic Empty Workspaces**: Automatically manages per-monitor workspaces (`Win+Alt+1..9`, `Win+PageUp`/`Win+PageDown`) and optional dynamic trailing empty workspaces.

### 4. Persistent Snap Memory & State Recovery
- **Session & Snap Persistence (`state.json`)**: Saves window coordinates, manual/preset cell sizes (`width_step`, `height_step`, `custom_width_frac`, `custom_height_frac`, `maximized`), camera coordinates, and per-workspace lattice toggles in `%APPDATA%\WinScroller2D\state.json`.
- **Resilient Layout Memory**: Preserves individual window snap sizes even when sibling windows snap, overview opens/closes, or windows reopen within `restore_timeout_secs`.

### 5. Native Win32 Polish, Physics & Input
- **Underdamped Spring Animation Engine**: Configurable `stiffness`, `damping_ratio`, `fps` (15–240 Hz, default 120 Hz), and `snap_epsilon_px` with `VsyncPacer`.
- **Windows 11 DWM Borders & Hover Focus**: Sets native `DWMWA_BORDER_COLOR` for focused/unfocused tiled windows and supports debounced hover-focus across edge peeks (`hover_focus_peeks`, `hover_focus_delay_ms`).
- **Low-Level Keyboard Hook (`WH_KEYBOARD_LL`)**: Cleanly intercepts `Win+*` shortcuts and suppresses accidental Start Menu popups while leaving `Win+L` and unbound system shortcuts untouched.
- **Precision Touchpad HID Gestures**: Supports 3-finger continuous 2D panning with inertia commit and 4-finger swipe up/down to open/close the overview.
- **Live Settings Overlay (`Win+Alt+S`) & Hot-Reload**: Adjust key layout, animation, overview, hover, gesture, and persistence settings interactively or edit `winscroller2d.toml` with automatic live reload.
- **Named-Pipe IPC (`winscroller2d msg ...`)**: Enforces single-instance execution and enables external scripting/bars to query `state` JSON or dispatch any tiling command.

---

## Verification & Quality Status

- `cargo test --workspace` passes (**25 tests passed, 0 failed**: 7 binary unit tests in `winscroller2d`, 18 layout/persistence/config/workspace tests in `winscroller2d-core`).
- Includes crash backtrace logging to `%APPDATA%\WinScroller2D\crash.log`.

---

## Installation & Quick Start

```powershell
# Build release binary (Rust 1.85+ / Edition 2024 + MSVC Build Tools)
cargo build --release

# Optional: generate default config at %APPDATA%\WinScroller2D\winscroller2d.toml
.\target\release\winscroller2d.exe --init-config

# Validate configuration
.\target\release\winscroller2d.exe --validate

# Launch WinScroller 2D
.\target\release\winscroller2d.exe
```

See [KEYBINDINGS.md](KEYBINDINGS.md) for the complete keyboard, mouse, touchpad, and tray reference, and [CONFIGURATION.md](CONFIGURATION.md) for every TOML setting.

---

## Known Notes & Upgrade Tips

- **Elevated Windows (UAC)**: Due to Windows UIPI, managing elevated windows (such as an Administrator terminal) requires running `winscroller2d.exe` elevated as well.
- **Rebuilding While Running**: Stop the active `winscroller2d.exe` process (`Win+Shift+Escape` or tray **Exit & Restore Windows**) before running `cargo build --release` so Windows can overwrite the binary.
- **Configuration Path**: Config and state files live under `%APPDATA%\WinScroller2D\` (`winscroller2d.toml`, `state.json`, `crash.log`) unless overridden with `--config <PATH>`.
