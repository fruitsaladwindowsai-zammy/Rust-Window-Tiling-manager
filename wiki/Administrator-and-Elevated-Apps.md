# Administrator & Elevated Applications

WinScroller 2D features a purpose-built privilege architecture that allows it to manage and navigate **elevated (Administrator) windows** without falling victim to Windows security blocks or Aero Snap hijacks.

---

## 🔒 The Windows UIPI Challenge

On modern Windows (Windows 10 and 11), User Account Control (UAC) enforces **User Interface Privilege Isolation (UIPI)**. Under UIPI, processes run at specific Integrity Levels:

* **Medium Integrity Level**: Standard user applications (VS Code, browsers, standard terminals).
* **High Integrity Level**: Administrator processes (`Administrator: Windows PowerShell`, `Task Manager`, `cmd.exe` (Admin), `regedit.exe`).

### Why Standard Tiling Window Managers Break on Admin Apps:
1. **Hook Silencing**: Windows strictly prohibits a Medium Integrity process from intercepting keyboard or mouse input directed to a High Integrity window. A standard hook (`WH_KEYBOARD_LL`) is **silently bypassed**.
2. **Aero Snap Hijack**: When focus enters an elevated window and the user presses `Win + Left`, `Win + Right`, `Win + Up`, or `Win + Down`:
   * The tiling manager never receives the keystroke.
   * **Windows Explorer** receives the keys and triggers Windows Aero Snap, snapping the admin window into a half-screen or corner quadrant!
   * The user is trapped on the admin window and cannot navigate away.
3. **API Access Denied**: A Medium Integrity process cannot call `SetForegroundWindow`, `BringWindowToTop`, or `SetWindowPos` on an elevated window without `ERROR_ACCESS_DENIED` (Error 5).

---

## 🛡️ The WinScroller 2D Architecture

WinScroller 2D resolves this problem cleanly through three built-in mechanisms:

### 1. Automatic Administrator Elevation
* On startup, WinScroller 2D inspects its token elevation status:
  ```rust
  GetTokenInformation(token, TokenElevation, &mut elevation, ...)
  ```
* If running non-elevated, it automatically relaunches itself elevated via `ShellExecuteExW` with verb `"runas"`.
* In an elevated state (High Integrity Level):
  * `WH_KEYBOARD_LL` receives keystrokes globally across **all** windows on the desktop.
  * `Win + Arrows` are intercepted and suppressed before Windows Explorer can trigger Aero Snap.
  * Calling `SetForegroundWindow` and `SetWindowPos` on admin windows succeeds 100% reliably.

### 2. Permissive IPC Named Pipe DACL
* By default, a named pipe created by an Administrator process is restricted to Admins and System. This would prevent non-elevated user terminals from running CLI commands like `winscroller2d msg state`.
* WinScroller 2D creates `\\.\pipe\winscroller2d-ipc` with a **NULL DACL security descriptor**:
  ```rust
  InitializeSecurityDescriptor(&mut sd, SECURITY_DESCRIPTOR_REVISION);
  SetSecurityDescriptorDacl(&mut sd, 1, std::ptr::null_mut(), 0);
  ```
* This allows **any** user terminal, script, or extension (even running at standard user privileges) to communicate with the elevated daemon smoothly.

### 3. Immediate Frame Enforcement (No Breakouts)
* On `WinEventKind::MoveSizeEnd`: If an external event or accidental mouse drag moves a managed window, WinScroller 2D immediately calls `apply_frame_now(st, ctrl_hwnd, true)` to snap it right back into its computed tiling frame. No application can break out into a floating or quadrant state.

---

## 🕹️ CLI Controls for Elevation & Replacement

| Command | Description |
|---------|-------------|
| `winscroller2d` | Starts the daemon, auto-elevating if needed. |
| `winscroller2d --replace` (`-r`) | Gracefully shuts down any running instance via IPC and starts the new instance. |
| `winscroller2d --no-elevate` | Runs non-elevated (skips elevation for testing or locked environments). |
| `winscroller2d msg <command>` | Sends an IPC command from **any** terminal (standard or admin). |
