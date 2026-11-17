use serde::{Deserialize, Serialize};

/// Cardinal directions in the 2D scrolling grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub fn delta(self) -> (i32, i32) {
        match self {
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "left" | "l" | "west" => Some(Direction::Left),
            "right" | "r" | "east" => Some(Direction::Right),
            "up" | "u" | "north" => Some(Direction::Up),
            "down" | "d" | "south" => Some(Direction::Down),
            _ => None,
        }
    }
}

/// Actions dispatched by keybindings, IPC commands, or tray menu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    Focus {
        direction: Direction,
    },
    MoveWindow {
        direction: Direction,
    },
    PanCamera {
        direction: Direction,
    },
    PanPixels {
        dx: f64,
        dy: f64,
    },
    PanCommit {
        vx: f64,
        vy: f64,
    },
    CenterCamera,
    ResizeWidth {
        delta: i32,
    },
    ResizeHeight {
        delta: i32,
    },
    CycleWidthPreset,
    CycleHeightPreset,
    ConsumeIntoCell {
        direction: Option<Direction>,
    },
    ExpelFromCell {
        direction: Option<Direction>,
    },
    ToggleCellStackMode,
    ToggleMaximizeCell,
    ToggleFullscreen,
    ToggleFloating,
    CloseFocused,
    SwitchWorkspace {
        index: usize,
    },
    SwitchWorkspaceRelative {
        delta: i32,
    },
    MoveToWorkspace {
        index: usize,
    },
    FocusMonitor {
        direction: Direction,
    },
    MoveToMonitor {
        direction: Direction,
    },
    JumpToCell {
        col: i32,
        row: i32,
        window_id: Option<u64>,
    },
    PlaceWindowAt {
        window_id: u64,
        col: i32,
        row: i32,
        stack: bool,
    },
    SplitWindowHorizontal {
        window_id: u64,
        target_window_id: u64,
        on_left: bool,
    },
    ToggleOverview,
    TogglePause,
    ToggleWorkspaceLayout,
    OpenSettings,
    Retile,
    ReloadConfig,
    Quit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcRequest {
    Run { action: Action },
    GetState,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSnapshot {
    pub id: u64,
    pub title: String,
    pub process_name: String,
    pub col: i32,
    pub row: i32,
    pub stack_index: usize,
    pub stack_len: usize,
    pub width_step: usize,
    pub height_step: usize,
    pub floating: bool,
    pub fullscreen: bool,
    pub focused: bool,
    pub cloaked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub index: usize,
    pub name: String,
    #[serde(default)]
    pub desktop_id: String,
    #[serde(default)]
    pub monitor_id: Option<u64>,
    pub layout_disabled: bool,
    pub camera_col: i32,
    pub camera_row: i32,
    pub camera_px: (f64, f64),
    pub window_count: usize,
    pub windows: Vec<WindowSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorSnapshot {
    pub id: u64,
    pub name: String,
    pub bounds: (i32, i32, i32, i32),
    pub work_area: (i32, i32, i32, i32),
    pub active_workspace: usize,
    pub is_focused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub paused: bool,
    pub overview_open: bool,
    pub focused_window_id: Option<u64>,
    #[serde(default)]
    pub active_desktop_id: String,
    #[serde(default)]
    pub desktop_ids: Vec<String>,
    pub monitors: Vec<MonitorSnapshot>,
    pub workspaces: Vec<WorkspaceSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum IpcResponse {
    Ok { message: Option<String> },
    Pong,
    State { state: StateSnapshot },
    Error { message: String },
}
