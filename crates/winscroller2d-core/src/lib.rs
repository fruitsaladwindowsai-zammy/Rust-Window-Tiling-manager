pub mod animation;
pub mod config;
pub mod grid;
pub mod ipc;
pub mod manager;
pub mod persist;

pub use animation::{CameraAnim, Spring1D};
pub use config::{Config, FocusCentering, LayoutConfig, NewWindowPlacement, OverviewBlurMode};
pub use grid::{CellSnap, Coord, GridWorkspace, IRect, StackMode, WindowId, WindowPlacement};
pub use ipc::{Action, Direction, IpcRequest, IpcResponse, StateSnapshot, WorkspaceSnapshot};
pub use manager::{DispatchEffects, FramePlan, MonitorInfo, WindowCommand, WindowInfo, WorkspaceManager};
pub use persist::{PersistedState, PersistedWindow, PersistedWorkspace};
