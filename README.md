# WinScroller 2D

**An infinite 2D scrolling & tiling window manager for Windows 10/11 — bringing Niri-style column stacks, Hyprscroll2D workspaces, and a live `Win+O` overview to keyboard-driven developers and power users.**

![Platform: Windows 10 / 11](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-0078D6?style=flat-square)
![Rust Edition 2024](https://img.shields.io/badge/rust-Edition%202024-orange?style=flat-square)
![License: MIT](https://img.shields.io/badge/license-MIT-green?style=flat-square)
![Architecture: Native Win32 + DWM](https://img.shields.io/badge/backend-Win32%20%2B%20DWM%20%2B%20HID-5C6BC0?style=flat-square)

```text
  ┌─────────────────── 2D Infinite Scrolling Workspace ───────────────────┐
  │                                                                        │
  │   [Peek]   ┌───────────────────────────┐   ┌──────────┐               │
  │   Editor   │   Terminal / IDE (Focus)  │   │ Browser  │  [Peek]       │
  │   (0, 0)   │          (1, 0)           │   │  (2, 0)  │               │
  │            └───────────────────────────┘   └──────────┘               │
  │            ┌───────────────────────────┐                              │
  │            │   Docs / Preview (1, 1)   │  ◄── Scroll smoothly in 2D   │
  │            └───────────────────────────┘      or press Win+O overview │
  └────────────────────────────────────────────────────────────────────────┘
```

> **Tip:** Drop a short screen recording (e.g., `docs/assets/overview-demo.gif`) here after capturing your desktop in **Win+O** mode.



<img width="538" height="228" alt="image" src="https://github.com/user-attachments/assets/3acf5db2-99d1-44a9-a5a5-633bd832bd9d" />

---


<img width="538" height="228" alt="image" src="https://github.com/user-attachments/assets/d003ac2a-175d-4e44-be82-2e8ba727c6bf" />

Showing the CPU and RAM Usage 

## Quick Start (Clean Windows Machine)

### 1. Install Prerequisites *(skip if Rust + C++ Build Tools are already installed)*

```powershell
winget install --id Rustlang.Rustup --accept-package-agreements --accept-source-agreements
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

### 2. Build & Run

```powershell
git clone https://github.com/<your-username>/winscroller2d.git
cd winscroller2d
cargo build --release
.\target\release\winscroller2d.exe --init-config
.\target\release\winscroller2d.exe
```

* First launch creates `%APPDATA%\WinScroller2D\winscroller2d.toml` (auto-reloaded on save) and adds a system tray icon.
* Press **`Win+Alt+S`** for the live graphical Settings window, or **`Win+Shift+Escape`** at any time to exit and restore all windows.
* Before rebuilding `--release`, exit any running `winscroller2d.exe` first (`Win+Shift+Escape`).

---

## Why WinScroller 2D?

- **Infinite 2D Sparse Grid** – Organize windows across unbounded `(col, row)` coordinates with neighbor edge peeking (`peek_x`/`peek_y`), 5 spawn modes, hole preservation, and optional empty row/column compaction.
- **Niri-Style Cell Stacking** – Consume (`Win+Comma`) or expel (`Win+Period`) windows inside a cell and toggle vertical split vs. tabbed stacks (`Win+W`) with DWM cloaking.
- **Dynamic `Win+O` Overview with Drag Snapping** – Fits all active windows into the display view over Gaussian-blurred wallpaper (`pure_blur` / `acrylic_clear`), displays live DWM thumbnails with bottom-centered app icons, animates highlighted windows, supports drag-and-drop cell reordering, and lets you drag and drop any window onto another to snap them horizontally (50% / 50% width) side-by-side into one combined view with infinite scrolling between views.
- **Clean Display Borders & Cloaking** – 0px edge peek margin guarantees that non-visible workspace columns and rows are cleanly cloaked with zero leftover title bar slivers or edge artifacts in screen corners.
- **Clean Exit Centering** – Graceful exit (`Win+Shift+Escape` or system tray exit) cleanly uncloaks and centers all applications in the monitor work area.
- **Dual Screen & Multi-Monitor Extended Mode** – Strict display boundaries with zero spillover across monitors or virtual desktops, smart cross-monitor drag transfer, and hardware edge clipping.
- **Per-Monitor & Per-Virtual-Desktop Instances** – Tracks Windows Virtual Desktops via COM `IVirtualDesktopManager` and keeps an independent scrolling workspace per `(monitor, desktop)` pair.
- **Persistent Snap Memory** – Saves snapped window sizes, coordinates, and per-workspace lattice toggles in `state.json` across sibling snaps, overviews, and restarts.
- **Spring Physics & Native Inputs** – Hardware-paced `120 Hz` underdamped springs, Windows 11 `DWMWA_BORDER_COLOR`, debounced hover-focus on edge peeks, Precision Touchpad 3-finger pan / 4-finger overview swipe, `WH_KEYBOARD_LL` Start-Menu suppression, and `winscroller2d msg` named-pipe IPC.

---

## Essential Keybindings

| Shortcut | Action |
|----------|--------|
| `Win+Left/Right/Up/Down` | Focus adjacent cell in the 2D grid |
| `Win+Shift+Left/Right/Up/Down` | Move / swap focused window in the 2D grid |
| `Win+Ctrl+Left/Right/Up/Down` | Pan camera independently of focus (`Win+C` centers) |
| `Win+Equal` / `Win+Minus` | Step cell width preset (`Win+Shift+Equal/Minus` for height) |
| `Win+Comma` / `Win+Period` / `Win+W` | Consume into cell / Expel / Toggle split-vs-tabbed stack |
| `Win+O` | Toggle dynamic 2D **Overview** (with drag-to-snap) |
| `Win+F` / `Win+Shift+F` / `Win+Shift+Space` | Maximize cell / Fullscreen / Float window |
| `Win+Alt+1..9` / `Win+PageUp/PageDown` | Switch workspaces (`Win+Alt+Shift+1..9` moves window) |
| `Win+Alt+Left/Right` | Focus monitor (`Win+Alt+Shift+Left/Right` moves window) |
| `Win+Alt+S` / `Win+Alt+L` | Open Settings UI / Toggle workspace lattice mode |
| `Win+Shift+P` / `Win+Shift+Escape` | Pause-Resume tiling / Exit & restore windows |

Complete keyboard, overview mouse/drag, Precision Touchpad, and tray reference: **[KEYBINDINGS.md](KEYBINDINGS.md)**.

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
