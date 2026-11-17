# Configuration Reference (`winscroller2d.toml`)

WinScroller 2D reads its configuration from `%APPDATA%\WinScroller2D\winscroller2d.toml` (or a custom file via `--config <PATH>`). Saving changes to the file triggers an **automatic live reload**.

Generate a starter configuration at any time:

```powershell
winscroller2d.exe --init-config
```

And validate syntax and value ranges without starting the daemon:

```powershell
winscroller2d.exe --validate
```

You can also view the complete commented template in [`winscroller2d.example.toml`](winscroller2d.example.toml).

---

## `[layout]`

| Key | Type | Default | Valid Range / Options | Description |
|-----|------|---------|-----------------------|-------------|
| `peek_x` | `i32` | `0` | `>= 0` | Horizontal outer margin / edge peek (px) around cells |
| `peek_y` | `i32` | `0` | `>= 0` | Vertical outer margin / edge peek (px) around cells |
| `auto_expand_focused` | `bool` | `false` | `true \| false` | Expand focused cell toward full viewport and shrink unfocused neighbors back to presets |
| `focused_peek_x` | `i32` | `0` | `>= 0` | Horizontal edge margin when `auto_expand_focused = true` |
| `focused_peek_y` | `i32` | `0` | `>= 0` | Vertical edge margin when `auto_expand_focused = true` |
| `gap_x` | `i32` | `8` | `>= 0` | Horizontal gap between adjacent grid cells (px) |
| `gap_y` | `i32` | `8` | `>= 0` | Vertical gap between adjacent grid cells (px) |
| `stack_gap` | `i32` | `8` | `>= 0` | Gap between stacked windows inside the same cell (px) |
| `width_steps` | `[f64]` | `[0.50, 0.67, 0.85, 1.00]` | non-empty, each `(0.05, 2.0]` | Width presets as fractions of `(viewport_w - 2 * peek_x)` |
| `height_steps` | `[f64]` | `[0.50, 0.67, 0.85, 1.00]` | non-empty, each `(0.05, 2.0]` | Height presets as fractions of `(viewport_h - 2 * peek_y)` |
| `default_width_step` | `usize` | `3` | `0..width_steps.len()` | 0-based default index into `width_steps` |
| `default_height_step` | `usize` | `3` | `0..height_steps.len()` | 0-based default index into `height_steps` |
| `new_window_placement` | `string` | `"right_of_focus"` | `"right_of_focus"`, `"below_focus"`, `"rightmost_in_row"`, `"stack_in_current_cell"`, `"neighbor_4way"` | Where newly opened tiled windows are inserted |
| `focus_centering` | `string` | `"always"` | `"always"`, `"on_overflow"` | Center camera on every focus change, or scroll minimally on overflow |
| `expand_single_window` | `bool` | `false` | `true \| false` | Expand a lone tiled window on a workspace to fill viewport minus `single_window_margin` |
| `single_window_margin` | `i32` | `8` (`12` in example) | `>= 0` | Outer padding when `expand_single_window = true` |
| `preserve_holes` | `bool` | `true` | `true \| false` | Preserve empty 2D coordinates when moving windows |
| `auto_compact_empty` | `bool` | `false` | `true \| false` | Automatically collapse empty rows/columns when windows move or close |
| `clip_cross_monitor_edges` | `bool` | `true` | `true \| false` | Prevent peeking windows from bleeding onto adjacent monitors |
| `overview_thumbnails` | `bool` | `true` | `true \| false` | Render live DWM thumbnails in **Win+O** (`false` resizes actual windows as fallback) |
| `overview_min_width` | `f64` | `0.25` | `0.05..=0.60` | Minimum overview card width ratio |
| `overview_min_height` | `f64` | `0.25` | `0.05..=0.60` | Minimum overview card height ratio |
| `overview_autoscroll_speed` | `f64` | `900.0` | `0.0..=5000.0` | Edge autoscroll speed setting (px/sec, `0` disables) |
| `hover_focus_peeks` | `bool` | `true` | `true \| false` | Auto-focus edge-peeking windows when resting the cursor over them |
| `hover_focus_delay_ms` | `u64` | `150` | `0..=2000` | Dwell delay (ms) before hover-focus switches window (`0` = immediate) |
| `center_cursor_on_peek` | `bool` | `false` | `true \| false` | Center cursor inside target window on hover-peek or overview selection |
| `three_finger_pan` | `bool` | `true` | `true \| false` | Enable 1:1 continuous 2D 3-finger Precision Touchpad panning |
| `four_finger_overview` | `bool` | `true` | `true \| false` | Enable 4-finger swipe up/down to open/close `Win+O` overview |
| `gesture_pan_sensitivity` | `f64` | `1.0` | `0.2..=5.0` | Touchpad pan sensitivity multiplier |
| `gesture_invert_x` | `bool` | `false` | `true \| false` | Invert horizontal 3-finger touchpad pan direction |
| `gesture_invert_y` | `bool` | `false` | `true \| false` | Invert vertical 3-finger touchpad pan direction |
| `persist_state` | `bool` | `true` | `true \| false` | Save grid coordinates, snap sizes, and workspace states in `state.json` |
| `restore_timeout_secs` | `u64` | `45` | `5..=300` | Startup window-matching window (seconds) for restoring saved windows |

