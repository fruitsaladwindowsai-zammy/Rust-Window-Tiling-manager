# Configuration Guide

WinScroller 2D is configured via a single TOML file located at:

```text
%APPDATA%\WinScroller2D\winscroller2d.toml
```

> [!TIP]
> **Live Auto-Reload**: WinScroller 2D monitors this file via a filesystem watcher. Any changes saved to `winscroller2d.toml` are validated and applied instantly without needing to restart the daemon!

---

## 📄 Complete Configuration Schema

### 1. `[layout]` — Grid Dimensions & Visual Spacing

```toml
[layout]
# Gap in pixels between adjacent windows
gap = 8

# Outer margin in pixels around the entire display work area
margin = 8

# Peeking distance in pixels for neighbor windows entering viewport
peek_x = 0
peek_y = 0

# Width fraction presets stepped via Win+Equal and Win+Minus
width_steps = [0.50, 0.67, 0.85, 1.00]
default_width_step = 3 # Index into width_steps (3 = 1.00 full width)

# Height fraction presets stepped via Win+Shift+Equal and Win+Shift+Minus
height_steps = [0.50, 0.67, 0.85, 1.00]
default_height_step = 3 # Index into height_steps (3 = 1.00 full height)

# Auto-expand focused window to full size when selected
auto_expand_focused = false

# Collapse empty columns and rows when windows close
auto_compact_empty = false

# Render live DWM thumbnails in Win+O overview mode
overview_thumbnails = true

# Wallpaper blur engine in Overview mode: "pure_blur", "acrylic_clear", or "none"
wallpaper_blur_mode = "pure_blur"

# Save lattice layout and window coordinates across restarts
persist_state = true
```

### 2. `[animation]` — Spring Physics Dynamics

WinScroller 2D models animations using an underdamped harmonic spring equation:

```toml
[animation]
# Camera spring stiffness (higher = faster snap, lower = softer glide)
# Clamped between 50.0 and 1200.0 (default: 320.0)
spring_stiffness = 320.0

# Damping ratio:
# < 1.0: Underdamped (subtle natural overshoot and settle)
# = 1.0: Critically damped (no overshoot)
# > 1.0: Overdamped (sluggish glide)
spring_damping = 0.84

# Separate spring physics for Win+O Overview zoom transitions
overview_spring_stiffness = 380.0
overview_spring_damping = 0.88
```

### 3. `[borders]` — Window Highlights & Focus Rings

```toml
[borders]
# Enable colored border highlight around the active window
enabled = true

# Active window border color (Hex: "#RRGGBB" or Win32 COLORREF format)
active_color = "#3B82F6"

# Inactive window border color
inactive_color = "#27272A"

# Border thickness in pixels (Windows 11 DWM native border)
width = 2
```

### 4. `[[rules]]` — Per-Application Rules

Filter, float, or ignore specific programs matching executable name, window class, or title substring:

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
floating = true

[[rules]]
class = "CabinetWClass"
title = "Properties"
floating = true
```

### 5. `[[bindings]]` — Custom Keybindings

Bind any key chord to a supported action:

```toml
[[bindings]]
keys = "Win+Left"
action = "focus"
direction = "left"

[[bindings]]
keys = "Win+Shift+Left"
action = "move"
direction = "left"

[[bindings]]
keys = "Win+Ctrl+Left"
action = "pan"
direction = "left"
```
