# Window Rules & Exclusions

WinScroller 2D automatically manages standard applications while providing fine-grained rule matching for specialized tools, games, overlays, and dialog boxes.

---

## 🔎 Built-In Manageability Filtering

Before applying custom rules, WinScroller 2D evaluates every window using Win32 Desktop Window Manager heuristics in `is_manageable_window`:

1. **Top-Level Check**: Child windows (`WS_CHILD`) are excluded.
2. **Tool Windows**: Windows with `WS_EX_TOOLWINDOW` are excluded unless they explicitly declare `WS_EX_APPWINDOW`.
3. **UWP Modern Apps**: Windows 11 Universal Windows Platform apps wrapped in `ApplicationFrameWindow` are unbundled and managed normally.
4. **Cloaked Shell Windows**: Windows cloaked by the Windows Shell (`DWM_CLOAKED_SHELL`) because they belong to another virtual desktop are isolated.
5. **System Shell Classes**: The taskbar (`Shell_TrayWnd`), desktop background (`Progman`, `WorkerW`), Alt+Tab frames (`MultitaskingViewFrame`), and IME editors are permanently ignored.
6. **Transparent Overlays**: Screen clipping tools (`ScreenClippingHost.exe`) and capture overlays (`ShareX.exe` "Region capture") are ignored so they don't break screenshots.

---

## ⚙️ Custom Rule Configuration

In `%APPDATA%\WinScroller2D\winscroller2d.toml`, define rules under `[[rules]]`:

### Match Fields
Rules evaluate case-insensitive substring matches against:
* `exe`: The executable filename (e.g. `"notepad.exe"`, `"steam.exe"`).
* `class`: The Win32 window class name (e.g. `"#32770"` for dialogs, `"CabinetWClass"` for File Explorer).
* `title`: The window title bar text (e.g. `"Picture in picture"`, `"Settings"`).

### Action Flags
* `ignore = true`: WinScroller 2D completely ignores the window. Windows handles its placement without tiling.
* `float = true`: The window is tracked by WinScroller 2D but floats freely above the tiling grid *(alias `floating = true` is also accepted)*.
* `workspace = <number>`: Automatically routes the application to a specific workspace (1 through 9) when launched.
* `width_step = <index>` / `height_step = <index>`: Optional 0-based initial width/height preset indices.

---

## 💡 Practical Examples

### 1. Screen Capture & Recording Tools
Ensure screen clipping tools overlay smoothly:

```toml
[[rules]]
exe = "ScreenClippingHost.exe"
ignore = true

[[rules]]
exe = "ShareX.exe"
title = "Region capture"
ignore = true

[[rules]]
exe = "obs64.exe"
title = "Projector"
float = true
```

### 2. Video Players & Picture-in-Picture
Allow mini players to float freely in the corner of your screen:

```toml
[[rules]]
title = "Picture in picture"
float = true

[[rules]]
exe = "vlc.exe"
title = "Adjustments and Effects"
float = true
```

### 3. Gaming Platforms (Steam, Discord, Launchers)
Keep game launchers floating and let main game windows fullscreen naturally:

```toml
[[rules]]
exe = "steam.exe"
title = "Friends List"
float = true

[[rules]]
exe = "Battle.net.exe"
float = true
```

### 4. Routing Development Applications to Workspaces
Pin your editor, terminal, and browser to dedicated numbered workspaces:

```toml
[[rules]]
exe = "Code.exe"
workspace = 1

[[rules]]
exe = "WindowsTerminal.exe"
workspace = 2

[[rules]]
exe = "msedge.exe"
workspace = 3
```
