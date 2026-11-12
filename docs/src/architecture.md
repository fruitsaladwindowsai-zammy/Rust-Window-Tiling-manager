# Architecture

WinScroller 2D is structured as a Rust Edition 2024 Cargo workspace with two crates:

```text
winscroller2d/
├── Cargo.toml                  # Workspace definition (edition = "2024")
├── winscroller2d.example.toml  # Embedded default config template
└── crates/
    ├── winscroller2d-core/     # OS-agnostic core logic & layout tests
    │   ├── src/
    │   │   ├── lib.rs          # Re-exports core types
    │   │   ├── grid.rs         # GridWorkspace, Cell, CellSnap, WindowPlacement
    │   │   ├── manager.rs      # WorkspaceManager, DispatchEffects, FramePlan
    │   │   ├── config.rs       # Config, LayoutConfig, AnimationConfig, WindowRule
    │   │   ├── animation.rs    # CameraAnim & Spring1D damped springs
    │   │   ├── persist.rs      # PersistedState & PendingRestoreWorkspace
    │   │   └── ipc.rs          # Action, Direction, IpcRequest/IpcResponse
    │   └── tests/layout_tests.rs
    └── winscroller2d/          # Native Win32 daemon, overview & hooks
        └── src/
            ├── main.rs         # CLI, panic hook (crash.log), IPC client
            ├── app.rs          # AppState event loop & state synchronization
            ├── ipc_pipe.rs     # Windows named-pipe IPC
            └── win32/
                ├── mod.rs      # VsyncPacer & custom WM_APP_* messages
                ├── overview.rs # Win+O DWM overview, drag-drop, close_springs
                ├── hotkeys.rs  # WH_KEYBOARD_LL hook & chord parser
                ├── vdesktop.rs # VirtualDesktopTracker (COM + Registry)
                ├── gestures.rs # TouchpadTracker (Raw HID 3/4-finger)
                ├── hover.rs    # HoverFocusTracker
                ├── settings_ui.rs # Live Win+Alt+S settings window
                ├── wallpaper.rs   # Wallpaper blur (pure_blur / acrylic_clear)
                ├── window.rs      # DWM borders, cloaking, WindowApplier
                ├── tray.rs        # TrayIcon & popup menu
                ├── events.rs      # WinEventListener
                └── dpi.rs         # DPI awareness & monitor enumeration
```

## Design Principles

1. **Testability First**: All grid math, snap persistence, placement rules, and spring physics live in `winscroller2d-core` and run under `cargo test` without a live desktop.
2. **Per-Monitor & Per-Desktop Keys**: `WorkspaceManager` indexes workspace sets by `(monitor_id, desktop_id)` so creating or switching Windows virtual desktops maintains independent scrolling state.
3. **Hardware-Paced Springs**: `VsyncPacer` drives `Spring1D` and `CameraAnim` smoothly without spinning CPU when idle.