---

## `[animation]`

| Key | Type | Default | Valid Range | Description |
|-----|------|---------|-------------|-------------|
| `enabled` | `bool` | `true` | `true \| false` | Enable smooth camera and window move animations |
| `animate_resize` | `bool` | `true` | `true \| false` | Smoothly animate window size transitions |
| `stiffness` | `f64` | `260.0` | `40.0..=1500.0` | Spring stiffness (higher = snappier) |
| `damping_ratio` | `f64` | `0.72` | `0.45..=2.0` | Spring damping ratio (`< 1.0` underdamped, `1.0` critically damped) |
| `speed` | `f64` | `16.0` | `1.0..=60.0` | Legacy speed multiplier retained for compatibility |
| `fps` | `u32` | `120` | `15..=240` | Target animation refresh rate in Hz |
| `snap_epsilon_px` | `f64` | `0.5` | `> 0.0` | Pixel threshold at which springs settle cleanly onto targets |

---

## `[appearance]`

| Key | Type | Default | Options | Description |
|-----|------|---------|---------|-------------|
| `enable_win11_border` | `bool` | `true` | `true \| false` | Set native Windows 11 `DWMWA_BORDER_COLOR` on tiled windows |
| `active_border_color` | `string` | `"#7AA2F7"` | `#RRGGBB`, `"default"`, `"none"` | Active/focused window border color |
| `inactive_border_color` | `string` | `"#2F334D"` | `#RRGGBB`, `"default"`, `"none"` | Unfocused tiled window border color |
| `blur_mode` | `string` | `"pure_blur"` | `"pure_blur"`, `"acrylic_clear"`, `"none"` | **Win+O** wallpaper blur style (`pure_blur` = Gaussian blur, `acrylic_clear` = blur + frost, `none` = sharp wallpaper) |
| `overview_alpha` | `u8` | `0` | `0..=220` | Subtle light frost alpha when `blur_mode = "acrylic_clear"` |

---

## `[workspaces]`

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `names` | `[string]` | `["1", "2", ..., "9"]` | Workspace names (falls back to `["1"]` if empty) |
| `per_monitor_workspaces` | `bool` | `true` | Give each monitor + Windows virtual desktop its own workspace set |
| `dynamic_empty_workspace` | `bool` | `true` | Dynamically append an empty trailing workspace when the last workspace receives a window |

---

## `[[rules]]` (Window Rules)

Rules match case-insensitive substrings against `exe`, `class`, and/or `title`:

```toml
[[rules]]
exe = "Taskmgr.exe"
float = true

[[rules]]
exe = "ScreenClippingHost.exe"
ignore = true

[[rules]]
class = "#32770" # Standard Win32 Open/Save/Confirm dialogs
float = true

[[rules]]
title = "Picture-in-Picture"
float = true
```

| Field | Type | Description |
|-------|------|-------------|
| `exe` | `Option<string>` | Substring match on executable name (e.g. `"calc.exe"`) |
| `class` | `Option<string>` | Substring match on Win32 class name |
| `title` | `Option<string>` | Substring match on window title |
| `ignore` | `bool` | Completely ignore window (do not manage or tile) |
| `float` | `bool` | Start matching window floating |
| `workspace` | `Option<usize>` | Optional 1-based workspace index to route matching windows into |
| `width_step` | `Option<usize>` | Optional 0-based initial width preset index |
| `height_step` | `Option<usize>` | Optional 0-based initial height preset index |

---

## `[[bindings]]` (Keybindings)

See **[KEYBINDINGS.md](KEYBINDINGS.md)** for the full list of default bindings, key tokens, and `action` variants.
