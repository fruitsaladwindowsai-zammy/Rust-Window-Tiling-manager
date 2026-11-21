use crate::config::{Config, WindowRule};
use crate::grid::{CellSnap, Coord, GridWorkspace, IRect, WindowId, WindowPlacement};
use crate::ipc::{
    Action, Direction, MonitorSnapshot, StateSnapshot, WindowSnapshot, WorkspaceSnapshot,
};
use crate::persist::{
    PendingRestoreWorkspace, PersistedState, PersistedWindow, PersistedWorkspace,
};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub id: u64,
    pub name: String,
    pub work_area: IRect,
    pub bounds: IRect,
}

#[derive(Debug, Clone, Default)]
pub struct WindowInfo {
    pub id: WindowId,
    pub title: String,
    pub process_name: String,
    pub class_name: String,
    pub rect: IRect,
    pub monitor_id: Option<u64>,
    pub desktop_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkspaceScope {
    os_desktop: String,
    monitor_id: Option<u64>,
}

#[derive(Debug, Clone)]
struct ManagedMonitor {
    info: MonitorInfo,
    active_workspace: usize,
    active_by_desktop: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
struct WindowMeta {
    title: String,
    process_name: String,
    class_name: String,
    workspace_id: usize,
    os_desktop: String,
    cloaked_by_wm: bool,
}

#[derive(Debug, Default)]
pub struct DispatchEffects {
    pub focus_window: Option<WindowId>,
    pub close_window: Option<WindowId>,
    pub layout_dirty: bool,
    pub overview_open: Option<bool>,
    pub paused: Option<bool>,
    pub reload_config: bool,
    pub open_settings: bool,
    pub state_dirty: bool,
    pub quit: bool,
    pub moved_to_desktop: Option<(WindowId, String)>,
}

#[derive(Debug, Clone)]
pub struct WindowCommand {
    pub window_id: WindowId,
    pub rect: Option<IRect>,
    pub visible: bool,
    pub clip_to: Option<IRect>,
    pub is_focused: bool,
    pub is_floating: bool,
}

#[derive(Debug, Default)]
pub struct FramePlan {
    pub commands: Vec<WindowCommand>,
    pub still_animating: bool,
}

#[derive(Debug, Default)]
struct MatchedRule {
    ignore: bool,
    float: bool,
    workspace: Option<usize>,
    width_step: Option<usize>,
    height_step: Option<usize>,
}

pub struct WorkspaceManager {
    pub config: Config,
    monitors: Vec<ManagedMonitor>,
    workspaces: Vec<GridWorkspace>,
    /// Ordered Windows OS virtual desktop IDs registered on this session.
    os_desktops: Vec<String>,
    /// Currently active Windows OS virtual desktop ID.
    active_os_desktop: String,
    /// Scope `(os_desktop, monitor_id)` owning each registered workspace.
    ws_scope: HashMap<usize, WorkspaceScope>,
    windows: HashMap<WindowId, WindowMeta>,
    focused_monitor_id: Option<u64>,
    focused_window_id: Option<WindowId>,
    pub paused: bool,
    pub overview_open: bool,
    pub state_dirty: bool,
    restore_pending: HashMap<String, PendingRestoreWorkspace>,
    restore_deadline_epoch: u64,
}

impl WorkspaceManager {
    pub fn new(config: Config, initial_monitors: Vec<MonitorInfo>) -> Self {
        let mut mgr = Self {
            config,
            monitors: Vec::new(),
            workspaces: Vec::new(),
            os_desktops: vec!["default".to_string()],
            active_os_desktop: "default".to_string(),
            ws_scope: HashMap::new(),
            windows: HashMap::new(),
            focused_monitor_id: None,
            focused_window_id: None,
            paused: false,
            overview_open: false,
            state_dirty: false,
            restore_pending: HashMap::new(),
            restore_deadline_epoch: 0,
        };
        mgr.sync_monitors(initial_monitors);
        mgr
    }

    pub fn active_os_desktop(&self) -> &str {
        &self.active_os_desktop
    }

    pub fn window_desktop_id(&self, id: WindowId) -> Option<&str> {
        self.windows.get(&id).map(|m| m.os_desktop.as_str())
    }

    pub fn workspace_persist_key(&self, ws_id: usize) -> Option<String> {
        let ws = self.workspace(ws_id)?;
        let scope = self.ws_scope.get(&ws_id)?;
        let base = if self.config.workspaces.per_monitor_workspaces {
            let mon_id = scope.monitor_id?;
            let mon_name = self
                .monitors
                .iter()
                .find(|m| m.info.id == mon_id)
                .map(|m| m.info.name.as_str())
                .unwrap_or("mon");
            format!("{mon_name}:{}", ws.name)
        } else {
            ws.name.clone()
        };
        if scope.os_desktop == "default" {
            Some(base)
        } else {
            Some(format!("{}@{base}", scope.os_desktop))
        }
    }

    pub fn import_persisted_state(&mut self, state: PersistedState) {
        if !self.config.layout.persist_state {
            return;
        }
        self.restore_deadline_epoch =
            now_epoch_secs().saturating_add(self.config.layout.restore_timeout_secs);
        self.restore_pending.clear();
        for pw in state.workspaces {
            let ws_key = pw.key.clone();
            let layout_disabled = pw.layout_disabled;
            let pending = PendingRestoreWorkspace::from_persisted(pw);
            // Apply saved layout_disabled flag to existing matching workspace if already created.
            for idx in 0..self.workspaces.len() {
                let ws_index = self.workspaces[idx].index;
                if self.workspace_persist_key(ws_index).as_deref() == Some(ws_key.as_str()) {
                    self.workspaces[idx].layout_disabled = layout_disabled;
                }
            }
            if !pending.pending.is_empty() {
                self.restore_pending.insert(ws_key, pending);
            }
        }
    }

    pub fn export_persisted_state(&self) -> PersistedState {
        let mut workspaces = Vec::new();
        for ws in &self.workspaces {
            let Some(key) = self.workspace_persist_key(ws.index) else {
                continue;
            };
            if ws.is_empty() && !ws.layout_disabled {
                continue;
            }
            let mut windows = Vec::new();
            for (&coord, cell) in &ws.cells {
                for &wid in &cell.windows {
                    let meta = self.windows.get(&wid);
                    windows.push(PersistedWindow {
                        process_name: meta.map(|m| m.process_name.clone()).unwrap_or_default(),
                        class_name: meta.map(|m| m.class_name.clone()).unwrap_or_default(),
                        title: meta.map(|m| m.title.clone()).unwrap_or_default(),
                        col: coord.col,
                        row: coord.row,
                        width_step: cell.width_step,
                        height_step: cell.height_step,
                        floating: false,
                        maximized: cell.maximized,
                        manual_override: cell.manual_override,
                        custom_width_frac: cell.custom_width_frac,
                        custom_height_frac: cell.custom_height_frac,
                    });
                }
            }
            for &wid in &ws.floating {
                let meta = self.windows.get(&wid);
                windows.push(PersistedWindow {
                    process_name: meta.map(|m| m.process_name.clone()).unwrap_or_default(),
                    class_name: meta.map(|m| m.class_name.clone()).unwrap_or_default(),
                    title: meta.map(|m| m.title.clone()).unwrap_or_default(),
                    col: 0,
                    row: 0,
                    width_step: 0,
                    height_step: 0,
                    floating: true,
                    maximized: false,
                    manual_override: false,
                    custom_width_frac: None,
                    custom_height_frac: None,
                });
            }
            workspaces.push(PersistedWorkspace {
                key,
                layout_disabled: ws.layout_disabled,
                camera_col: ws.camera_coord.col,
                camera_row: ws.camera_coord.row,
                windows,
            });
        }
        PersistedState {
            version: 1,
            workspaces,
        }
    }

