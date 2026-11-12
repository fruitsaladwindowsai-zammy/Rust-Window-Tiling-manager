# Contributing to WinScroller 2D

Contributions, bug reports, and pull requests are welcome!

---

## Prerequisites

- **Windows 10 or Windows 11** (64-bit) — WinScroller 2D targets Win32 / DWM / HID APIs natively.
- **Rust toolchain 1.85+** (workspace uses **Rust Edition 2024**).
- **Visual Studio Build Tools** with the **Desktop development with C++** / Windows SDK workload.

---

## Workspace Layout

```text
.
├── Cargo.toml                         # Cargo workspace (edition = "2024")
├── winscroller2d.example.toml         # Reference default config template
├── crates/
│   ├── winscroller2d-core/            # Pure window-manager logic (OS-agnostic, unit-tested)
│   │   ├── src/
│   │   │   ├── lib.rs                 # Public API exports
│   │   │   ├── grid.rs                # 2D sparse GridWorkspace, cells, stacking & layout math
│   │   │   ├── manager.rs             # WorkspaceManager (per-monitor/per-desktop, rules, IPC)
│   │   │   ├── config.rs              # TOML Config schema, validation & defaults
│   │   │   ├── animation.rs           # Spring1D & CameraAnim physics
│   │   │   ├── persist.rs             # PersistedState JSON serialization & matching
│   │   │   └── ipc.rs                 # Action, Direction, IpcRequest/IpcResponse types
│   │   └── tests/
│   │       └── layout_tests.rs        # Core layout, stacking, persistence & workspace tests
│   └── winscroller2d/                 # Native Windows daemon, UI & Win32 integration
│       └── src/
│           ├── main.rs                # CLI, panic crash logger, IPC client
│           ├── app.rs                 # Main event loop, state sync, live reload & WinEvents
│           ├── ipc_pipe.rs            # Named-pipe IPC server/client
│           └── win32/
│               ├── mod.rs             # Shared Win32 helpers & VsyncPacer
│               ├── overview.rs        # Win+O DWM thumbnail overview, drag-drop & close badge
│               ├── hotkeys.rs         # WH_KEYBOARD_LL chord parser & Start Menu suppression
│               ├── vdesktop.rs        # COM IVirtualDesktopManager + Explorer registry watcher
│               ├── gestures.rs        # Raw HID Precision Touchpad 3/4-finger tracker
│               ├── hover.rs           # Debounced hover-focus tracker
│               ├── settings_ui.rs     # Interactive Win+Alt+S settings window
│               ├── wallpaper.rs       # Wallpaper capture & Gaussian blur
│               ├── window.rs          # DWM borders, cloaking, move/size batching & enumeration
│               ├── tray.rs            # Notification tray icon & menu
│               ├── events.rs          # WinEvent hook listener
│               └── dpi.rs             # Per-monitor DPI v2 & monitor enumeration
└── docs/                              # mdBook documentation for GitHub Pages
```

---

## Development & Testing Workflow

1. **Fork and clone** the repository, then create a feature branch:
   ```powershell
   git checkout -b feature/my-improvement
   ```
2. **Keep core logic testable**: Place pure layout, persistence, and state-machine logic inside `crates/winscroller2d-core` so it can be unit tested without a live Win32 desktop.
3. **Run checks before committing**:
   ```powershell
   cargo fmt -- --check
   cargo clippy --workspace --all-targets
   cargo test --workspace
   cargo check --release --workspace --tests
   ```
   > If rebuilding `--release` while testing the app locally, close the running `winscroller2d.exe` first (`Win+Shift+Escape`).
4. **Update docs when changing user-facing behavior**: Update `winscroller2d.example.toml`, `KEYBINDINGS.md`, `CONFIGURATION.md`, `docs/src/`, and `CHANGELOG.md` as needed.
5. **Open a Pull Request** describing what changed, why, and how it was verified.

---

## Coding Standards

- Follow `rustfmt` formatting (`cargo fmt`).
- Avoid duplicating logic; keep functions focused and shallow.
- Preserve existing comments and documentation unless directly superseded by your change.
- Validate any config changes against `example_toml_parses_cleanly_and_validates` in `crates/winscroller2d-core/tests/layout_tests.rs`.

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
