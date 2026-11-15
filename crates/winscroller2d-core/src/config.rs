use crate::ipc::{Action, Direction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NewWindowPlacement {
    /// Place in the first free cell to the right of the focused cell (Niri + Hyprscroll2D default).
    #[default]
    RightOfFocus,
    /// Smart 4-way neighbor check around focus (`right -> down -> left -> up`) before shifting a row.
    Neighbor4Way,
    /// Place in the first free cell below the focused cell.
    BelowFocus,
    /// Place at the rightmost edge of the focused row.
    RightmostInRow,
    /// Stack into the focused cell directly (Niri column stack).
    StackInCurrentCell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FocusCentering {
    /// Always center the camera on the focused cell (Hyprscroll2D / Niri `center-focused-column = always`).
    #[default]
    Always,
    /// Only scroll the camera if the focused cell + edge peek would overflow the viewport (Niri default).
    OnOverflow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutConfig {
    /// Visible pixels kept around unfocused cells from neighboring columns.
    pub peek_x: i32,
    /// Visible pixels kept around unfocused cells from neighboring rows.
    pub peek_y: i32,
    /// Automatically expand the currently focused/highlighted cell to utilize the screen,
    /// shrinking it back to its resting preset when focus moves away.
    pub auto_expand_focused: bool,
    /// Edge peek margin kept when `auto_expand_focused` expands the focused cell
    /// (e.g. `24` fills ~97% of the screen while keeping a 12px neighbor peek past `gap = 12`;
    /// set equal to `gap_x` for edge-to-edge fill with outer gap only).
    pub focused_peek_x: i32,
    /// Vertical edge peek margin kept when `auto_expand_focused` expands the focused cell.
    pub focused_peek_y: i32,
    /// Horizontal gap between adjacent grid cells in pixels.
    pub gap_x: i32,
    /// Vertical gap between adjacent grid cells in pixels.
    pub gap_y: i32,
    /// Vertical gap between windows stacked inside the same cell (Niri column mode).
    pub stack_gap: i32,
    /// Discrete width presets as fractions of `(viewport_w - 2 * peek_x)`.
    pub width_steps: Vec<f64>,
    /// Discrete height presets as fractions of `(viewport_h - 2 * peek_y)`.
    pub height_steps: Vec<f64>,
    /// 0-based default index into `width_steps`.
    pub default_width_step: usize,
    /// 0-based default index into `height_steps`.
    pub default_height_step: usize,
    /// Where newly opened tiled windows are inserted.
    pub new_window_placement: NewWindowPlacement,
    /// Whether the camera centers the focused cell or scrolls minimally on overflow.
    pub focus_centering: FocusCentering,
    /// When there is only 1 tiled window in a workspace, expand it to fill viewport minus outer gap (`true`)
    /// or keep its discrete preset size centered (`false`).
    pub expand_single_window: bool,
    /// Keep empty holes when moving a window into an empty 2D coordinate (`true`, Hyprscroll2D style).
    pub preserve_holes: bool,
    /// Automatically compact empty columns and rows when windows move or close (`omalattice` style).
    pub auto_compact_empty: bool,
    /// Clip peeking windows against adjacent monitor edges so they never bleed onto another display.
    pub clip_cross_monitor_edges: bool,
    /// Default outer padding when `expand_single_window = true`.
    pub single_window_margin: i32,
    /// Render live DWM thumbnails in Win+O (`false` uses real window resize fallback).
    pub overview_thumbnails: bool,
    /// Minimum overview card width fraction of viewport (`0.25` keeps large grids readable & scrollable).
    pub overview_min_width: f64,
    /// Minimum overview card height fraction of viewport.
    pub overview_min_height: f64,
    /// Speed (px/sec) for edge autoscroll in a scrollable Win+O overview (`0` disables).
    pub overview_autoscroll_speed: f64,
    /// Focus neighboring peeking windows when resting pointer over them.
    pub hover_focus_peeks: bool,
    /// Milliseconds the cursor must rest over another window before auto-focusing it (`0` = immediate).
    pub hover_focus_delay_ms: u64,
    /// Center cursor inside target window when focus jumps via hover-peek or Win+O click.
    pub center_cursor_on_peek: bool,
    /// Enable 1:1 continuous 2D touchpad panning (3-finger pan / 4-finger overview swipe).
    pub three_finger_pan: bool,
    /// Multiplier for touchpad pan delta sensitivity.
    pub gesture_pan_sensitivity: f64,
    /// Persist workspace grids, sizes, and per-workspace lattice toggles across restarts.
    pub persist_state: bool,
    /// Seconds on startup to wait and match reopened windows to saved grid positions.
    pub restore_timeout_secs: u64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            peek_x: 0,
            peek_y: 0,
            auto_expand_focused: false,
            focused_peek_x: 0,
            focused_peek_y: 0,
            gap_x: 8,
            gap_y: 8,
            stack_gap: 8,
            width_steps: vec![0.50, 0.67, 0.85, 1.00],
            height_steps: vec![0.50, 0.67, 0.85, 1.00],
            default_width_step: 3,  // 1.00 (fills screen minus small gap)
            default_height_step: 3, // 1.00 (fills screen minus small gap)
            new_window_placement: NewWindowPlacement::RightOfFocus,
            focus_centering: FocusCentering::Always,
            expand_single_window: false,
            preserve_holes: true,
            auto_compact_empty: false,
            clip_cross_monitor_edges: true,
            single_window_margin: 8,
            overview_thumbnails: true,
            overview_min_width: 0.25,
            overview_min_height: 0.25,
            overview_autoscroll_speed: 900.0,
            hover_focus_peeks: true,
            hover_focus_delay_ms: 150,
            center_cursor_on_peek: false,
            three_finger_pan: true,
            gesture_pan_sensitivity: 1.0,
            persist_state: true,
            restore_timeout_secs: 45,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnimationConfig {
    /// Smoothly animate scrolling camera and window moves.
    pub enabled: bool,
    /// Smoothly animate window width/height transitions.
    pub animate_resize: bool,
    /// Spring stiffness (higher = snappier, ~240.0 - 360.0 is crisp).
    pub stiffness: f64,
    /// Spring damping ratio (1.0 = critically damped, 0.88-0.95 = subtle spring feel).
    pub damping_ratio: f64,
    /// Legacy speed multiplier (mapped to stiffness if stiffness is unchanged, kept for config compat).
    pub speed: f64,
    /// Target refresh rate in Hz while actively animating.
    pub fps: u32,
    /// Distance in pixels under which animation snaps cleanly to the target.
    pub snap_epsilon_px: f64,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            animate_resize: true,
            stiffness: 260.0,
            damping_ratio: 0.72,
            speed: 16.0,
            fps: 120,
            snap_epsilon_px: 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OverviewBlurMode {
    /// True Gaussian wallpaper blur with zero dark or white veil.
    #[default]
    PureBlur,
    /// True Gaussian wallpaper blur with optional subtle light frost (`overview_alpha`).
    AcrylicClear,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    /// Use Windows 11 native DWMWA_BORDER_COLOR on focused/unfocused tiled windows.
    pub enable_win11_border: bool,
    /// Hex RGB color (`#RRGGBB`) for the focused window border.
    pub active_border_color: String,
    /// Hex RGB color (`#RRGGBB`) for unfocused tiled window borders (or `"default"`).
    pub inactive_border_color: String,
    /// Blur style for Win+O wallpaper overview (`"pure_blur"` | `"acrylic_clear"`).
    pub blur_mode: OverviewBlurMode,
    /// Subtle light frost opacity (`0..=48`) when `blur_mode = "acrylic_clear"` (`0` in `pure_blur`).
    pub overview_alpha: u8,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            enable_win11_border: true,
            active_border_color: "#7AA2F7".to_string(),
            inactive_border_color: "#2F334D".to_string(),
            blur_mode: OverviewBlurMode::PureBlur,
            overview_alpha: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WorkspaceConfig {
    /// Named/numbered workspaces available per monitor or globally.
    pub names: Vec<String>,
    /// Give each monitor its own workspace set (Niri style) vs shared global workspaces.
    pub per_monitor_workspaces: bool,
    /// Automatically append a dynamic empty workspace when the last workspace gets a window (Niri style).
    pub dynamic_empty_workspace: bool,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            names: (1..=9).map(|n| n.to_string()).collect(),
            per_monitor_workspaces: true,
            dynamic_empty_workspace: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindConfig {
    /// Key chord like `"Win+Right"`, `"Win+Shift+Left"`, `"Win+Ctrl+Up"`, `"Win+Equal"`.
    pub keys: String,
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct WindowRule {
    /// Substring match (case-insensitive) on executable name (e.g. `"calc.exe"`).
    pub exe: Option<String>,
    /// Substring match (case-insensitive) on Win32 window class name.
    pub class: Option<String>,
    /// Substring match (case-insensitive) on window title.
    pub title: Option<String>,
    /// Completely ignore this window (do not manage or tile).
    pub ignore: bool,
    /// Start this window floating.
    pub float: bool,
    /// Optional workspace index (1-based) to route matching windows into.
    pub workspace: Option<usize>,
    /// Optional 0-based initial width preset index.
    pub width_step: Option<usize>,
    /// Optional 0-based initial height preset index.
    pub height_step: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub layout: LayoutConfig,
    pub animation: AnimationConfig,
    pub appearance: AppearanceConfig,
    pub workspaces: WorkspaceConfig,
    pub rules: Vec<WindowRule>,
    pub bindings: Vec<KeybindConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            layout: LayoutConfig::default(),
            animation: AnimationConfig::default(),
            appearance: AppearanceConfig::default(),
            workspaces: WorkspaceConfig::default(),
            rules: default_rules(),
            bindings: default_bindings(),
        }
    }
}

impl Config {
    pub fn from_toml_str(src: &str) -> Result<Self, String> {
        let mut cfg: Config = toml::from_str(src).map_err(|e| format!("TOML parse error: {e}"))?;
        cfg.normalize()?;
        Ok(cfg)
    }

    pub fn normalize(&mut self) -> Result<(), String> {
        if self.layout.width_steps.is_empty() {
            return Err("layout.width_steps cannot be empty".into());
        }
        if self.layout.height_steps.is_empty() {
            return Err("layout.height_steps cannot be empty".into());
        }
        for &w in &self.layout.width_steps {
            if !(w > 0.05 && w <= 2.0) {
                return Err(format!("invalid width_step {w}; must be in (0.05, 2.0]"));
            }
        }
        for &h in &self.layout.height_steps {
            if !(h > 0.05 && h <= 2.0) {
                return Err(format!("invalid height_step {h}; must be in (0.05, 2.0]"));
            }
        }
        self.layout.default_width_step = self
            .layout
            .default_width_step
            .min(self.layout.width_steps.len() - 1);
        self.layout.default_height_step = self
            .layout
            .default_height_step
            .min(self.layout.height_steps.len() - 1);
        self.layout.peek_x = self.layout.peek_x.max(0);
        self.layout.peek_y = self.layout.peek_y.max(0);
        self.layout.focused_peek_x = self.layout.focused_peek_x.max(0);
        self.layout.focused_peek_y = self.layout.focused_peek_y.max(0);
        self.layout.gap_x = self.layout.gap_x.max(0);
        self.layout.gap_y = self.layout.gap_y.max(0);
        self.layout.stack_gap = self.layout.stack_gap.max(0);
        self.layout.overview_min_width = self.layout.overview_min_width.clamp(0.05, 0.60);
        self.layout.overview_min_height = self.layout.overview_min_height.clamp(0.05, 0.60);
        self.layout.overview_autoscroll_speed = self.layout.overview_autoscroll_speed.clamp(0.0, 5000.0);
        self.layout.hover_focus_delay_ms = self.layout.hover_focus_delay_ms.min(2000);
        self.layout.gesture_pan_sensitivity = self.layout.gesture_pan_sensitivity.clamp(0.2, 5.0);
        self.layout.restore_timeout_secs = self.layout.restore_timeout_secs.clamp(5, 300);
        self.animation.fps = self.animation.fps.clamp(15, 240);
        self.animation.stiffness = self.animation.stiffness.clamp(40.0, 1500.0);
        self.animation.damping_ratio = self.animation.damping_ratio.clamp(0.45, 2.0);
        self.animation.speed = self.animation.speed.clamp(1.0, 60.0);
        self.appearance.overview_alpha = self.appearance.overview_alpha.min(220);
        if self.workspaces.names.is_empty() {
            self.workspaces.names = vec!["1".to_string()];
        }
        Ok(())
    }

    pub fn to_toml_string(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|e| format!("TOML serialize error: {e}"))
    }

    pub fn default_toml_template() -> &'static str {
        include_str!("../../../winscroller2d.example.toml")
    }
}

pub fn parse_hex_colorref(s: &str) -> Option<u32> {
    let trimmed = s.trim();
    if trimmed.eq_ignore_ascii_case("default") {
        return Some(0xFFFF_FFFF); // DWMWA_COLOR_DEFAULT
    }
    if trimmed.eq_ignore_ascii_case("none") {
        return Some(0xFFFF_FFFE); // DWMWA_COLOR_NONE
    }
    let hex = trimmed.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u32::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u32::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u32::from_str_radix(&hex[4..6], 16).ok()?;
    // Win32 COLORREF is 0x00BBGGRR
    Some((b << 16) | (g << 8) | r)
}

fn default_rules() -> Vec<WindowRule> {
    vec![
        WindowRule {
            exe: Some("ScreenClippingHost.exe".into()),
            ignore: true,
            ..Default::default()
        },
    ]
}

fn bind(keys: &str, action: Action) -> KeybindConfig {
    KeybindConfig {
        keys: keys.to_string(),
        action,
    }
}

fn default_bindings() -> Vec<KeybindConfig> {
    let mut list = vec![
        // Directional 2D Focus (Niri + Hyprscroll2D)
        bind("Win+Left", Action::Focus { direction: Direction::Left }),
        bind("Win+Right", Action::Focus { direction: Direction::Right }),
        bind("Win+Up", Action::Focus { direction: Direction::Up }),
        bind("Win+Down", Action::Focus { direction: Direction::Down }),
        // Directional 2D Move / Swap (preserves holes on empty cells, swaps on occupied)
        bind("Win+Shift+Left", Action::MoveWindow { direction: Direction::Left }),
        bind("Win+Shift+Right", Action::MoveWindow { direction: Direction::Right }),
        bind("Win+Shift+Up", Action::MoveWindow { direction: Direction::Up }),
        bind("Win+Shift+Down", Action::MoveWindow { direction: Direction::Down }),
        // Independent 2D Camera Panning (Hyprscroll2D)
        bind("Win+Ctrl+Left", Action::PanCamera { direction: Direction::Left }),
        bind("Win+Ctrl+Right", Action::PanCamera { direction: Direction::Right }),
        bind("Win+Ctrl+Up", Action::PanCamera { direction: Direction::Up }),
        bind("Win+Ctrl+Down", Action::PanCamera { direction: Direction::Down }),
        bind("Win+C", Action::CenterCamera),
        // Independent Width & Height Presets (Hyprscroll2D + Niri)
        bind("Win+Equal", Action::ResizeWidth { delta: 1 }),
        bind("Win+Minus", Action::ResizeWidth { delta: -1 }),
        bind("Win+Shift+Equal", Action::ResizeHeight { delta: 1 }),
        bind("Win+Shift+Minus", Action::ResizeHeight { delta: -1 }),
        bind("Win+Alt+R", Action::CycleWidthPreset),
        bind("Win+Alt+Shift+R", Action::CycleHeightPreset),
        // Niri Column/Cell Stacking (Consume / Expel / Tabbed)
        bind("Win+Comma", Action::ConsumeIntoCell { direction: Some(Direction::Right) }),
        bind("Win+Period", Action::ExpelFromCell { direction: Some(Direction::Right) }),
        bind("Win+W", Action::ToggleCellStackMode),
        // Cell Modes & Overview
        bind("Win+F", Action::ToggleMaximizeCell),
        bind("Win+Shift+F", Action::ToggleFullscreen),
        bind("Win+Shift+Space", Action::ToggleFloating),
        bind("Win+O", Action::ToggleOverview),
        bind("Win+Shift+Q", Action::CloseFocused),
        // Monitor navigation
        bind("Win+Alt+Left", Action::FocusMonitor { direction: Direction::Left }),
        bind("Win+Alt+Right", Action::FocusMonitor { direction: Direction::Right }),
        bind("Win+Alt+Shift+Left", Action::MoveToMonitor { direction: Direction::Left }),
        bind("Win+Alt+Shift+Right", Action::MoveToMonitor { direction: Direction::Right }),
        // Vertical / Relative Workspace switching (Niri)
        bind("Win+PageUp", Action::SwitchWorkspaceRelative { delta: -1 }),
        bind("Win+PageDown", Action::SwitchWorkspaceRelative { delta: 1 }),
        // Control & safety
        bind("Win+Alt+L", Action::ToggleWorkspaceLayout),
        bind("Win+Alt+S", Action::OpenSettings),
        bind("Win+Shift+P", Action::TogglePause),
        bind("Win+Shift+Escape", Action::Quit),
    ];

    for n in 1..=9 {
        list.push(bind(&format!("Win+Alt+{n}"), Action::SwitchWorkspace { index: n - 1 }));
        list.push(bind(
            &format!("Win+Alt+Shift+{n}"),
            Action::MoveToWorkspace { index: n - 1 },
        ));
    }

    list
}
