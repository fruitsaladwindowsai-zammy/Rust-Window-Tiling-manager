# Configuration Guide

WinScroller 2D is configured via a single TOML file located at:

```text
%APPDATA%\WinScroller2D\winscroller2d.toml
```

> [!TIP]
> **Live Auto-Reload**: WinScroller 2D monitors this file via a filesystem watcher. Any changes saved to `winscroller2d.toml` are validated and applied instantly without needing to restart the daemon!

---

## 📄 Complete Configuration Schema

### 1. `[layout]` — Grid Dimensions, Gestures & Spacing

```toml
[layout]
# Clean display borders: 0px edge peek so offscreen windows cloak with zero corner slivers
peek_x = 0
peek_y = 0

# Auto-expand focused window to full size when selected
auto_expand_focused = false
focused_peek_x = 0
focused_peek_y = 0

# Gap in pixels between adjacent grid cells and horizontally split windows
gap_x = 8
gap_y = 8
stack_gap = 8

# Width fraction presets stepped via Win+Equal and Win+Minus
width_steps = [0.50, 0.67, 0.85, 1.00]
default_width_step = 3   # 0-based index into width_steps (3 = 1.00 full width)

# Height fraction presets stepped via Win+Shift+Equal and Win+Shift+Minus
height_steps = [0.50, 0.67, 0.85, 1.00]
default_height_step = 3  # 0-based index into height_steps (3 = 1.00 full height)

# Where new tiled windows spawn:
# "right_of_focus" | "below_focus" | "rightmost_in_row" | "stack_in_current_cell" | "neighbor_4way"
new_window_placement = "right_of_focus"

# Camera centering behavior: "always" | "on_overflow"
focus_centering = "always"

# Expand a lone tiled window to fill viewport minus single_window_margin
expand_single_window = false
single_window_margin = 8

# Preserve empty 2D holes on move, or auto-compact empty rows/columns
preserve_holes = true
auto_compact_empty = false
clip_cross_monitor_edges = true

# Win+O Overview thumbnails & bounds
overview_thumbnails = true
overview_min_width = 0.25
overview_min_height = 0.25
overview_autoscroll_speed = 900.0

# Debounced hover-focus on edge-peeking windows
hover_focus_peeks = true
hover_focus_delay_ms = 150
center_cursor_on_peek = false

# Precision Touchpad 3-finger pan & 4-finger overview swipe
three_finger_pan = true
four_finger_overview = true
gesture_pan_sensitivity = 1.0
gesture_invert_x = false
gesture_invert_y = false

# Save lattice layout and window coordinates across restarts in state.json
persist_state = true
restore_timeout_secs = 45
```

### 2. `[animation]` — Spring Physics Dynamics

WinScroller 2D models animations using an underdamped harmonic spring solver (`Spring1D` / `CameraAnim`):

```toml
[animation]
enabled = true
animate_resize = true

# Camera spring stiffness (higher = snappier, lower = softer glide)
# Clamped between 40.0 and 1500.0 (default: 260.0)
stiffness = 260.0

# Damping ratio (clamped between 0.45 and 2.0, default: 0.72):
# < 1.0: Underdamped (subtle natural overshoot and settle)
# = 1.0: Critically damped (no overshoot)
# > 1.0: Overdamped (sluggish glide)
damping_ratio = 0.72

# Target animation refresh rate in Hz (clamped: 15..=240)
fps = 120

# Pixel threshold below which springs snap cleanly to target and sleep at 0% CPU
snap_epsilon_px = 0.5
```

### 3. `[appearance]` — Windows 11 Borders & Overview Wallpaper Blur

```toml
[appearance]
# Use Windows 11 native DWMWA_BORDER_COLOR on focused/unfocused tiled windows
enable_win11_border = true

# Hex RGB color ("#RRGGBB"), "default", or "none"
active_border_color = "#7AA2F7"
inactive_border_color = "#2F334D"

# Overview wallpaper blur style: "pure_blur" | "acrylic_clear" | "none"
blur_mode = "pure_blur"

# Subtle light frost opacity (0..=220, clamped to 48 in shader) for "acrylic_clear"
overview_alpha = 0
```

### 4. `[workspaces]` — Per-Monitor & Dynamic Workspaces

```toml
[workspaces]
names = ["1", "2", "3", "4", "5", "6", "7", "8", "9"]
per_monitor_workspaces = true
dynamic_empty_workspace = true
```

### 5. `[[rules]]` — Per-Application Rules

Filter, float, resize, or ignore specific programs matching case-insensitive substrings of executable name (`exe`), Win32 window class (`class`), or window title (`title`):

```toml
# Ignore screen capture overlays so they aren't tiled
[[rules]]
exe = "ScreenClippingHost.exe"
ignore = true

[[rules]]
exe = "ShareX.exe"
title = "Region capture"
ignore = true

# Automatically float dialogs or utility tools
[[rules]]
exe = "calculatorapp.exe"
float = true

[[rules]]
class = "CabinetWClass"
title = "Properties"
float = true

# Route apps to a specific workspace (1-based) and initial size preset (0-based)
[[rules]]
exe = "Code.exe"
workspace = 1
width_step = 3
```

### 6. `[[bindings]]` — Custom Keybindings

Bind any key chord to a supported `Action` tag:

```toml
[[bindings]]
keys = "Win+Left"
action = "focus"
direction = "left"

[[bindings]]
keys = "Win+Shift+Left"
action = "move_window"
direction = "left"

[[bindings]]
keys = "Win+Ctrl+Left"
action = "pan_camera"
direction = "left"

[[bindings]]
keys = "Win+Shift+R"
action = "retile"
```