    pub fn active_workspace_layout_disabled(&self) -> bool {
        self.current_workspace_id()
            .and_then(|id| self.workspace(id))
            .is_some_and(|w| w.layout_disabled)
    }

    pub fn update_config(&mut self, config: Config) {
        self.config = config;
        self.ensure_workspaces_for_desktops_and_monitors();
    }

    /// Synchronize Windows OS virtual desktops and guarantee every `(desktop, monitor)` pair
    /// has its own registered scroller workspace instance. Returns `true` if the active
    /// virtual desktop or registered desktop list changed.
    pub fn sync_os_desktops(&mut self, incoming: Vec<String>, active: String) -> bool {
        let active_norm = if active.trim().is_empty() {
            "default".to_string()
        } else {
            active.trim().to_string()
        };
        let mut desktops: Vec<String> = Vec::new();
        for raw in incoming {
            let d = raw.trim();
            if !d.is_empty() && !desktops.iter().any(|x| x == d) {
                desktops.push(d.to_string());
            }
        }
        if !desktops.iter().any(|x| x == &active_norm) {
            desktops.push(active_norm.clone());
        }

        // Preserve any older desktop still holding managed windows until OS reassigns them.
        let mut live_with_windows: Vec<String> = Vec::new();
        for w in &self.workspaces {
            if !w.is_empty() || w.layout_disabled {
                if let Some(scope) = self.ws_scope.get(&w.index) {
                    if !desktops.iter().any(|d| d == &scope.os_desktop)
                        && !live_with_windows.iter().any(|d| d == &scope.os_desktop)
                    {
                        live_with_windows.push(scope.os_desktop.clone());
                    }
                }
            }
        }
        for d in live_with_windows {
            desktops.push(d);
        }

        let changed = self.os_desktops != desktops || self.active_os_desktop != active_norm;
        self.os_desktops = desktops;
        self.active_os_desktop = active_norm;

        // Prune empty placeholder workspaces (e.g. initial "default" or closed OS desktops).
        let valid_desktops = &self.os_desktops;
        let scope_map = &mut self.ws_scope;
        self.workspaces.retain(|w| {
            let keep = !w.is_empty()
                || w.layout_disabled
                || scope_map
                    .get(&w.index)
                    .is_some_and(|s| valid_desktops.iter().any(|d| d == &s.os_desktop));
            if !keep {
                scope_map.remove(&w.index);
            }
            keep
        });
        for mon in &mut self.monitors {
            mon.active_by_desktop
                .retain(|d, _| self.os_desktops.iter().any(|x| x == d));
        }

        self.ensure_workspaces_for_desktops_and_monitors();
        self.sync_monitor_active_workspaces();

        let keep_focus = self.focused_window_id.is_some_and(|fid| {
            self.windows
                .get(&fid)
                .is_some_and(|m| m.os_desktop == self.active_os_desktop)
        });
        if !keep_focus {
            self.focused_window_id = None;
            self.focused_window_id = self
                .current_workspace_id()
                .and_then(|ws_id| self.workspace(ws_id))
                .and_then(|ws| ws.focused);
        }

        changed
    }

    pub fn sync_monitors(&mut self, incoming: Vec<MonitorInfo>) {
        if incoming.is_empty() {
            return;
        }
        let mut prev_state: HashMap<u64, (usize, HashMap<String, usize>)> = self
            .monitors
            .iter()
            .map(|m| (m.info.id, (m.active_workspace, m.active_by_desktop.clone())))
            .collect();

        self.monitors = incoming
            .into_iter()
            .map(|info| {
                let id = info.id;
                let (active_workspace, active_by_desktop) = prev_state
                    .remove(&id)
                    .unwrap_or_else(|| (0, HashMap::new()));
                ManagedMonitor {
                    info,
                    active_workspace,
                    active_by_desktop,
                }
            })
            .collect();

        if self
            .focused_monitor_id
            .is_none_or(|fid| !self.monitors.iter().any(|m| m.info.id == fid))
        {
            self.focused_monitor_id = self.monitors.first().map(|m| m.info.id);
        }

        self.ensure_workspaces_for_desktops_and_monitors();
        self.sync_monitor_active_workspaces();
    }

    fn sync_monitor_active_workspaces(&mut self) {
        let per_mon = self.config.workspaces.per_monitor_workspaces;
        let active_desktop = self.active_os_desktop.clone();
        let desktops = self.os_desktops.clone();

        for mon in &mut self.monitors {
            for desktop in &desktops {
                let candidate = mon.active_by_desktop.get(desktop).copied();
                let valid = candidate.filter(|&ws_id| {
                    self.workspaces.iter().any(|w| {
                        w.index == ws_id
                            && self.ws_scope.get(&w.index).is_some_and(|s| {
                                s.os_desktop == *desktop
                                    && (!per_mon || s.monitor_id == Some(mon.info.id))
                            })
                    })
                });
                if valid.is_none() {
                    if let Some(first) = self.workspaces.iter().find(|w| {
                        self.ws_scope.get(&w.index).is_some_and(|s| {
                            s.os_desktop == *desktop
                                && (!per_mon || s.monitor_id == Some(mon.info.id))
                        })
                    }) {
                        mon.active_by_desktop.insert(desktop.clone(), first.index);
                    }
                }
            }

            if let Some(&active_ws) = mon.active_by_desktop.get(&active_desktop) {
                mon.active_workspace = active_ws;
            }
        }
    }

    fn ensure_workspaces_for_desktops_and_monitors(&mut self) {
        let desktops = self.os_desktops.clone();
        if self.config.workspaces.per_monitor_workspaces {
            let mon_ids: Vec<u64> = self.monitors.iter().map(|m| m.info.id).collect();
            for desktop in &desktops {
                for &mon_id in &mon_ids {
                    let count = self
                        .workspaces
                        .iter()
                        .filter(|w| {
                            self.ws_scope.get(&w.index).is_some_and(|s| {
                                s.os_desktop == *desktop && s.monitor_id == Some(mon_id)
                            })
                        })
                        .count();
                    if count == 0 {
                        let names = self.config.workspaces.names.clone();
                        for name in names {
                            let id = self.next_workspace_id();
                            self.workspaces.push(GridWorkspace::new(id, name));
                            self.ws_scope.insert(
                                id,
                                WorkspaceScope {
                                    os_desktop: desktop.clone(),
                                    monitor_id: Some(mon_id),
                                },
                            );
                        }
                    }
                }
            }
        } else {
            for desktop in &desktops {
                let count = self
                    .workspaces
                    .iter()
                    .filter(|w| {
                        self.ws_scope
                            .get(&w.index)
                            .is_some_and(|s| s.os_desktop == *desktop)
                    })
                    .count();
                if count == 0 {
                    let names = self.config.workspaces.names.clone();
                    for name in names {
                        let id = self.next_workspace_id();
                        self.workspaces.push(GridWorkspace::new(id, name));
                        self.ws_scope.insert(
                            id,
                            WorkspaceScope {
                                os_desktop: desktop.clone(),
                                monitor_id: None,
                            },
                        );
                    }
                }
            }
        }
        if self.config.workspaces.dynamic_empty_workspace {
            self.maintain_dynamic_empty_workspaces();
        }
    }

