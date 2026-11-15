# Multi-Monitor & Virtual Desktops

WinScroller 2D provides enterprise-grade multi-display management and seamless integration with Windows 11 Virtual Desktops.

---

## 🖥️ Multi-Monitor Extended Mode

When Windows is set to **Extend these displays** (`Win + P` > Extend), WinScroller 2D ensures:

```text
 ┌───────────────── Display 1 ─────────────────┐   ┌───────────────── Display 2 ─────────────────┐
 │                                             │   │                                             │
 │   Col 0           Col 1 (Focused)           │   │   Col 0 (Focused)         Col 1             │
 │ ┌───────┐       ┌─────────────────┐         │   │ ┌─────────────────┐       ┌───────┐         │
 │ │ Slack │       │ Visual Studio   │         │   │ │ Microsoft Edge  │       │ Teams │         │
 │ └───────┘       └─────────────────┘         │   │ └─────────────────┘       └───────┘         │
 │                                             │   │                                             │
 └─────────────────────────────────────────────┘   └─────────────────────────────────────────────┘
  ◄── Strict Monitor Boundary (0px spillover) ──►   ◄── Independent Grid & Camera ──────────────►
```

### 1. Strict Boundary Clipping (Zero Spillover)
* Windows tiling managers frequently suffer from "edge spillover" where windows peeking on Display 1 leak across the pixel boundary onto Display 2.
* WinScroller 2D enforces strict hardware coordinate clipping against adjacent monitor bounds (`other_monitors` clipping). Peeking windows are clamped to their monitor's work area, ensuring zero bleeding or cursor clipping onto neighboring screens.

### 2. Independent Camera & Grid State
* Each monitor maintains its own separate camera position `(col, row)` and zoom physics.
* Scrolling horizontally or vertically on Display 1 does **not** affect or scroll Display 2.

### 3. Moving Windows Between Displays
* **Keyboard Shortcut**: Press **`Win + Alt + Shift + Left`** or **`Win + Alt + Shift + Right`** to immediately move the focused application to the adjacent monitor.
* **Mouse Drag**: Dragging a tiled window past the monitor border automatically detects the destination monitor via `MonitorFromWindow`, transfers the window to that display's active workspace, and retiles both screens smoothly.

---

## 🗂️ Windows Virtual Desktops

WinScroller 2D interfaces directly with the native Windows COM `IVirtualDesktopManager` subsystem:

### 1. Per-Desktop Workspace Isolation
* Each `(Monitor, VirtualDesktop)` pair forms an independent workspace.
* Switching OS desktops via three-finger touchpad swipe or **`Win + Ctrl + Left / Right`** triggers native GUID change events.
* WinScroller 2D tracks desktop IDs (`GUID`) and synchronizes window placement without losing camera positions or tiled layouts.

### 2. Numbered Workspaces (1–9)
* Within any monitor, you can also use WinScroller 2D's internal workspaces:
  * **`Win + Alt + 1` .. `9`**: Switch directly to workspace 1 through 9.
  * **`Win + Alt + Shift + 1` .. `9`**: Move the active window to workspace 1 through 9.
  * **`Win + PageUp` / `Win + PageDown`**: Cycle back and forth across workspaces.
