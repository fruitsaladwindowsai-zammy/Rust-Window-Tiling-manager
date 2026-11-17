# Configuration

WinScroller 2D loads `%APPDATA%\WinScroller2D\winscroller2d.toml` by default (override with `-c` / `--config <PATH>`) and **automatically hot-reloads** when the file is saved.

```powershell
winscroller2d.exe --init-config   # Create default winscroller2d.toml
winscroller2d.exe --validate      # Validate syntax and constraints
```

## `[layout]`

| Key | Default | Range / Values | Purpose |
|-----|---------|----------------|---------|
| `peek_x`, `peek_y` | `0`, `0` | `>= 0` | Edge peek padding (px) |
| `auto_expand_focused` | `false` | `bool` | Expand focused cell and shrink unfocused neighbors |
| `focused_peek_x`, `focused_peek_y` | `0`, `0` | `>= 0` | Margin kept when `auto_expand_focused = true` |
| `gap_x`, `gap_y`, `stack_gap` | `8`, `8`, `8` | `>= 0` | Cell gaps and horizontal split stack gap (px) |
| `width_steps`, `height_steps` | `[0.50, 0.67, 0.85, 1.00]` | `(0.05, 2.0]` | Discrete cell fraction presets |
| `default_width_step`, `default_height_step` | `3`, `3` | valid indices | Default preset index (`1.00`) |
| `new_window_placement` | `"right_of_focus"` | `"right_of_focus"`, `"below_focus"`, `"rightmost_in_row"`, `"stack_in_current_cell"`, `"neighbor_4way"` | New window insertion policy |
| `focus_centering` | `"always"` | `"always"`, `"on_overflow"` | Camera centering behavior |
| `expand_single_window` | `false` | `bool` | Fill viewport when workspace has 1 tiled window |
| `single_window_margin` | `8` (`12` in example) | `>= 0` | Outer margin for single-window expansion |
| `preserve_holes` | `true` | `bool` | Keep empty 2D coordinates when moving windows |
| `auto_compact_empty` | `false` | `bool` | Compact empty rows/columns on move or close |
| `clip_cross_monitor_edges` | `true` | `bool` | Prevent edge peeks bleeding across monitors |
| `overview_thumbnails` | `true` | `bool` | Use live DWM thumbnails in **Win+O** |
| `overview_min_width`, `overview_min_height` | `0.25`, `0.25` | `0.05..=0.60` | Minimum overview card scale bounds |
| `overview_autoscroll_speed` | `900.0` | `0.0..=5000.0` | Drag autoscroll speed (px/sec) |
| `hover_focus_peeks` | `true` | `bool` | Focus peeking windows on cursor dwell |
| `hover_focus_delay_ms` | `150` | `0..=2000` | Dwell delay (ms) before hover-focus triggers |
| `center_cursor_on_peek` | `false` | `bool` | Center cursor inside focused window on hover/overview jump |
| `three_finger_pan` | `true` | `bool` | Enable 3-finger Precision Touchpad 2D panning |
| `four_finger_overview` | `true` | `bool` | Enable 4-finger swipe up/down overview toggle |
| `gesture_pan_sensitivity` | `1.0` | `0.2..=5.0` | Touchpad pan sensitivity multiplier |
| `gesture_invert_x`, `gesture_invert_y` | `false`, `false` | `bool` | Invert horizontal / vertical 3-finger pan direction |
| `persist_state` | `true` | `bool` | Save window coordinates & snap state in `state.json` |
| `restore_timeout_secs` | `45` | `5..=300` | Startup matching window for persisted windows |

## `[animation]`

| Key | Default | Range | Purpose |
|-----|---------|-------|---------|
| `enabled` | `true` | `bool` | Animate camera & moves |
| `animate_resize` | `true` | `bool` | Animate resize transitions |
| `stiffness` | `260.0` | `40.0..=1500.0` | Spring stiffness |
| `damping_ratio` | `0.72` | `0.45..=2.0` | Spring damping ratio |
| `fps` | `120` | `15..=240` | Active animation refresh rate |
| `snap_epsilon_px` | `0.5` | `> 0.0` | Settle threshold in pixels |

## `[appearance]`

| Key | Default | Options | Purpose |
|-----|---------|---------|---------|
| `enable_win11_border` | `true` | `bool` | Native `DWMWA_BORDER_COLOR` on Windows 11 |
| `active_border_color` | `"#7AA2F7"` | `#RRGGBB`, `"default"`, `"none"` | Focused border color |
| `inactive_border_color` | `"#2F334D"` | `#RRGGBB`, `"default"`, `"none"` | Unfocused border color |
| `blur_mode` | `"pure_blur"` | `"pure_blur"`, `"acrylic_clear"`, `"none"` | **Win+O** wallpaper blur style |
| `overview_alpha` | `0` | `0..=220` | Light frost alpha for `acrylic_clear` |

## `[workspaces]`, `[[rules]]`, `[[bindings]]`

```toml
[workspaces]
names = ["1", "2", "3", "4", "5", "6", "7", "8", "9"]
per_monitor_workspaces = true
dynamic_empty_workspace = true

[[rules]]
exe = "Taskmgr.exe"
float = true

[[bindings]]
keys = "Win+O"
action = "toggle_overview"
```

Rule fields: `exe`, `class`, `title`, `ignore`, `float`, `workspace` (1-based), `width_step` (0-based), `height_step` (0-based).