    fn next_workspace_id(&self) -> usize {
        self.workspaces
            .iter()
            .map(|w| w.index)
            .max()
            .map_or(0, |m| m + 1)
    }

    fn maintain_dynamic_empty_workspaces(&mut self) {
        if !self.config.workspaces.dynamic_empty_workspace {
            return;
        }
        let desktops = self.os_desktops.clone();
        if self.config.workspaces.per_monitor_workspaces {
            let mon_ids: Vec<u64> = self.monitors.iter().map(|m| m.info.id).collect();
            for desktop in &desktops {
                for &mon_id in &mon_ids {
                    let ws_ids: Vec<usize> = self
                        .workspaces
                        .iter()
                        .filter(|w| {
                            self.ws_scope.get(&w.index).is_some_and(|s| {
                                s.os_desktop == *desktop && s.monitor_id == Some(mon_id)
                            })
                        })
                        .map(|w| w.index)
                        .collect();
                    if let Some(&last_id) = ws_ids.last() {
                        let last_empty = self
                            .workspaces
                            .iter()
                            .find(|w| w.index == last_id)
                            .is_some_and(|w| w.is_empty());
                        if !last_empty {
                            let new_id = self.next_workspace_id();
                            let label = (ws_ids.len() + 1).to_string();
                            self.workspaces.push(GridWorkspace::new(new_id, label));
                            self.ws_scope.insert(
                                new_id,
                                WorkspaceScope {
                                    os_desktop: desktop.clone(),
                                    monitor_id: Some(mon_id),
                                },
                            );
                        }
                    }
                }
            }
        } else {
            for desktop in &desktops {
                let ws_ids: Vec<usize> = self
                    .workspaces
                    .iter()
                    .filter(|w| {
                        self.ws_scope
                            .get(&w.index)
                            .is_some_and(|s| s.os_desktop == *desktop)
                    })
                    .map(|w| w.index)
                    .collect();
                if let Some(&last_id) = ws_ids.last() {
                    let last_empty = self
                        .workspaces
                        .iter()
                        .find(|w| w.index == last_id)
                        .is_some_and(|w| w.is_empty());
                    if !last_empty {
                        let new_id = self.next_workspace_id();
                        let label = (ws_ids.len() + 1).to_string();
                        self.workspaces.push(GridWorkspace::new(new_id, label));
                        self.ws_scope.insert(
                            new_id,
                            WorkspaceScope {
                                os_desktop: desktop.clone(),
                                monitor_id: None,
                            },
                        );
                    }
                }
            }
        }
    }

    fn match_rules(&self, info: &WindowInfo) -> MatchedRule {
        let mut matched = MatchedRule::default();
        for rule in &self.config.rules {
            if rule_matches(rule, info) {
                matched.ignore |= rule.ignore;
                matched.float |= rule.float;
                if rule.workspace.is_some() {
                    matched.workspace = rule.workspace;
                }
                if rule.width_step.is_some() {
                    matched.width_step = rule.width_step;
                }
                if rule.height_step.is_some() {
                    matched.height_step = rule.height_step;
                }
            }
        }
        matched
    }

    fn monitor_for_rect(&self, rect: IRect) -> Option<u64> {
        let (cx, cy) = rect.center();
        self.monitors
            .iter()
            .find(|m| {
                cx >= m.info.bounds.x
                    && cx < m.info.bounds.right()
                    && cy >= m.info.bounds.y
                    && cy < m.info.bounds.bottom()
            })
            .or_else(|| self.monitors.first())
            .map(|m| m.info.id)
    }

    pub fn active_workspace_for_monitor(&self, mon_id: u64) -> Option<usize> {
        self.active_workspace_for(&self.active_os_desktop, mon_id)
    }

    fn active_workspace_for(&self, os_desktop: &str, mon_id: u64) -> Option<usize> {
        let mon = self.monitors.iter().find(|m| m.info.id == mon_id)?;
        if let Some(&ws_id) = mon.active_by_desktop.get(os_desktop) {
            return Some(ws_id);
        }
        self.monitor_workspaces_for(os_desktop, mon_id)
            .first()
            .copied()
    }

    fn monitor_workspaces(&self, mon_id: u64) -> Vec<usize> {
        self.monitor_workspaces_for(&self.active_os_desktop, mon_id)
    }

    fn monitor_workspaces_for(&self, os_desktop: &str, mon_id: u64) -> Vec<usize> {
        let per_mon = self.config.workspaces.per_monitor_workspaces;
        self.workspaces
            .iter()
            .filter(|w| {
                self.ws_scope.get(&w.index).is_some_and(|s| {
                    s.os_desktop == os_desktop && (!per_mon || s.monitor_id == Some(mon_id))
                })
            })
            .map(|w| w.index)
            .collect()
    }

    fn current_monitor_id(&self) -> Option<u64> {
        if let Some(fid) = self.focused_window_id {
            if let Some(meta) = self.windows.get(&fid) {
                if let Some(owner) = self.monitor_owning_workspace(meta.workspace_id) {
                    return Some(owner);
                }
            }
        }
        self.focused_monitor_id
            .or_else(|| self.monitors.first().map(|m| m.info.id))
    }

    pub fn monitor_owning_workspace(&self, ws_id: usize) -> Option<u64> {
        if self.config.workspaces.per_monitor_workspaces {
            self.ws_scope.get(&ws_id).and_then(|s| s.monitor_id)
        } else {
            self.monitors
                .iter()
                .find(|m| m.active_workspace == ws_id)
                .map(|m| m.info.id)
                .or(self.focused_monitor_id)
        }
    }

    pub fn monitor_owning_window(&self, win_id: WindowId) -> Option<u64> {
        let ws_id = self.windows.get(&win_id)?.workspace_id;
        self.monitor_owning_workspace(ws_id)
    }

    pub fn move_window_to_workspace(
        &mut self,
        win_id: WindowId,
        target_ws_id: usize,
    ) -> Option<String> {
        self.move_window_to_workspace_id(win_id, target_ws_id)
    }

    pub fn set_focused_monitor(&mut self, mon_id: u64) {
        if self.monitors.iter().any(|m| m.info.id == mon_id) {
            self.focused_monitor_id = Some(mon_id);
        }
    }

    pub fn unpack_stacked_cells(&mut self) -> bool {
        let layout_cfg = self.config.layout.clone();
        let mut changed = false;
        for ws in &mut self.workspaces {
            changed |= ws.unpack_stacked_cells(&layout_cfg);
        }
        changed
    }

    fn current_workspace_id(&self) -> Option<usize> {
        if let Some(fid) = self.focused_window_id {
            if let Some(meta) = self.windows.get(&fid) {
                return Some(meta.workspace_id);
            }
        }
        let mon_id = self.current_monitor_id()?;
        self.active_workspace_for_monitor(mon_id)
    }

    fn workspace_mut(&mut self, ws_id: usize) -> Option<&mut GridWorkspace> {
        self.workspaces.iter_mut().find(|w| w.index == ws_id)
    }

    fn workspace(&self, ws_id: usize) -> Option<&GridWorkspace> {
        self.workspaces.iter().find(|w| w.index == ws_id)
    }

    pub fn is_managed(&self, id: WindowId) -> bool {
        self.windows.contains_key(&id)
    }

    pub fn set_cloaked_state(&mut self, id: WindowId, cloaked: bool) {
        if let Some(meta) = self.windows.get_mut(&id) {
            meta.cloaked_by_wm = cloaked;
        }
    }

