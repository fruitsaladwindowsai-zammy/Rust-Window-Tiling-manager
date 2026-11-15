# Architecture & Internals

WinScroller 2D is engineered as a clean two-tier Rust architecture separating pure tiling geometry from native Win32 platform bindings.

---

## 🏗️ Monorepo Structure

```text
Rust-Window-Tiling-manager/
├── Cargo.toml
├── crates/
│   ├── winscroller2d-core/       # Pure Rust layout engine & spring physics
│   │   ├── src/
│   │   │   ├── grid.rs           # 2D sparse grid, cells, stacks, coordinate math
│   │   │   ├── manager.rs        # WorkspaceManager state machine
│   │   │   ├── spring.rs         # Underdamped harmonic oscillator
│   │   │   ├── config.rs         # Configuration parser and schema
│   │   │   └── persist.rs        # State serialization
│   │   └── tests/
│   │       └── layout_tests.rs   # 100% deterministic layout unit tests
│   │
│   └── winscroller2d/            # Win32 platform integration & daemon
│       ├── src/
│       │   ├── main.rs           # CLI, privilege elevation, IPC client
│       │   ├── app.rs            # Main event loop & message pump
│       │   ├── ipc_pipe.rs       # Named pipe server with permissive DACL
│       │   └── win32/
│       │       ├── hotkeys.rs    # WH_KEYBOARD_LL input hook
│       │       ├── window.rs     # Win32 window applier & DWM cloaking
│       │       ├── overview.rs   # Win+O live overview GDI/DWM renderer
│       │       ├── vdesktop.rs   # COM IVirtualDesktopManager interface
│       │       ├── wallpaper.rs  # Acrylic Gaussian blur generator
│       │       └── touchpad.rs   # Raw Input precision touch tracking
└── wiki/                         # Comprehensive documentation
```

---

## 🧩 Architectural Design Principles

### 1. `winscroller2d-core`: Pure Geometry, Zero Win32 Dependencies
* Contains zero OS-specific imports.
* Represents windows strictly by numeric `WindowId (u64)` and integer `IRect (x, y, w, h)`.
* Every layout transition, camera movement, consume/expel stack operation, and split calculation is 100% unit-testable in CI without needing a Windows GUI display session.

### 2. `winscroller2d`: Win32 & DWM Platform Integration
The host executable links native Windows subsystem components:

* **Input Hooking (`WH_KEYBOARD_LL`)**:
  Captures key events ahead of Windows Explorer. Reconciles tracked modifier state with live `GetAsyncKeyState` to prevent stuck modifier flags across UAC prompts.
* **Accessibility Event Listener (`SetWinEventHook`)**:
  Listens for OS window lifecycle events:
  * `EVENT_OBJECT_CREATE` / `EVENT_OBJECT_SHOW`: Adds new windows.
  * `EVENT_OBJECT_DESTROY` / `EVENT_OBJECT_HIDE`: Removes closed windows.
  * `EVENT_SYSTEM_FOREGROUND`: Tracks active focus.
  * `EVENT_SYSTEM_MOVESIZEEND`: Enforces tiling geometry and catches cross-monitor drags.
* **Desktop Window Manager (DWM)**:
  * `DwmSetWindowAttribute(DWMWA_CLOAK)`: Completely halts GPU composition for offscreen windows.
  * `DwmRegisterThumbnail` & `DwmUpdateThumbnailProperties`: Streams live video surfaces for the `Win+O` overview.
* **120 Hz VSync Animation Pacer**:
  A dedicated background pacer thread utilizes high-resolution waitable timers (`CreateWaitableTimerExW`) to deliver smooth 120 FPS step ticks to the main message loop without spinning the CPU.
