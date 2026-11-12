use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistedWindow {
    pub process_name: String,
    pub class_name: String,
    pub title: String,
    pub col: i32,
    pub row: i32,
    pub width_step: usize,
    pub height_step: usize,
    pub floating: bool,
    #[serde(default)]
    pub maximized: bool,
    #[serde(default)]
    pub manual_override: bool,
    #[serde(default)]
    pub custom_width_frac: Option<f64>,
    #[serde(default)]
    pub custom_height_frac: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistedWorkspace {
    pub key: String,
    pub layout_disabled: bool,
    pub camera_col: i32,
    pub camera_row: i32,
    pub windows: Vec<PersistedWindow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PersistedState {
    pub version: u32,
    pub workspaces: Vec<PersistedWorkspace>,
}

impl PersistedState {
    pub fn from_json_str(src: &str) -> Result<Self, String> {
        serde_json::from_str(src).map_err(|e| format!("state parse error: {e}"))
    }

    pub fn to_json_pretty(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("state serialize error: {e}"))
    }
}

pub struct PendingRestoreWorkspace {
    pub pending: Vec<PersistedWindow>,
    pub camera: (i32, i32),
    pub layout_disabled: bool,
    pub claimed: HashSet<(i32, i32)>,
}

impl PendingRestoreWorkspace {
    pub fn from_persisted(ws: PersistedWorkspace) -> Self {
        Self {
            pending: ws.windows,
            camera: (ws.camera_col, ws.camera_row),
            layout_disabled: ws.layout_disabled,
            claimed: HashSet::new(),
        }
    }

    /// Match a newly appearing OS window by `(process_name, class_name)`, preferring exact `title` match first.
    pub fn take_match(
        &mut self,
        process_name: &str,
        class_name: &str,
        title: &str,
    ) -> Option<PersistedWindow> {
        let p_low = process_name.to_ascii_lowercase();
        let c_low = class_name.to_ascii_lowercase();
        let mut best: Option<usize> = None;
        for (i, w) in self.pending.iter().enumerate() {
            if w.process_name.eq_ignore_ascii_case(&p_low)
                && w.class_name.eq_ignore_ascii_case(&c_low)
            {
                if w.title == title {
                    best = Some(i);
                    break;
                }
                if best.is_none() {
                    best = Some(i);
                }
            }
        }
        best.map(|idx| self.pending.remove(idx))
    }
}