    pub fn is_cloaked_by_wm(&self, id: WindowId) -> bool {
        self.windows.get(&id).is_some_and(|m| m.cloaked_by_wm)
    }

    pub fn is_floating(&self, id: WindowId) -> bool {
        self.windows
            .get(&id)
            .and_then(|m| self.workspace(m.workspace_id))
            .is_some_and(|ws| ws.floating.contains(&id))
    }

    pub fn all_managed_windows(&self) -> Vec<WindowId> {
        self.windows.keys().copied().collect()
    }

    pub fn add_window(&mut self, info: WindowInfo) -> bool {
        if let Some(meta) = self.windows.get_mut(&info.id) {
            meta.title = info.title;
            meta.process_name = info.process_name;
            meta.class_name = info.class_name;
            return false;
        }

        let rule = self.match_rules(&info);
        if rule.ignore {
            return false;
        }

        let mon_id = info
            .monitor_id
            .or_else(|| self.monitor_for_rect(info.rect))
            .or(self.focused_monitor_id)
            .or_else(|| self.monitors.first().map(|m| m.info.id));

        let Some(mon_id) = mon_id else {
            return false;
        };

        let target_desktop = info
            .desktop_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.active_os_desktop)
            .to_string();

        if !self.os_desktops.iter().any(|d| d == &target_desktop) {
            self.os_desktops.push(target_desktop.clone());
            self.ensure_workspaces_for_desktops_and_monitors();
            self.sync_monitor_active_workspaces();
        }

        let target_ws_id = if let Some(one_based) = rule.workspace {
            let list = self.monitor_workspaces_for(&target_desktop, mon_id);
            list.get(one_based.saturating_sub(1))
                .copied()
                .or_else(|| self.active_workspace_for(&target_desktop, mon_id))
        } else {
            self.active_workspace_for(&target_desktop, mon_id)
        };

        let Some(ws_id) = target_ws_id else {
            return false;
        };

