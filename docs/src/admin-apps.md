# Administrator & Elevated Applications

WinScroller 2D features a purpose-built privilege architecture that allows it to manage and navigate **elevated (Administrator) windows** without falling victim to Windows UIPI blocks or Aero Snap hijacks.

---

## 🔒 The Windows UIPI Challenge

Under Windows User Interface Privilege Isolation (UIPI), Medium Integrity processes (standard user applications) are prohibited from hooking inputs or sending messages to High Integrity processes (Administrator PowerShell, Task Manager, regedit, etc.).

When a standard window manager runs non-elevated:
1. `WH_KEYBOARD_LL` is silently bypassed by the OS when an elevated window is focused.
2. Windows Explorer catches `Win + Arrows` instead, triggering Windows Aero Snap and snapping the admin window into a half-screen or corner quadrant.
3. The user is trapped on the admin window and cannot navigate away.

---

## 🛡️ The WinScroller 2D Solution

1. **Automatic Administrator Elevation**:
   On startup, WinScroller 2D checks its token elevation (`TOKEN_ELEVATION`). If running non-elevated, it automatically relaunches itself elevated via `ShellExecuteExW` with verb `"runas"`. Running at High Integrity Level allows `WH_KEYBOARD_LL` to intercept hotkeys globally across **all** windows on the desktop before Explorer can see them.
2. **Permissive Named Pipe DACL**:
   The IPC named pipe `\\.\pipe\winscroller2d-ipc` is created with a NULL DACL security descriptor, allowing standard user terminals (Medium integrity) to run `winscroller2d msg ...` CLI commands against the elevated daemon without permission issues.
3. **Immediate Frame Enforcement**:
   On `WinEventKind::MoveSizeEnd`, WinScroller 2D immediately enforces the computed tiling frame back onto the window, preventing external events or accidental drags from breaking windows out of the layout.
