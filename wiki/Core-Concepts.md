# Core Concepts

WinScroller 2D redefines window management on Windows by organizing open applications into an **infinite, two-dimensional sparse lattice** rather than rigid desktop tiles or fixed columns.

---

## 🌐 The 2D Infinite Sparse Grid

Traditional window managers on Linux (like i3 or bspwm) constrain windows into binary space partitioning trees, while horizontal scrollers (like Niri or PaperWM) arrange windows in a single 1D strip.

WinScroller 2D expands this into an **unbounded 2D coordinate system** defined by `(col, row)` integer coordinates:

* **Columns (`col`)**: Span horizontally from $(-\infty, +\infty)$. Columns naturally represent distinct projects, workflows, or contexts (e.g., Code in Col 0, Terminal in Col 1, Browser in Col 2).
* **Rows (`row`)**: Span vertically from $(-\infty, +\infty)$. Rows represent related auxiliary windows within a context (e.g., Row -1 for Notes, Row 0 for Main App, Row 1 for Profiler/Debugger).
* **Hole Preservation**: By default, empty cells in the grid are preserved so your spatial mental model never shifts unpredictably. If you prefer automatic compaction, `auto_compact_empty = true` collapses empty rows and columns automatically when windows close.

---

## 🎥 The Virtual Camera & Spring Physics

Instead of snapping windows instantly across the screen with jarring jumps, WinScroller 2D models a **Virtual Camera** that moves smoothly over the infinite canvas.

* **Target Position**: Focus changes update the target camera coordinates in continuous floating-point pixels:
  $$\mathbf{x}_{\text{target}} = \text{Cell Center}_X, \quad \mathbf{y}_{\text{target}} = \text{Cell Center}_Y$$
* **Spring Dynamics**: An underdamped harmonic oscillator calculates camera movement:
  $$F = -k (\mathbf{x} - \mathbf{x}_{\text{target}}) - c \mathbf{v}$$
  where $k$ is spring stiffness (`stiffness`, default `260.0`), and $c = 2\zeta\sqrt{k}$ is derived from the damping ratio (`damping_ratio`, default `0.72`).
* **120 Hz Hardware Pacing**: The animation step loop (`VsyncPacer`) synchronizes directly to DWM VSync via `DwmFlush()` and sleeps at 0% CPU on a `Condvar` when idle.

---

## 🗂️ Cells, Stacks, and Split Views

Every coordinate `(col, row)` holds a **Cell**. A cell can contain:

1. **Single Window**: The window occupies the cell dimensions according to width and height presets.
2. **Horizontal Split View (Snapped Side-by-Side)**:
   * When dragging and dropping a window onto another in **`Win + O` Overview**, or via `split_window_horizontal`, both applications share the cell side-by-side ($50\% / 50\%$ width).
   * They act as a unified dual-app view with infinite scroll between views.
3. **Tabbed Stack**:
   * Windows are stacked on top of one another at the same coordinate.
   * `Win + W` toggles between horizontal split and tabbed modes.
   * In tabbed mode, non-active windows in the stack are automatically DWM-cloaked and switch instantly via **`Win + Left` / `Win + Up`** and **`Win + Right` / `Win + Down`**.

---

## 👻 Peeking vs. DWM Cloaking

To maintain clean aesthetics and optimal system performance:

* **Peeking**: If adjacent cells are within visual distance (configured via `peek_x` and `peek_y` in pixels), their edges peek into the current display viewport. This provides immediate visual context of neighboring workspaces.
* **DWM Cloaking**: Any window completely outside the screen bounds is marked cloaked using native Win32 Desktop Window Manager APIs (`DwmSetWindowAttribute(DWMWA_CLOAK)`):
  * **Zero GPU/CPU Overhead**: Windows does not compose or render cloaked windows.
  * **Zero Click Leaks**: You cannot accidentally click or drag an offscreen window.
  * **Clean Zero-Pixel Borders**: Offscreen windows produce zero leftover pixel slivers or title bar artifacts at the edges of the monitor.

---

## 🖥️ Multi-Monitor & Virtual Desktop Independence

Each physical monitor maintains its own **independent** 2D workspace lattice:

* Panning or navigating on Monitor 1 never shifts or scrolls the camera on Monitor 2.
* When switching Windows 11 Virtual Desktops via Task View (`Win+Tab`) or OS virtual desktop controls, WinScroller 2D switches its active grid state to match the OS virtual desktop via COM `IVirtualDesktopManager` (note: `Win+Ctrl+Left/Right` is bound to camera panning by default and can be customized in `winscroller2d.toml`).