        let layout_cfg = self.config.layout.clone();
        let ws_key = self.workspace_persist_key(ws_id);
        let restored = if layout_cfg.persist_state
            && now_epoch_secs() <= self.restore_deadline_epoch
        {
            if let Some(ref key) = ws_key {
                self.restore_pending.get_mut(key).and_then(|pending| {
                    let m =
                        pending.take_match(&info.process_name, &info.class_name, &info.title)?;
                    let occupied = pending.claimed.contains(&(m.col, m.row));
                    if !occupied {
                        pending.claimed.insert((m.col, m.row));
                        Some(m)
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        } else {
            None
        };

        if let Some(ws) = self.workspace_mut(ws_id) {
            if let Some(m) = restored {
                ws.insert_window_at(
                    info.id,
                    Coord::new(m.col, m.row),
                    rule.float || m.floating,
                    m.width_step,
                    m.height_step,
                    &layout_cfg,
                );
                if !(rule.float || m.floating) {
                    ws.apply_restored_snap(
                        info.id,
                        CellSnap {
                            width_step: m
                                .width_step
                                .min(layout_cfg.width_steps.len().saturating_sub(1)),
                            height_step: m
                                .height_step
                                .min(layout_cfg.height_steps.len().saturating_sub(1)),
                            maximized: m.maximized,
                            manual_override: m.manual_override,
                            custom_width_frac: m.custom_width_frac,
                            custom_height_frac: m.custom_height_frac,
                        },
                    );
                }
            } else {
                ws.insert_window(
                    info.id,
                    rule.float,
                    rule.width_step,
                    rule.height_step,
                    &layout_cfg,
                );
            }
        }

        self.windows.insert(
            info.id,
            WindowMeta {
                title: info.title,
                process_name: info.process_name,
                class_name: info.class_name,
                workspace_id: ws_id,
                os_desktop: target_desktop.clone(),
                cloaked_by_wm: false,
            },
        );
        if target_desktop == self.active_os_desktop {
            self.focused_window_id = Some(info.id);
            self.focused_monitor_id = Some(mon_id);
        }
        self.state_dirty = true;
        self.maintain_dynamic_empty_workspaces();
        true
    }

    /// Migrate a managed window to a different Windows OS virtual desktop, creating the
    /// desktop's per-monitor scroller instance automatically if not yet registered.
    pub fn move_window_to_os_desktop(&mut self, id: WindowId, target_desktop: &str) -> bool {
        let target = target_desktop.trim();
        if target.is_empty() {
            return false;
        }
        let Some(old_ws_id) = self.windows.get(&id).map(|m| m.workspace_id) else {
            return false;
        };
        if self
            .windows
            .get(&id)
            .is_some_and(|m| m.os_desktop == target)
        {
            return false;
        }

        if !self.os_desktops.iter().any(|d| d == target) {
            self.os_desktops.push(target.to_string());
            self.ensure_workspaces_for_desktops_and_monitors();
            self.sync_monitor_active_workspaces();
        }

        let mon_id = self
            .monitor_owning_workspace(old_ws_id)
            .or(self.focused_monitor_id)
            .or_else(|| self.monitors.first().map(|m| m.info.id));
        let Some(mon_id) = mon_id else {
            return false;
        };
        let Some(target_ws_id) = self.active_workspace_for(target, mon_id) else {
            return false;
        };

        self.move_window_to_workspace_id(id, target_ws_id);
        if let Some(meta) = self.windows.get_mut(&id) {
            meta.os_desktop = target.to_string();
        }
        if target == self.active_os_desktop {
            self.focused_window_id = Some(id);
            self.focused_monitor_id = Some(mon_id);
        } else if self.focused_window_id == Some(id) {
            self.focused_window_id = self.workspace(old_ws_id).and_then(|w| w.focused);
        }
        self.state_dirty = true;
        true
    }

    pub fn remove_window(&mut self, id: WindowId) -> bool {
        let Some(meta) = self.windows.remove(&id) else {
            return false;
        };
        let compact = self.config.layout.auto_compact_empty;
        if let Some(ws) = self.workspace_mut(meta.workspace_id) {
            ws.remove_window_with_compact(id, compact);
        }
        if self.focused_window_id == Some(id) {
            self.focused_window_id = self.workspace(meta.workspace_id).and_then(|ws| ws.focused);
        }
        self.state_dirty = true;
        true
    }

    pub fn update_window_title(&mut self, id: WindowId, title: String) {
        if let Some(meta) = self.windows.get_mut(&id) {
            meta.title = title;
        }
    }

    pub fn on_foreground_changed(&mut self, id: WindowId) -> bool {
        let Some((ws_id, win_desktop)) = self
            .windows
            .get(&id)
            .map(|m| (m.workspace_id, m.os_desktop.clone()))
        else {
            return false;
        };
        if win_desktop != self.active_os_desktop {
            return false;
        }
        let mon_id = self.monitor_owning_workspace(ws_id);
        if let Some(mon_id) = mon_id {
            if let Some(mon) = self.monitors.iter_mut().find(|m| m.info.id == mon_id) {
                mon.active_workspace = ws_id;
                mon.active_by_desktop
                    .insert(self.active_os_desktop.clone(), ws_id);
            }
            self.focused_monitor_id = Some(mon_id);
        }
        if let Some(ws) = self.workspace_mut(ws_id) {
            ws.set_focus(id);
        }
        self.focused_window_id = Some(id);
        true
    }

    pub fn focus_window(&mut self, id: WindowId) {
        self.on_foreground_changed(id);
    }

    /// Returns `(max_cell_w, max_cell_h)` for the monitor currently hosting `win_id`.
    pub fn window_max_cell_size(&self, win_id: WindowId) -> Option<(f64, f64)> {
        let ws_id = self.windows.get(&win_id)?.workspace_id;
        let area = self
            .ws_scope
            .get(&ws_id)
            .and_then(|s| s.monitor_id)
            .and_then(|mid| self.monitors.iter().find(|m| m.info.id == mid))
            .or_else(|| {
                self.monitors
                    .iter()
                    .find(|m| m.active_workspace == ws_id)
                    .or_else(|| self.monitors.first())
            })
            .map(|m| m.info.work_area)?;
        let area_w = area.w.max(1) as f64;
        let area_h = area.h.max(1) as f64;
        let peek_x = (self.config.layout.peek_x as f64).clamp(0.0, ((area_w - 1.0) / 2.0).max(0.0));
        let peek_y = (self.config.layout.peek_y as f64).clamp(0.0, ((area_h - 1.0) / 2.0).max(0.0));
        Some((
            (area_w - peek_x * 2.0).max(1.0),
            (area_h - peek_y * 2.0).max(1.0),
        ))
    }

    pub fn focused_window_rect_on_monitor(&self, mon_bounds: IRect) -> Option<IRect> {
        let win_id = self.focused_window_id?;
        let ws_id = self.windows.get(&win_id)?.workspace_id;
        let ws = self.workspace(ws_id)?;
        let r = ws.cell_rect_for(win_id)?;
        Some(IRect::new(r.x - mon_bounds.x, r.y - mon_bounds.y, r.w, r.h))
    }

    pub fn record_window_snap(
        &mut self,
        win_id: WindowId,
        width_frac: f64,
        height_frac: f64,
    ) -> bool {
        let Some(ws_id) = self.windows.get(&win_id).map(|m| m.workspace_id) else {
            return false;
        };
        let w_steps = self.config.layout.width_steps.clone();
        let h_steps = self.config.layout.height_steps.clone();
        let changed = self.workspace_mut(ws_id).is_some_and(|ws| {
            ws.set_custom_snap(win_id, width_frac, height_frac, &w_steps, &h_steps)
        });
        if changed {
            self.state_dirty = true;
        }
        changed
    }

    pub fn record_window_maximized(&mut self, win_id: WindowId, maximized: bool) -> bool {
        let Some(ws_id) = self.windows.get(&win_id).map(|m| m.workspace_id) else {
            return false;
        };
        let changed = self
            .workspace_mut(ws_id)
            .is_some_and(|ws| ws.set_maximized(win_id, maximized));
        if changed {
            self.state_dirty = true;
        }
        changed
    }

    pub fn is_window_maximized(&self, win_id: WindowId) -> bool {
        let Some(ws_id) = self.windows.get(&win_id).map(|m| m.workspace_id) else {
            return false;
        };
        let Some(ws) = self.workspace(ws_id) else {
            return false;
        };
        let Some(coord) = ws.loc.get(&win_id) else {
            return false;
        };
        ws.cells.get(coord).is_some_and(|c| c.maximized)
    }

    pub fn dispatch(&mut self, action: Action) -> DispatchEffects {
        let mut effects = DispatchEffects::default();
        match action {
            Action::TogglePause => {
                self.paused = !self.paused;
                effects.paused = Some(self.paused);
                if !self.paused {
                    effects.layout_dirty = true;
                }
            }
            Action::ToggleWorkspaceLayout => {
                if let Some(ws_id) = self.current_workspace_id() {
                    if let Some(ws) = self.workspace_mut(ws_id) {
                        ws.layout_disabled = !ws.layout_disabled;
                        self.state_dirty = true;
                        effects.state_dirty = true;
                        effects.layout_dirty = true;
                    }
                }
            }
            Action::OpenSettings => {
                effects.open_settings = true;
            }
            Action::ReloadConfig => {
                effects.reload_config = true;
            }
            Action::Quit => {
                effects.quit = true;
            }
            Action::Retile => {
                for ws in &mut self.workspaces {
                    ws.retile(&self.config.layout);
                }
                self.state_dirty = true;
                effects.state_dirty = true;
                effects.layout_dirty = true;
            }
            Action::ToggleOverview => {
                self.overview_open = !self.overview_open;
                effects.overview_open = Some(self.overview_open);
            }
            Action::Focus { direction } => {
                if let Some(ws_id) = self.current_workspace_id() {
                    let target = self
                        .workspace_mut(ws_id)
                        .and_then(|ws| ws.focus_direction(direction));
                    if let Some(win_id) = target {
                        self.focused_window_id = Some(win_id);
                        effects.focus_window = Some(win_id);
                        effects.layout_dirty = true;
                    }
                }
            }
            Action::MoveWindow { direction } => {
                let preserve_holes = self.config.layout.preserve_holes;
                let compact_rows = self.config.layout.auto_compact_empty;
                if let Some(ws_id) = self.current_workspace_id() {
                    if let Some(ws) = self.workspace_mut(ws_id) {
                        if ws.move_focused_with_compact(direction, preserve_holes, compact_rows) {
                            self.state_dirty = true;
                            effects.state_dirty = true;
                            effects.layout_dirty = true;
                        }
                    }
                }
            }
            Action::PanCamera { direction } => {
                if let Some(ws_id) = self.current_workspace_id() {
                    if let Some(ws) = self.workspace_mut(ws_id) {
                        if ws.pan_grid(direction) {
                            effects.layout_dirty = true;
                        }
                    }
                }
            }
            Action::PanPixels { dx, dy } => {
                if let Some(ws_id) = self.current_workspace_id() {
                    if let Some(ws) = self.workspace_mut(ws_id) {
                        if ws.pan_pixels(dx, dy) {
                            effects.layout_dirty = true;
                        }
                    }
                }
            }
            Action::PanCommit { vx, vy } => {
                if let Some(ws_id) = self.current_workspace_id() {
                    let target = self
                        .workspace_mut(ws_id)
                        .and_then(|ws| ws.finish_pixel_pan(vx, vy));
                    if let Some(win_id) = target {
                        self.focused_window_id = Some(win_id);
                        effects.focus_window = Some(win_id);
                    }
                    effects.layout_dirty = true;
                }
            }
            Action::CenterCamera => {
                if let Some(ws_id) = self.current_workspace_id() {
                    if let Some(ws) = self.workspace_mut(ws_id) {
                        if ws.follow_focused() {
                            effects.layout_dirty = true;
                        }
                    }
                }
            }
            Action::ResizeWidth { delta } => {
                let len = self.config.layout.width_steps.len();
                if let Some(ws_id) = self.current_workspace_id() {
                    if let Some(ws) = self.workspace_mut(ws_id) {
                        if ws.resize_width(delta, len) {
                            self.state_dirty = true;
                            effects.state_dirty = true;
                            effects.layout_dirty = true;
                        }
                    }
                }
            }
            Action::ResizeHeight { delta } => {
                let len = self.config.layout.height_steps.len();
                if let Some(ws_id) = self.current_workspace_id() {
                    if let Some(ws) = self.workspace_mut(ws_id) {
                        if ws.resize_height(delta, len) {
                            self.state_dirty = true;
                            effects.state_dirty = true;
                            effects.layout_dirty = true;
                        }
                    }
                }
            }
            Action::CycleWidthPreset => {
                let len = self.config.layout.width_steps.len();
                if let Some(ws_id) = self.current_workspace_id() {
                    if let Some(ws) = self.workspace_mut(ws_id) {
                        if ws.cycle_width(len) {
                            self.state_dirty = true;
                            effects.state_dirty = true;
                            effects.layout_dirty = true;
                        }
                    }
                }
            }
            Action::CycleHeightPreset => {
                let len = self.config.layout.height_steps.len();
                if let Some(ws_id) = self.current_workspace_id() {
                    if let Some(ws) = self.workspace_mut(ws_id) {
                        if ws.cycle_height(len) {
                            self.state_dirty = true;
                            effects.state_dirty = true;
                            effects.layout_dirty = true;
                        }
                    }
                }
            }
            Action::ConsumeIntoCell { direction } => {
                if let Some(ws_id) = self.current_workspace_id() {
                    if let Some(ws) = self.workspace_mut(ws_id) {
                        if ws.consume_into_cell(direction) {
                            self.state_dirty = true;
                            effects.state_dirty = true;
                            effects.layout_dirty = true;
                        }
                    }
                }
            }
            Action::ExpelFromCell { direction } => {
                if let Some(ws_id) = self.current_workspace_id() {
                    if let Some(ws) = self.workspace_mut(ws_id) {
                        if ws.expel_from_cell(direction.unwrap_or(Direction::Right)) {
                            self.state_dirty = true;
                            effects.state_dirty = true;
                            effects.layout_dirty = true;
                        }
                    }
                }
            }
            Action::ToggleCellStackMode => {
                if let Some(ws_id) = self.current_workspace_id() {
                    if let Some(ws) = self.workspace_mut(ws_id) {
                        if ws.toggle_stack_mode() {
                            self.state_dirty = true;
                            effects.state_dirty = true;
                            effects.layout_dirty = true;
                        }
                    }
                }
            }
            Action::ToggleMaximizeCell => {
                if let Some(ws_id) = self.current_workspace_id() {
                    if let Some(ws) = self.workspace_mut(ws_id) {
                        if ws.toggle_maximize_cell() {
                            self.state_dirty = true;
                            effects.state_dirty = true;
                            effects.layout_dirty = true;
                        }
                    }
                }
            }
            Action::ToggleFullscreen => {
                if let Some(win_id) = self.focused_window_id {
                    if let Some(ws_id) = self.windows.get(&win_id).map(|m| m.workspace_id) {
                        if let Some(ws) = self.workspace_mut(ws_id) {
                            ws.fullscreen = if ws.fullscreen == Some(win_id) {
                                None
                            } else {
                                Some(win_id)
                            };
                            self.state_dirty = true;
                            effects.state_dirty = true;
                            effects.layout_dirty = true;
                        }
                    }
                }
            }
            Action::ToggleFloating => {
                if let Some(win_id) = self.focused_window_id {
                    if let Some(ws_id) = self.windows.get(&win_id).map(|m| m.workspace_id) {
                        let layout_cfg = self.config.layout.clone();
                        if let Some(ws) = self.workspace_mut(ws_id) {
                            if ws.toggle_floating(win_id, &layout_cfg).is_some() {
                                self.state_dirty = true;
                                effects.state_dirty = true;
                                effects.layout_dirty = true;
                            }
                        }
                    }
                }
            }
            Action::CloseFocused => {
                if let Some(win_id) = self.focused_window_id {
                    effects.close_window = Some(win_id);
                }
            }
            Action::SwitchWorkspace { index } => {
                self.switch_workspace_slot(index, &mut effects);
            }
            Action::SwitchWorkspaceRelative { delta } => {
                if let Some(mon_id) = self.current_monitor_id() {
                    let list = self.monitor_workspaces(mon_id);
                    if let Some(cur_ws) = self.active_workspace_for_monitor(mon_id) {
                        if let Some(pos) = list.iter().position(|&id| id == cur_ws) {
                            let next_pos = (pos as isize + delta as isize)
                                .clamp(0, list.len().saturating_sub(1) as isize)
                                as usize;
                            self.switch_workspace_slot(next_pos, &mut effects);
                        }
                    }
                }
            }
            Action::MoveToWorkspace { index } => {
                self.move_focused_to_workspace_slot(index, &mut effects);
            }
            Action::FocusMonitor { direction } => {
                if let Some(target_mon) = self.find_monitor_in_direction(direction) {
                    self.focused_monitor_id = Some(target_mon);
                    if let Some(ws_id) = self.active_workspace_for_monitor(target_mon) {
                        if let Some(win_id) = self.workspace(ws_id).and_then(|w| w.focused) {
                            self.focused_window_id = Some(win_id);
                            effects.focus_window = Some(win_id);
                        }
                    }
                    effects.layout_dirty = true;
                }
            }
            Action::MoveToMonitor { direction } => {
                if let (Some(win_id), Some(target_mon)) = (
                    self.focused_window_id,
                    self.find_monitor_in_direction(direction),
                ) {
                    if let Some(target_ws) = self.active_workspace_for_monitor(target_mon) {
                        if let Some(target_desktop) =
                            self.move_window_to_workspace_id(win_id, target_ws)
                        {
                            effects.moved_to_desktop = Some((win_id, target_desktop));
                        }
                        self.focused_monitor_id = Some(target_mon);
                        effects.focus_window = Some(win_id);
                        self.state_dirty = true;
                        effects.state_dirty = true;
                        effects.layout_dirty = true;
                    }
                }
            }
            Action::JumpToCell {
                col,
                row,
                window_id,
            } => {
                if let Some(ws_id) = self.current_workspace_id() {
                    let target = self
                        .workspace_mut(ws_id)
                        .and_then(|ws| ws.jump_to(Coord::new(col, row), window_id));
                    if let Some(win_id) = target {
                        self.focused_window_id = Some(win_id);
                        effects.focus_window = Some(win_id);
                    }
                    self.overview_open = false;
                    effects.overview_open = Some(false);
                    self.state_dirty = true;
                    effects.state_dirty = true;
                    effects.layout_dirty = true;
                }
            }
            Action::PlaceWindowAt {
                window_id,
                col,
                row,
                stack,
            } => {
                let ws_id = self
                    .windows
                    .get(&window_id)
                    .map(|m| m.workspace_id)
                    .or_else(|| self.current_workspace_id());
                if let Some(ws_id) = ws_id {
                    let preserve_holes = self.config.layout.preserve_holes;
                    let layout_cfg = self.config.layout.clone();
                    let moved = self.workspace_mut(ws_id).is_some_and(|ws| {
                        ws.place_window_at(
                            window_id,
                            Coord::new(col, row),
                            stack,
                            preserve_holes,
                            &layout_cfg,
                        )
                    });
                    if moved {
                        self.focused_window_id = Some(window_id);
                        if !self.overview_open {
                            effects.focus_window = Some(window_id);
                        }
                        self.state_dirty = true;
                        effects.state_dirty = true;
                        effects.layout_dirty = true;
                    }
                }
            }
            Action::SplitWindowHorizontal {
                window_id,
                target_window_id,
                on_left,
            } => {
                let ws_id = self
                    .windows
                    .get(&window_id)
                    .map(|m| m.workspace_id)
                    .or_else(|| self.current_workspace_id());
                if let Some(ws_id) = ws_id {
                    let layout_cfg = self.config.layout.clone();
                    let split = self.workspace_mut(ws_id).is_some_and(|ws| {
                        ws.split_window_horizontal(
                            window_id,
                            target_window_id,
                            on_left,
                            &layout_cfg,
                        )
                    });
                    if split {
                        self.focused_window_id = Some(window_id);
                        if !self.overview_open {
                            effects.focus_window = Some(window_id);
                        }
                        self.state_dirty = true;
                        effects.state_dirty = true;
                        effects.layout_dirty = true;
                    }
                }
            }
        }
        effects
    }

    fn switch_workspace_slot(&mut self, slot: usize, effects: &mut DispatchEffects) {
        let Some(mon_id) = self.current_monitor_id() else {
            return;
        };
        let list = self.monitor_workspaces(mon_id);
        let Some(&target_ws_id) = list.get(slot) else {
            return;
        };
        if let Some(mon) = self.monitors.iter_mut().find(|m| m.info.id == mon_id) {
            if mon.active_workspace == target_ws_id {
                return;
            }
            mon.active_workspace = target_ws_id;
            mon.active_by_desktop
                .insert(self.active_os_desktop.clone(), target_ws_id);
        }
        self.focused_monitor_id = Some(mon_id);
        let focus = self.workspace(target_ws_id).and_then(|w| w.focused);
        self.focused_window_id = focus;
        effects.focus_window = focus;
        effects.layout_dirty = true;
    }

    fn move_focused_to_workspace_slot(&mut self, slot: usize, effects: &mut DispatchEffects) {
        let Some(win_id) = self.focused_window_id else {
            return;
        };
        let Some(mon_id) = self.current_monitor_id() else {
            return;
        };
        let list = self.monitor_workspaces(mon_id);
        let Some(&target_ws_id) = list.get(slot) else {
            return;
        };
        if let Some(target_desktop) = self.move_window_to_workspace_id(win_id, target_ws_id) {
            effects.moved_to_desktop = Some((win_id, target_desktop));
        }
        if let Some(mon) = self.monitors.iter_mut().find(|m| m.info.id == mon_id) {
            mon.active_workspace = target_ws_id;
            mon.active_by_desktop
                .insert(self.active_os_desktop.clone(), target_ws_id);
        }
        self.focused_window_id = Some(win_id);
        effects.focus_window = Some(win_id);
        self.state_dirty = true;
        effects.state_dirty = true;
        effects.layout_dirty = true;
    }

    fn move_window_to_workspace_id(
        &mut self,
        win_id: WindowId,
        target_ws_id: usize,
    ) -> Option<String> {
        let Some(old_ws_id) = self.windows.get(&win_id).map(|m| m.workspace_id) else {
            return None;
        };
        if old_ws_id == target_ws_id {
            return None;
        }
        let was_floating = self
            .workspace(old_ws_id)
            .is_some_and(|w| w.floating.contains(&win_id));
        let prev_snap = self
            .workspace(old_ws_id)
            .and_then(|w| w.loc.get(&win_id).and_then(|c| w.cells.get(c)))
            .map(|c| c.snap_state());

        if let Some(old_ws) = self.workspace_mut(old_ws_id) {
            old_ws.remove_window(win_id);
        }
        let layout_cfg = self.config.layout.clone();
        if let Some(new_ws) = self.workspace_mut(target_ws_id) {
            new_ws.insert_window(
                win_id,
                was_floating,
                prev_snap.map(|s| s.width_step),
                prev_snap.map(|s| s.height_step),
                &layout_cfg,
            );
            if !was_floating {
                if let Some(snap) = prev_snap {
                    new_ws.apply_restored_snap(win_id, snap);
                }
            }
        }
        let target_desktop = self
            .ws_scope
            .get(&target_ws_id)
            .map(|s| s.os_desktop.clone());
        if let Some(meta) = self.windows.get_mut(&win_id) {
            meta.workspace_id = target_ws_id;
            if let Some(ref d) = target_desktop {
                meta.os_desktop = d.clone();
            }
        }
        self.maintain_dynamic_empty_workspaces();
        target_desktop
    }

    fn find_monitor_in_direction(&self, dir: Direction) -> Option<u64> {
        let cur_id = self.current_monitor_id()?;
        let cur = self.monitors.iter().find(|m| m.info.id == cur_id)?;
        let (cx, cy) = cur.info.bounds.center();

        self.monitors
            .iter()
            .filter(|m| m.info.id != cur_id)
            .filter_map(|m| {
                let (mx, my) = m.info.bounds.center();
                let dx = mx - cx;
                let dy = my - cy;
                let ok = match dir {
                    Direction::Left => dx < 0,
                    Direction::Right => dx > 0,
                    Direction::Up => dy < 0,
                    Direction::Down => dy > 0,
                };
                if !ok {
                    return None;
                }
                let dist = dx.abs() + dy.abs();
                Some((dist, m.info.id))
            })
            .min_by_key(|(d, _)| *d)
            .map(|(_, id)| id)
    }

    pub fn compute_frame(&mut self, dt_secs: f64) -> FramePlan {
        let mut plan = FramePlan::default();
        if self.paused {
            return plan;
        }

        let all_mon_bounds: Vec<(u64, IRect)> = self
            .monitors
            .iter()
            .map(|m| (m.info.id, m.info.bounds))
            .collect();
        let active_map: HashMap<usize, (IRect, IRect, Vec<IRect>)> = self
            .monitors
            .iter()
            .map(|m| {
                let others: Vec<IRect> = all_mon_bounds
                    .iter()
                    .filter(|(id, _)| *id != m.info.id)
                    .map(|(_, r)| *r)
                    .collect();
                (
                    m.active_workspace,
                    (m.info.work_area, m.info.bounds, others),
                )
            })
            .collect();

        let layout_cfg = self.config.layout.clone();
        let anim_cfg = self.config.animation.clone();
        let focused_win = self.focused_window_id;
        let active_desktop = self.active_os_desktop.clone();
        let cur_ws_id = self.current_workspace_id();

        for ws in &mut self.workspaces {
            if self
                .ws_scope
                .get(&ws.index)
                .is_some_and(|s| s.os_desktop != active_desktop)
            {
                // Leave windows on inactive OS virtual desktops untouched by WM tiling/cloaking.
                continue;
            }
            if let Some(&(work_area, full_bounds, ref others)) = active_map.get(&ws.index) {
                let is_overview_target_ws = self.overview_open
                    && cur_ws_id.map_or_else(|| active_map.len() == 1, |id| id == ws.index);

                if is_overview_target_ws && !layout_cfg.overview_thumbnails {
                    for p in ws.compute_overview_resize_placements(work_area, &layout_cfg) {
                        plan.commands.push(WindowCommand {
                            window_id: p.window_id,
                            rect: Some(p.rect),
                            visible: p.visible,
                            clip_to: None,
                            is_focused: focused_win == Some(p.window_id),
                            is_floating: false,
                        });
                    }
                    continue;
                }

                if is_overview_target_ws && layout_cfg.overview_thumbnails {
                    // In DWM live thumbnail overview mode:
                    // All windows in the overview workspace must:
                    // 1. Be uncloaked (`visible: true`), so DWM maintains their composition surfaces.
                    // 2. Have clipping removed (`clip_to: None`), so SetWindowRgn does not cut off peek areas.
                    // 3. Be positioned at their natural full tiled size clamped inside the active monitor's
                    //    work_area. This prevents DirectComposition / Chromium apps (Chrome, Edge, VS Code)
                    //    from detecting (-32000, -32000) or off-screen bounds and discarding their swapchains,
                    //    while guaranteeing they never bleed across multi-monitor display boundaries.
                    let (placements, _) = ws.compute_placements(
                        work_area,
                        others,
                        &layout_cfg,
                        false,
                        false,
                        dt_secs,
                        anim_cfg.stiffness,
                        anim_cfg.damping_ratio,
                        anim_cfg.snap_epsilon_px,
                    );
                    for p in placements {
                        let eff_w = p.rect.w.min(work_area.w).max(40);
                        let eff_h = p.rect.h.min(work_area.h).max(40);
                        let max_x = (work_area.right() - eff_w).max(work_area.x);
                        let max_y = (work_area.bottom() - eff_h).max(work_area.y);
                        let eff_x = p.rect.x.clamp(work_area.x, max_x);
                        let eff_y = p.rect.y.clamp(work_area.y, max_y);
                        let clamped_rect = IRect::new(eff_x, eff_y, eff_w, eff_h);

                        plan.commands.push(WindowCommand {
                            window_id: p.window_id,
                            rect: Some(clamped_rect),
                            visible: true,
                            clip_to: None,
                            is_focused: focused_win == Some(p.window_id),
                            is_floating: false,
                        });
                    }

                    for &fid in &ws.floating {
                        plan.commands.push(WindowCommand {
                            window_id: fid,
                            rect: None,
                            visible: true,
                            clip_to: None,
                            is_focused: focused_win == Some(fid),
                            is_floating: true,
                        });
                    }
                    continue;
                }

                if ws.layout_disabled {
                    for win_id in ws.all_windows() {
                        plan.commands.push(WindowCommand {
                            window_id: win_id,
                            rect: None,
                            visible: !self.overview_open,
                            clip_to: None,
                            is_focused: focused_win == Some(win_id),
                            is_floating: true,
                        });
                    }
                    continue;
                }

                if let Some(fs_id) = ws.fullscreen {
                    for win_id in ws.all_windows() {
                        plan.commands.push(WindowCommand {
                            window_id: win_id,
                            rect: (win_id == fs_id).then_some(full_bounds),
                            visible: win_id == fs_id,
                            clip_to: None,
                            is_focused: focused_win == Some(win_id),
                            is_floating: ws.floating.contains(&win_id),
                        });
                    }
                    continue;
                }

                let (placements, animating) = ws.compute_placements(
                    work_area,
                    others,
                    &layout_cfg,
                    anim_cfg.enabled,
                    anim_cfg.animate_resize,
                    dt_secs,
                    anim_cfg.stiffness,
                    anim_cfg.damping_ratio,
                    anim_cfg.snap_epsilon_px,
                );
                plan.still_animating |= animating;

                for p in placements {
                    plan.commands.push(WindowCommand {
                        window_id: p.window_id,
                        rect: Some(p.rect),
                        visible: p.visible,
                        clip_to: p.clip_to,
                        is_focused: focused_win == Some(p.window_id),
                        is_floating: false,
                    });
                }

                for &fid in &ws.floating {
                    plan.commands.push(WindowCommand {
                        window_id: fid,
                        rect: None,
                        visible: true,
                        clip_to: None,
                        is_focused: focused_win == Some(fid),
                        is_floating: true,
                    });
                }
            } else {
                // Inactive workspace: cloak all its windows cleanly.
                for win_id in ws.all_windows() {
                    plan.commands.push(WindowCommand {
                        window_id: win_id,
                        rect: None,
                        visible: false,
                        clip_to: None,
                        is_focused: false,
                        is_floating: ws.floating.contains(&win_id),
                    });
                }
            }
        }

        plan
    }

    pub fn snapshot(&self) -> StateSnapshot {
        let monitors = self
            .monitors
            .iter()
            .map(|m| MonitorSnapshot {
                id: m.info.id,
                name: m.info.name.clone(),
                bounds: (
                    m.info.bounds.x,
                    m.info.bounds.y,
                    m.info.bounds.w,
                    m.info.bounds.h,
                ),
                work_area: (
                    m.info.work_area.x,
                    m.info.work_area.y,
                    m.info.work_area.w,
                    m.info.work_area.h,
                ),
                active_workspace: m.active_workspace,
                is_focused: self.focused_monitor_id == Some(m.info.id),
            })
            .collect();

        let workspaces = self
            .workspaces
            .iter()
            .map(|ws| {
                let mut windows = Vec::new();
                for (&coord, cell) in &ws.cells {
                    for (idx, &win_id) in cell.windows.iter().enumerate() {
                        let meta = self.windows.get(&win_id);
                        windows.push(WindowSnapshot {
                            id: win_id,
                            title: meta.map(|m| m.title.clone()).unwrap_or_default(),
                            process_name: meta.map(|m| m.process_name.clone()).unwrap_or_default(),
                            col: coord.col,
                            row: coord.row,
                            stack_index: idx,
                            stack_len: cell.windows.len(),
                            width_step: cell.width_step,
                            height_step: cell.height_step,
                            floating: false,
                            fullscreen: ws.fullscreen == Some(win_id),
                            focused: self.focused_window_id == Some(win_id),
                            cloaked: meta.is_some_and(|m| m.cloaked_by_wm),
                        });
                    }
                }
                for &win_id in &ws.floating {
                    let meta = self.windows.get(&win_id);
                    windows.push(WindowSnapshot {
                        id: win_id,
                        title: meta.map(|m| m.title.clone()).unwrap_or_default(),
                        process_name: meta.map(|m| m.process_name.clone()).unwrap_or_default(),
                        col: 0,
                        row: 0,
                        stack_index: 0,
                        stack_len: 1,
                        width_step: 0,
                        height_step: 0,
                        floating: true,
                        fullscreen: ws.fullscreen == Some(win_id),
                        focused: self.focused_window_id == Some(win_id),
                        cloaked: meta.is_some_and(|m| m.cloaked_by_wm),
                    });
                }

                let scope = self.ws_scope.get(&ws.index);
                WorkspaceSnapshot {
                    index: ws.index,
                    name: ws.name.clone(),
                    desktop_id: scope.map(|s| s.os_desktop.clone()).unwrap_or_default(),
                    monitor_id: scope.and_then(|s| s.monitor_id),
                    layout_disabled: ws.layout_disabled,
                    camera_col: ws.camera_coord.col,
                    camera_row: ws.camera_coord.row,
                    camera_px: (ws.camera_anim.current_x(), ws.camera_anim.current_y()),
                    window_count: windows.len(),
                    windows,
                }
            })
            .collect();

        StateSnapshot {
            paused: self.paused,
            overview_open: self.overview_open,
            focused_window_id: self.focused_window_id,
            active_desktop_id: self.active_os_desktop.clone(),
            desktop_ids: self.os_desktops.clone(),
            monitors,
            workspaces,
        }
    }

    pub fn active_workspace_snapshot(&self) -> Option<WorkspaceSnapshot> {
        let ws_id = self.current_workspace_id()?;
        self.snapshot()
            .workspaces
            .into_iter()
            .find(|w| w.index == ws_id)
    }
}

fn rule_matches(rule: &WindowRule, info: &WindowInfo) -> bool {
    if rule.exe.is_none() && rule.class.is_none() && rule.title.is_none() {
        return false;
    }
    if let Some(ref exe) = rule.exe {
        if !contains_icase(&info.process_name, exe) {
            return false;
        }
    }
    if let Some(ref cls) = rule.class {
        if !contains_icase(&info.class_name, cls) {
            return false;
        }
    }
    if let Some(ref title) = rule.title {
        if !contains_icase(&info.title, title) {
            return false;
        }
    }
    true
}

fn contains_icase(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

// Keep unused import clean if Placement isn't needed directly outside crate.
#[allow(dead_code)]
fn _assert_placement_reachable(_: WindowPlacement) {}
