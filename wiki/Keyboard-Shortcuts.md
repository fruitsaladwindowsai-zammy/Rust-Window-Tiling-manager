# Keyboard Shortcuts Reference

WinScroller 2D uses low-level keyboard hooks (`WH_KEYBOARD_LL`) with Start-Menu suppression so that `Win` key chords trigger instantly without opening the Windows Start Menu or colliding with Windows Explorer Aero Snap.

---

## 🧭 2D Focus & Navigation

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Win + Left` | `focus left` | Focus the cell to the left in the 2D grid. |
| `Win + Right` | `focus right` | Focus the cell to the right in the 2D grid. |
| `Win + Up` | `focus up` | Focus the cell above in the 2D grid. |
| `Win + Down` | `focus down` | Focus the cell below in the 2D grid. |

---

## 📦 Window Movement & Swapping

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Win + Shift + Left` | `move left` | Move focused window left (swaps if occupied, preserves holes if empty). |
| `Win + Shift + Right` | `move right` | Move focused window right (swaps if occupied, preserves holes if empty). |
| `Win + Shift + Up` | `move up` | Move focused window up (swaps if occupied, preserves holes if empty). |
| `Win + Shift + Down` | `move down` | Move focused window down (swaps if occupied, preserves holes if empty). |

---

## 🎥 Independent Camera Panning

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Win + Ctrl + Left` | `pan left` | Pan camera viewport left without changing window focus. |
| `Win + Ctrl + Right` | `pan right` | Pan camera viewport right without changing window focus. |
| `Win + Ctrl + Up` | `pan up` | Pan camera viewport up without changing window focus. |
| `Win + Ctrl + Down` | `pan down` | Pan camera viewport down without changing window focus. |
| `Win + C` | `center` | Re-center camera viewport onto the currently focused cell. |

---

## 📐 Cell Width & Height Preset Resizing

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Win + Equal` (`=`) | `width grow` | Step width to next preset (e.g., $50\% \to 67\% \to 85\% \to 100\%$). |
| `Win + Minus` (`-`) | `width shrink` | Step width to previous preset. |
| `Win + Shift + Equal` | `height grow` | Step height to next vertical preset. |
| `Win + Shift + Minus` | `height shrink` | Step height to previous vertical preset. |
| `Win + Alt + R` | `cycle_width_preset` | Cycle through configured `width_steps`. |
| `Win + Alt + Shift + R` | `cycle_height_preset` | Cycle through configured `height_steps`. |

---

## 🗃️ Stacking & Cell Operations

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Win + Comma` (`,`) | `consume` | Pull neighbor window into the current cell (creates a stack/split). |
| `Win + Period` (`.`) | `expel` | Expel top window out of the current cell into an adjacent empty cell. |
| `Win + W` | `tabbed` | Toggle between horizontal split view and tabbed stack mode. |
| `Win + F` | `maximize` | Toggle maximize cell (expands cell to fill work area). |
| `Win + Shift + F` | `fullscreen` | Toggle true borderless fullscreen for the focused window. |
| `Win + Shift + Space` | `float` | Toggle floating mode (detaches window from tiling grid). |
| `Win + Shift + Q` | `close` | Gracefully close the focused window. |

---

## 🔍 Overview Mode

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Win + O` | `overview` | Toggle the live 2D Overview mode over acrylic blurred wallpaper. |
| `Escape` / `Win + O` | *Close Overview* | Closes Overview and returns camera to active window. |

*(In Overview mode, you can drag and drop any window thumbnail onto another to snap them side-by-side into a single view!)*

---

## 🗂️ Workspaces & Monitors

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Win + Alt + 1` .. `9` | `workspace 1..9` | Switch to workspace 1 through 9 on the active monitor. |
| `Win + Alt + Shift + 1` .. `9` | `move-to-workspace 1..9` | Move focused window to workspace 1 through 9. |
| `Win + PageDown` | `switch_workspace_relative (+1)` | Cycle forward through active workspaces. |
| `Win + PageUp` | `switch_workspace_relative (-1)` | Cycle backward through active workspaces. |
| `Win + Alt + Left` | `focus-monitor left` | Move focus to adjacent left monitor. |
| `Win + Alt + Right` | `focus-monitor right` | Move focus to adjacent right monitor. |
| `Win + Alt + Shift + Left` | `move-to-monitor left` | Move focused window to adjacent left monitor. |
| `Win + Alt + Shift + Right` | `move-to-monitor right` | Move focused window to adjacent right monitor. |

---

## ⚙️ System & Utilities

| Keybinding | Action | Description |
|------------|--------|-------------|
| `Win + Alt + S` | `settings` | Open the graphical Settings configuration UI. |
| `Win + Alt + L` | `layout-toggle` | Toggle workspace layout (scrolling lattice vs. single screen). |
| `Win + Shift + R` | `retile` | Reset all windows to default preset geometry. |
| `Win + Shift + P` | `pause` | Pause/Resume WinScroller 2D window management. |
| `Win + Shift + Escape` | `quit` | Gracefully quit WinScroller 2D, restore all windows, and center them. |
