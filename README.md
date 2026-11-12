# WinScroller 2D

A native, lightweight **2D scrolling and tiling window manager for Windows 10/11** written in Rust.

WinScroller 2D blends **Niri-style scrolling columns & stacks** with **Hyprscroll2D-style infinite 2D grids** and an interactive **Win+O dynamic overview** — built directly on Win32, DWM, and HID APIs with zero external runtime dependencies.

---

## Key Features

- **Infinite 2D Sparse Grid** – Arrange windows across unbounded `(col, row)` coordinates with configurable gaps, neighbor edge peeking (`peek_x`/`peek_y`), and five smart spawn modes.
- **Niri-Style Cell Stacking** – Consume (`Win+Comma`) or expel (`Win+Period`) windows inside a cell and toggle vertical split vs. tabbed stacking (`Win+W`).
- **Dynamic `Win+O` Overview** – Scales all open windows dynamically to fit inside the display view over Gaussian-blurred wallpaper (`pure_blur` / `acrylic_clear`), shows app icons cleanly without extra card clutter, animates the highlighted window to the front/back, supports drag-and-drop reordering (`Shift+Drag` to stack), and reveals a smooth red top-right `✕` close button on the highlighted card.
- **Per-Monitor & Per-Desktop Workspaces** – Tracks Windows Virtual Desktops via `IVirtualDesktopManager` and gives every `(monitor, desktop)` pair its own independent scrolling workspace instance.
- **Persistent Snap Memory** – Saves snapped window proportions, coordinates, and per-workspace lattice toggles in `state.json` so sizes stay intact across sibling snaps, overviews, and restarts.
- **Smooth Spring Animations** – Hardware-paced (`120 Hz` default) underdamped spring physics for camera scrolling, resizing, overview transitions, and close-button badges.
- **Precision Touchpad & Hover Focus** – 3-finger continuous 2D panning, 4-finger swipe overview toggle, and debounced hover-focus on peeking edge windows.
- **Live Settings UI & Hot-Reload Config** – Press `Win+Alt+S` for the graphical settings window or edit `%APPDATA%\WinScroller2D\winscroller2d.toml` with automatic live reload.
- **Scriptable IPC CLI** – Query JSON workspace state or dispatch actions from scripts via `winscroller2d msg <command>`.

---

## Essential Keybindings

| Shortcut | Action |
|----------|--------|
| `Win+Left/Right/Up/Down` | Focus adjacent cell in the 2D grid |
| `Win+Shift+Left/Right/Up/Down` | Move / swap focused window in the 2D grid |
| `Win+Ctrl+Left/Right/Up/Down` | Pan camera independently of focus (`Win+C` centers) |
| `Win+Equal` / `Win+Minus` | Step cell width preset (`Win+Shift+Equal/Minus` for height) |
| `Win+Comma` / `Win+Period` / `Win+W` | Consume into cell / Expel / Toggle split-vs-tabbed stack |
| `Win+O` | Toggle dynamic 2D **Overview** |
| `Win+F` / `Win+Shift+F` / `Win+Shift+Space` | Maximize cell / Fullscreen / Float window |
| `Win+Alt+1..9` / `Win+PageUp/PageDown` | Switch workspaces (`Win+Alt+Shift+1..9` moves window) |
| `Win+Alt+Left/Right` | Focus monitor (`Win+Alt+Shift+Left/Right` moves window) |
| `Win+Alt+S` / `Win+Alt+L` | Open Settings UI / Toggle workspace lattice mode |
| `Win+Shift+P` / `Win+Shift+Escape` | Pause-Resume tiling / Exit & restore windows |

Full keyboard, mouse overview, touchpad gesture, and tray reference: **[KEYBINDINGS.md](KEYBINDINGS.md)**.

---

## Quick Start

### Prerequisites
- **Windows 10 or Windows 11** (64-bit)
- **Rust toolchain (1.85+)** — Workspace uses **Rust Edition 2024**
- **Visual Studio Build Tools** (MSVC C++ workload / Windows SDK)

### Build & Run

```powershell
# Clone and build
git clone https://github.com/<your-username>/winscroller2d.git
cd winscroller2d
cargo build --release

# Generate default config at %APPDATA%\WinScroller2D\winscroller2d.toml
.\target\release\winscroller2d.exe --init-config

# Run the window manager
.\target\release\winscroller2d.exe
```

> **Note:** Stop any running `winscroller2d.exe` (`Win+Shift+Escape` or tray menu **Exit & Restore Windows**) before running `cargo build --release` again.

---

## Documentation

| Document | Contents |
|----------|----------|
| [KEYBINDINGS.md](KEYBINDINGS.md) | Complete shortcuts, overview interactions, touchpad gestures & custom binding syntax |
| [CONFIGURATION.md](CONFIGURATION.md) | Complete `winscroller2d.toml` schema, defaults, clamps & window rules |
| [USAGE.md](USAGE.md) | CLI flags, IPC `msg` commands, tray icon & Settings UI guide |
| [RELEASE_NOTES.md](RELEASE_NOTES.md) | Detailed `v0.1.0` release notes |
| [CHANGELOG.md](CHANGELOG.md) | Version changelog |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Workspace structure, testing (`cargo test --workspace`) & PR guidelines |
| [docs/](docs/README.md) | Source for the mdBook GitHub Pages documentation site |

---

## License

MIT © 2024–2026 WinScroller 2D contributors — see [LICENSE](LICENSE).
