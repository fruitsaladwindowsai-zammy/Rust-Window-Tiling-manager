use crate::animation::{CameraAnim, Spring1D};
use crate::config::{FocusCentering, LayoutConfig, NewWindowPlacement};
use crate::ipc::Direction;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

pub type WindowId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Coord {
    pub col: i32,
    pub row: i32,
}

impl Coord {
    pub const ZERO: Self = Self { col: 0, row: 0 };

    pub fn new(col: i32, row: i32) -> Self {
        Self { col, row }
    }

    pub fn step(self, dir: Direction) -> Self {
        let (dc, dr) = dir.delta();
        Self {
            col: self.col + dc,
            row: self.row + dr,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl IRect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(self) -> i32 {
        self.y + self.h
    }

    pub fn intersects(self, other: Self) -> bool {
        self.w > 0
            && self.h > 0
            && other.w > 0
            && other.h > 0
            && self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());
        if x2 > x1 && y2 > y1 {
            Some(Self::new(x1, y1, x2 - x1, y2 - y1))
        } else {
            None
        }
    }

    pub fn center(self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h / 2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StackMode {
    #[default]
    Split,
    Tabbed,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CellSnap {
    pub width_step: usize,
    pub height_step: usize,
    pub maximized: bool,
    pub manual_override: bool,
    pub custom_width_frac: Option<f64>,
    pub custom_height_frac: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub windows: Vec<WindowId>,
    pub active_idx: usize,
    pub stack_mode: StackMode,
    pub width_step: usize,
    pub height_step: usize,
    pub maximized: bool,
    pub manual_override: bool,
    pub custom_width_frac: Option<f64>,
    pub custom_height_frac: Option<f64>,
    pub anim_w: Spring1D,
    pub anim_h: Spring1D,
}

impl Cell {
    pub(crate) fn new(window_id: WindowId, width_step: usize, height_step: usize) -> Self {
        Self {
            windows: vec![window_id],
            active_idx: 0,
            stack_mode: StackMode::Split,
            width_step,
            height_step,
            maximized: false,
            manual_override: false,
            custom_width_frac: None,
            custom_height_frac: None,
            anim_w: Spring1D::default(),
            anim_h: Spring1D::default(),
        }
    }

    pub fn snap_state(&self) -> CellSnap {
        CellSnap {
            width_step: self.width_step,
            height_step: self.height_step,
            maximized: self.maximized,
            manual_override: self.manual_override,
            custom_width_frac: self.custom_width_frac,
            custom_height_frac: self.custom_height_frac,
        }
    }

    pub fn apply_snap(&mut self, snap: CellSnap) {
        self.width_step = snap.width_step;
        self.height_step = snap.height_step;
        self.maximized = snap.maximized;
        self.manual_override = snap.manual_override;
        self.custom_width_frac = snap.custom_width_frac;
        self.custom_height_frac = snap.custom_height_frac;
    }

    pub fn active_window(&self) -> WindowId {
        self.windows[self.active_idx.min(self.windows.len() - 1)]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowPlacement {
    pub window_id: WindowId,
    pub rect: IRect,
    /// When `false`, the window is completely outside the viewport or hidden in a tabbed stack
    /// and should be DWM-cloaked.
    pub visible: bool,
    /// Optional monitor work-area clamp if a peeking window overflows onto an adjacent monitor.
    pub clip_to: Option<IRect>,
    pub coord: Coord,
    pub stack_index: usize,
    pub stack_len: usize,
}

/// A single workspace backed by an infinite 2D sparse grid with per-cell Niri stacking.
#[derive(Debug, Clone)]
pub struct GridWorkspace {
    pub index: usize,
    pub name: String,
    pub layout_disabled: bool,
    pub cells: BTreeMap<Coord, Cell>,
    pub loc: HashMap<WindowId, Coord>,
    pub floating: Vec<WindowId>,
    pub fullscreen: Option<WindowId>,
    pub focused: Option<WindowId>,
    pub camera_coord: Coord,
    pub camera_pan_offset: (f64, f64),
    pub camera_manual_pan: bool,
    pub camera_anim: CameraAnim,
    last_anchor: Option<(Coord, f64, f64)>,
    last_viewport: Option<IRect>,
    last_cell_rects: HashMap<Coord, IRect>,
}

impl GridWorkspace {
    pub fn new(index: usize, name: impl Into<String>) -> Self {
        Self {
            index,
            name: name.into(),
            layout_disabled: false,
            cells: BTreeMap::new(),
            loc: HashMap::new(),
            floating: Vec::new(),
            fullscreen: None,
            focused: None,
            camera_coord: Coord::ZERO,
            camera_pan_offset: (0.0, 0.0),
            camera_manual_pan: false,
            camera_anim: CameraAnim::default(),
            last_anchor: None,
            last_viewport: None,
            last_cell_rects: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.loc.is_empty() && self.floating.is_empty()
    }

    pub fn contains_window(&self, id: WindowId) -> bool {
        self.loc.contains_key(&id) || self.floating.contains(&id)
    }

    pub fn tiled_window_count(&self) -> usize {
        self.loc.len()
    }

    pub fn all_windows(&self) -> Vec<WindowId> {
        let mut out: Vec<WindowId> = Vec::with_capacity(self.loc.len() + self.floating.len());
        for cell in self.cells.values() {
            out.extend_from_slice(&cell.windows);
        }
        out.extend_from_slice(&self.floating);
        out
    }

    pub fn insert_window(
        &mut self,
        id: WindowId,
        floating: bool,
        width_step: Option<usize>,
        height_step: Option<usize>,
        config: &LayoutConfig,
    ) {
        if self.contains_window(id) {
            return;
        }
        let w_step = width_step
            .unwrap_or(config.default_width_step)
            .min(config.width_steps.len().saturating_sub(1));
        let h_step = height_step
            .unwrap_or(config.default_height_step)
            .min(config.height_steps.len().saturating_sub(1));

        if floating {
            self.floating.push(id);
            self.focused = Some(id);
            return;
        }

        let was_empty = self.cells.is_empty();
        if matches!(config.new_window_placement, NewWindowPlacement::StackInCurrentCell) {
            if let Some(focused_id) = self.focused {
                if let Some(&coord) = self.loc.get(&focused_id) {
                    if let Some(cell) = self.cells.get_mut(&coord) {
                        cell.windows.push(id);
                        cell.active_idx = cell.windows.len() - 1;
                        self.loc.insert(id, coord);
                        self.focused = Some(id);
                        self.follow_focused();
                        return;
                    }
                }
            }
        }

        let target_coord = self.find_insertion_coord(config.new_window_placement);
        self.cells
            .insert(target_coord, Cell::new(id, w_step, h_step));
        self.loc.insert(id, target_coord);
        self.focused = Some(id);
        self.camera_coord = target_coord;
        self.camera_pan_offset = (0.0, 0.0);
        self.camera_manual_pan = false;
        if was_empty {
            self.camera_anim.reset();
        }
    }

    /// Inserts or restores a window at an explicit saved coordinate (used by state persistence).
    pub fn insert_window_at(
        &mut self,
        id: WindowId,
        coord: Coord,
        floating: bool,
        width_step: usize,
        height_step: usize,
        config: &LayoutConfig,
    ) {
        if self.contains_window(id) {
            return;
        }
        let w_step = width_step.min(config.width_steps.len().saturating_sub(1));
        let h_step = height_step.min(config.height_steps.len().saturating_sub(1));
        if floating {
            self.floating.push(id);
            self.focused = Some(id);
            return;
        }
        let was_empty = self.cells.is_empty();
        if let Some(cell) = self.cells.get_mut(&coord) {
            cell.windows.push(id);
            cell.active_idx = cell.windows.len() - 1;
        } else {
            self.cells.insert(coord, Cell::new(id, w_step, h_step));
        }
        self.loc.insert(id, coord);
        self.focused = Some(id);
        self.camera_coord = coord;
        self.camera_pan_offset = (0.0, 0.0);
        self.camera_manual_pan = false;
        if was_empty {
            self.camera_anim.reset();
        }
    }

    pub fn apply_restored_snap(&mut self, id: WindowId, snap: CellSnap) {
        if let Some(&coord) = self.loc.get(&id) {
            if let Some(cell) = self.cells.get_mut(&coord) {
                cell.apply_snap(CellSnap {
                    custom_width_frac: snap.custom_width_frac.map(|v| v.clamp(0.15, 1.0)),
                    custom_height_frac: snap.custom_height_frac.map(|v| v.clamp(0.15, 1.0)),
                    ..snap
                });
            }
        }
    }

    /// Record an OS snap / manual border-resize fraction onto a window's cell so it stays
    /// sticky across focus switches, sibling snaps, Win+O overview, and restarts.
    pub fn set_custom_snap(
        &mut self,
        id: WindowId,
        width_frac: f64,
        height_frac: f64,
        width_steps: &[f64],
        height_steps: &[f64],
    ) -> bool {
        let Some(&coord) = self.loc.get(&id) else {
            return false;
        };
        let Some(cell) = self.cells.get_mut(&coord) else {
            return false;
        };
        let wf = width_frac.clamp(0.15, 1.0);
        let hf = height_frac.clamp(0.15, 1.0);
        let nearest_step = |steps: &[f64], target: f64| -> usize {
            steps
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    ((**a - target).abs()).total_cmp(&(**b - target).abs())
                })
                .map(|(i, _)| i)
                .unwrap_or(0)
        };
        cell.width_step = nearest_step(width_steps, wf);
        cell.height_step = nearest_step(height_steps, hf);
        cell.custom_width_frac = Some(wf);
        cell.custom_height_frac = Some(hf);
        cell.maximized = false;
        cell.manual_override = true;
        true
    }

    pub fn cell_rect_for(&self, id: WindowId) -> Option<IRect> {
        let coord = self.loc.get(&id)?;
        self.last_cell_rects.get(coord).copied()
    }

    pub fn set_maximized(&mut self, id: WindowId, maximized: bool) -> bool {
        let Some(&coord) = self.loc.get(&id) else {
            return false;
        };
        let Some(cell) = self.cells.get_mut(&coord) else {
            return false;
        };
        cell.maximized = maximized;
        cell.manual_override = true;
        true
    }

    fn find_insertion_coord(&mut self, policy: NewWindowPlacement) -> Coord {
        let anchor = self
            .focused
            .and_then(|id| self.loc.get(&id).copied())
            .or_else(|| self.cells.keys().next().copied());

        let Some(anchor) = anchor else {
            return Coord::ZERO;
        };

        match policy {
            NewWindowPlacement::RightOfFocus | NewWindowPlacement::StackInCurrentCell => {
                let mut col = anchor.col + 1;
                while self.cells.contains_key(&Coord::new(col, anchor.row)) {
                    col += 1;
                }
                Coord::new(col, anchor.row)
            }
            NewWindowPlacement::Neighbor4Way => {
                // Omalattice rule: check direct adjacent neighbors right -> down -> left -> up.
                for dir in [Direction::Right, Direction::Down, Direction::Left, Direction::Up] {
                    let cand = anchor.step(dir);
                    if !self.cells.contains_key(&cand) {
                        return cand;
                    }
                }
                // All 4 neighbors are occupied: shift existing cells beyond anchor right by 1 in this row.
                let dest = anchor.step(Direction::Right);
                self.shift_row_right_from(anchor.row, dest.col);
                dest
            }
            NewWindowPlacement::BelowFocus => {
                let mut row = anchor.row + 1;
                while self.cells.contains_key(&Coord::new(anchor.col, row)) {
                    row += 1;
                }
                Coord::new(anchor.col, row)
            }
            NewWindowPlacement::RightmostInRow => {
                let max_col = self
                    .cells
                    .keys()
                    .filter(|c| c.row == anchor.row)
                    .map(|c| c.col)
                    .max()
                    .unwrap_or(anchor.col);
                Coord::new(max_col + 1, anchor.row)
            }
        }
    }

    fn shift_row_right_from(&mut self, row: i32, start_col: i32) {
        let mut to_move: Vec<Coord> = self
            .cells
            .keys()
            .copied()
            .filter(|c| c.row == row && c.col >= start_col)
            .collect();
        to_move.sort_by_key(|c| -c.col);
        for old_coord in to_move {
            if let Some(cell) = self.cells.remove(&old_coord) {
                let new_coord = Coord::new(old_coord.col + 1, old_coord.row);
                for &wid in &cell.windows {
                    self.loc.insert(wid, new_coord);
                }
                if self.camera_coord == old_coord {
                    self.camera_coord = new_coord;
                }
                self.cells.insert(new_coord, cell);
            }
        }
    }

    pub fn shift_cols_right_from(&mut self, start_col: i32) {
        let mut to_move: Vec<Coord> = self
            .cells
            .keys()
            .copied()
            .filter(|c| c.col >= start_col)
            .collect();
        to_move.sort_by_key(|c| -c.col);
        for old_coord in to_move {
            if let Some(cell) = self.cells.remove(&old_coord) {
                let new_coord = Coord::new(old_coord.col + 1, old_coord.row);
                for &wid in &cell.windows {
                    self.loc.insert(wid, new_coord);
                }
                if self.camera_coord == old_coord {
                    self.camera_coord = new_coord;
                }
                self.cells.insert(new_coord, cell);
            }
        }
    }

    pub fn remove_window(&mut self, id: WindowId) -> bool {
        self.remove_window_with_compact(id, false)
    }

    pub fn remove_window_with_compact(&mut self, id: WindowId, compact_empty: bool) -> bool {
        let mut removed = false;
        let mut removed_coord = None;
        if self.fullscreen == Some(id) {
            self.fullscreen = None;
        }
        if let Some(pos) = self.floating.iter().position(|&w| w == id) {
            self.floating.remove(pos);
            removed = true;
        } else if let Some(coord) = self.loc.remove(&id) {
            if let Some(cell) = self.cells.get_mut(&coord) {
                if let Some(idx) = cell.windows.iter().position(|&w| w == id) {
                    cell.windows.remove(idx);
                    if cell.windows.is_empty() {
                        self.cells.remove(&coord);
                        removed_coord = Some(coord);
                    } else if cell.active_idx >= cell.windows.len() {
                        cell.active_idx = cell.windows.len() - 1;
                    }
                }
            }
            removed = true;
        }

        if !removed {
            return false;
        }

        if compact_empty {
            if let Some(c) = removed_coord {
                self.compact_empty_bands_at(c, true);
            }
        }

        if self.focused == Some(id) {
            self.focused = self.pick_fallback_focus(self.camera_coord);
            if let Some(new_focus) = self.focused {
                if let Some(&coord) = self.loc.get(&new_focus) {
                    self.camera_coord = coord;
                    self.camera_pan_offset = (0.0, 0.0);
                    self.camera_manual_pan = false;
                }
            }
        }
        true
    }

    fn pick_fallback_focus(&self, origin: Coord) -> Option<WindowId> {
        if let Some(cell) = self.cells.get(&origin) {
            return Some(cell.active_window());
        }
        self.cells
            .iter()
            .min_by_key(|(c, _)| {
                let dx = (c.col - origin.col).abs();
                let dy = (c.row - origin.row).abs();
                (dx + dy, dy, dx, c.col, c.row)
            })
            .map(|(_, cell)| cell.active_window())
            .or_else(|| self.floating.last().copied())
    }

    pub fn set_focus(&mut self, id: WindowId) -> bool {
        if let Some(&coord) = self.loc.get(&id) {
            let changed = self.focused != Some(id);
            if let Some(cell) = self.cells.get_mut(&coord) {
                if let Some(idx) = cell.windows.iter().position(|&w| w == id) {
                    cell.active_idx = idx;
                }
            }
            self.focused = Some(id);
            if changed {
                self.camera_coord = coord;
                self.camera_pan_offset = (0.0, 0.0);
                self.camera_manual_pan = false;
            }
            return true;
        }
        if self.floating.contains(&id) {
            self.focused = Some(id);
            return true;
        }
        false
    }

    pub fn follow_focused(&mut self) -> bool {
        if let Some(id) = self.focused {
            if let Some(&coord) = self.loc.get(&id) {
                self.camera_coord = coord;
                self.camera_pan_offset = (0.0, 0.0);
                self.camera_manual_pan = false;
                return true;
            }
        }
        false
    }

    pub fn focus_direction(&mut self, dir: Direction) -> Option<WindowId> {
        let focused_id = self.focused?;
        let origin = *self.loc.get(&focused_id)?;
        if let Some(cell) = self.cells.get_mut(&origin) {
            if cell.windows.len() > 1 {
                match dir {
                    Direction::Left | Direction::Up if cell.active_idx > 0 => {
                        cell.active_idx -= 1;
                        let target = cell.active_window();
                        self.focused = Some(target);
                        self.follow_focused();
                        return Some(target);
                    }
                    Direction::Right | Direction::Down if cell.active_idx + 1 < cell.windows.len() => {
                        cell.active_idx += 1;
                        let target = cell.active_window();
                        self.focused = Some(target);
                        self.follow_focused();
                        return Some(target);
                    }
                    _ => {}
                }
            }
        }

        let target_coord = self.best_directional_cell(origin, dir)?;
        let cell = self.cells.get_mut(&target_coord)?;
        match dir {
            Direction::Up => cell.active_idx = cell.windows.len().saturating_sub(1),
            Direction::Down => cell.active_idx = 0,
            _ => {}
        }
        let target_id = cell.active_window();
        self.focused = Some(target_id);
        self.camera_coord = target_coord;
        self.camera_pan_offset = (0.0, 0.0);
        self.camera_manual_pan = false;
        Some(target_id)
    }

    fn best_directional_cell(&self, origin: Coord, dir: Direction) -> Option<Coord> {
        self.cells
            .keys()
            .copied()
            .filter(|&cand| cand != origin)
            .filter_map(|cand| {
                let dc = cand.col - origin.col;
                let dr = cand.row - origin.row;
                let valid = match dir {
                    Direction::Left => dc < 0,
                    Direction::Right => dc > 0,
                    Direction::Up => dr < 0,
                    Direction::Down => dr > 0,
                };
                if !valid {
                    return None;
                }
                let (along, across) = match dir {
                    Direction::Left | Direction::Right => (dc.abs(), dr.abs()),
                    Direction::Up | Direction::Down => (dr.abs(), dc.abs()),
                };
                let aligned = if across == 0 { 0 } else { 1 };
                let manhattan = dc.abs() + dr.abs();
                Some(((aligned, across, along, manhattan, cand.col, cand.row), cand))
            })
            .min_by_key(|(metrics, _)| *metrics)
            .map(|(_, coord)| coord)
    }

    /// Move the focused window in `dir`.
    /// - Inside a multi-window cell, Up/Down reorders within the stack before exiting.
    /// - Moving into an occupied cell swaps active windows (and size presets for 1-window cells).
    /// - Moving into an empty cell moves the window there (preserving holes when enabled,
    ///   or collapsing emptied columns like Niri when `preserve_holes` is false).
    pub fn move_focused(&mut self, dir: Direction, preserve_holes: bool) -> bool {
        self.move_focused_with_compact(dir, preserve_holes, false)
    }

    pub fn move_focused_with_compact(
        &mut self,
        dir: Direction,
        preserve_holes: bool,
        compact_rows: bool,
    ) -> bool {
        let Some(focused_id) = self.focused else {
            return false;
        };
        let Some(&origin) = self.loc.get(&focused_id) else {
            return false;
        };

        if let Some(cell) = self.cells.get_mut(&origin) {
            if cell.windows.len() > 1 {
                let idx = cell.active_idx;
                match dir {
                    Direction::Up if idx > 0 => {
                        cell.windows.swap(idx, idx - 1);
                        cell.active_idx = idx - 1;
                        self.follow_focused();
                        return true;
                    }
                    Direction::Down if idx + 1 < cell.windows.len() => {
                        cell.windows.swap(idx, idx + 1);
                        cell.active_idx = idx + 1;
                        self.follow_focused();
                        return true;
                    }
                    _ => {}
                }
            }
        }

        let dest = origin.step(dir);
        if self.cells.contains_key(&dest) {
            // Occupied destination: swap focused window with destination's active window.
            let (orig_single, orig_snap) = {
                let c = &self.cells[&origin];
                (c.windows.len() == 1, c.snap_state())
            };
            let (dest_single, dest_snap, other_id) = {
                let c = &self.cells[&dest];
                (c.windows.len() == 1, c.snap_state(), c.active_window())
            };

            if let Some(c) = self.cells.get_mut(&origin) {
                let idx = c.active_idx;
                c.windows[idx] = other_id;
                if orig_single && dest_single {
                    c.apply_snap(dest_snap);
                }
            }
            if let Some(c) = self.cells.get_mut(&dest) {
                let idx = c.active_idx;
                c.windows[idx] = focused_id;
                if orig_single && dest_single {
                    c.apply_snap(orig_snap);
                }
            }
            self.loc.insert(focused_id, dest);
            self.loc.insert(other_id, origin);
        } else {
            // Empty destination: move focused window out into `dest`.
            let snap = {
                let Some(orig) = self.cells.get_mut(&origin) else {
                    return false;
                };
                let snap = orig.snap_state();
                let idx = orig.active_idx;
                orig.windows.remove(idx);
                if orig.windows.is_empty() {
                    self.cells.remove(&origin);
                } else if orig.active_idx >= orig.windows.len() {
                    orig.active_idx = orig.windows.len() - 1;
                }
                snap
            };

            let mut new_cell = Cell::new(focused_id, snap.width_step, snap.height_step);
            new_cell.apply_snap(snap);
            self.cells.insert(dest, new_cell);
            self.loc.insert(focused_id, dest);

            if !preserve_holes || compact_rows {
                self.compact_empty_bands_at(origin, compact_rows);
            }
        }

        if let Some(&actual_dest) = self.loc.get(&focused_id) {
            self.camera_coord = actual_dest;
        } else {
            self.camera_coord = dest;
        }
        self.camera_pan_offset = (0.0, 0.0);
        self.camera_manual_pan = false;
        true
    }

    pub fn compact_empty_bands_at(&mut self, vacated: Coord, compact_rows: bool) {
        if !self.cells.keys().any(|c| c.col == vacated.col) {
            self.collapse_empty_axis(true, vacated.col);
        }
        if compact_rows && !self.cells.keys().any(|c| c.row == vacated.row) {
            self.collapse_empty_axis(false, vacated.row);
        }
    }

    fn collapse_empty_axis(&mut self, horizontal: bool, empty_idx: i32) {
        let old_cells = std::mem::take(&mut self.cells);
        for (coord, cell) in old_cells {
            let new_coord = if horizontal {
                let c = if coord.col > empty_idx { coord.col - 1 } else { coord.col };
                Coord::new(c, coord.row)
            } else {
                let r = if coord.row > empty_idx { coord.row - 1 } else { coord.row };
                Coord::new(coord.col, r)
            };
            for &id in &cell.windows {
                self.loc.insert(id, new_coord);
            }
            self.cells.insert(new_coord, cell);
        }
        if horizontal && self.camera_coord.col > empty_idx {
            self.camera_coord.col -= 1;
        } else if !horizontal && self.camera_coord.row > empty_idx {
            self.camera_coord.row -= 1;
        }
    }

    pub fn pan_grid(&mut self, dir: Direction) -> bool {
        self.camera_coord = self.camera_coord.step(dir);
        self.camera_pan_offset = (0.0, 0.0);
        self.camera_manual_pan = true;
        true
    }

    pub fn pan_pixels(&mut self, dx: f64, dy: f64) -> bool {
        // Elastic resistance when panning past grid bounds so boundary spring-back stays gentle.
        let mut scale_x = 1.0;
        let mut scale_y = 1.0;
        if let Some(vp) = self.last_viewport {
            let half_w = vp.w as f64 * 0.55;
            let half_h = vp.h as f64 * 0.55;
            if let Some((min_x, max_x, min_y, max_y)) = self.screen_bounds_of_cells() {
                let center_x = vp.x as f64 + vp.w as f64 / 2.0;
                let center_y = vp.y as f64 + vp.h as f64 / 2.0;
                if (min_x - dx > center_x - half_w && dx < 0.0)
                    || (max_x - dx < center_x + half_w && dx > 0.0)
                {
                    scale_x = 0.35;
                }
                if (min_y - dy > center_y - half_h && dy < 0.0)
                    || (max_y - dy < center_y + half_h && dy > 0.0)
                {
                    scale_y = 0.35;
                }
            }
        }
        self.camera_pan_offset.0 += dx * scale_x;
        self.camera_pan_offset.1 += dy * scale_y;
        self.camera_manual_pan = true;
        true
    }

    fn screen_bounds_of_cells(&self) -> Option<(f64, f64, f64, f64)> {
        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for r in self.last_cell_rects.values() {
            min_x = min_x.min(r.x as f64);
            max_x = max_x.max(r.right() as f64);
            min_y = min_y.min(r.y as f64);
            max_y = max_y.max(r.bottom() as f64);
        }
        if min_x.is_finite() {
            Some((min_x, max_x, min_y, max_y))
        } else {
            None
        }
    }

    /// Commits a continuous 2D touchpad/mouse pixel pan onto the window with the largest
    /// visibility gain in the pan direction (or snaps back cleanly if travel was < 8px).
    pub fn finish_pixel_pan(&mut self, vx: f64, vy: f64) -> Option<WindowId> {
        let (ox, oy) = self.camera_pan_offset;
        let travel_x = ox + vx * 0.10;
        let travel_y = oy + vy * 0.10;

        if ox.abs() < 8.0 && oy.abs() < 8.0 && vx.hypot(vy) < 40.0 {
            self.camera_pan_offset = (0.0, 0.0);
            self.camera_manual_pan = false;
            return None;
        }

        let origin = self
            .focused
            .and_then(|id| self.loc.get(&id).copied())
            .unwrap_or(self.camera_coord);

        let axis_horiz = travel_x.abs() >= 2.0 * travel_y.abs();
        let axis_vert = travel_y.abs() >= 2.0 * travel_x.abs();
        let vp = self.last_viewport.unwrap_or(IRect::new(0, 0, 1920, 1080));

        let pick_best = |only_same_axis: bool| -> Option<Coord> {
            self.cells
                .keys()
                .copied()
                .filter(|&cand| cand != origin)
                .filter(|&cand| {
                    let dc = cand.col - origin.col;
                    let dr = cand.row - origin.row;
                    if axis_horiz {
                        let sign_ok = (dc as f64) * travel_x > 0.0;
                        sign_ok && (!only_same_axis || dr == 0)
                    } else if axis_vert {
                        let sign_ok = (dr as f64) * travel_y > 0.0;
                        sign_ok && (!only_same_axis || dc == 0)
                    } else if only_same_axis {
                        (dc as f64) * travel_x > 0.0 && (dr as f64) * travel_y > 0.0
                    } else {
                        (dc as f64) * travel_x + (dr as f64) * travel_y > 0.0
                    }
                })
                .min_by_key(|&cand| {
                    let vis_area = self
                        .last_cell_rects
                        .get(&cand)
                        .map(|&r| {
                            let ix = (r.x.max(vp.x)..r.right().min(vp.right())).len() as i64;
                            let iy = (r.y.max(vp.y)..r.bottom().min(vp.bottom())).len() as i64;
                            ix * iy
                        })
                        .unwrap_or(0);
                    let dc = (cand.col - origin.col).abs();
                    let dr = (cand.row - origin.row).abs();
                    (-(vis_area), dc + dr, dc * dc + dr * dr)
                })
        };

        let chosen = pick_best(true).or_else(|| pick_best(false));
        self.camera_pan_offset = (0.0, 0.0);
        self.camera_manual_pan = false;

        if let Some(coord) = chosen {
            if let Some(id) = self.cells.get(&coord).map(|c| c.active_window()) {
                self.set_focus(id);
                return Some(id);
            }
        }
        None
    }

    pub fn resize_width(&mut self, delta: i32, steps_len: usize) -> bool {
        self.resize_axis(true, delta, steps_len)
    }

    pub fn resize_height(&mut self, delta: i32, steps_len: usize) -> bool {
        self.resize_axis(false, delta, steps_len)
    }

    pub fn cycle_width(&mut self, steps_len: usize) -> bool {
        self.cycle_axis(true, steps_len)
    }

    pub fn cycle_height(&mut self, steps_len: usize) -> bool {
        self.cycle_axis(false, steps_len)
    }

    fn resize_axis(&mut self, horizontal: bool, delta: i32, steps_len: usize) -> bool {
        if steps_len == 0 {
            return false;
        }
        let Some(id) = self.focused else { return false };
        let Some(&coord) = self.loc.get(&id) else { return false };
        let Some(cell) = self.cells.get_mut(&coord) else { return false };
        cell.maximized = false;
        cell.manual_override = true;
        if horizontal {
            cell.custom_width_frac = None;
        } else {
            cell.custom_height_frac = None;
        }
        let field = if horizontal {
            &mut cell.width_step
        } else {
            &mut cell.height_step
        };
        let next = (*field as isize + delta as isize).clamp(0, (steps_len - 1) as isize) as usize;
        let changed = next != *field;
        *field = next;
        changed
    }

    fn cycle_axis(&mut self, horizontal: bool, steps_len: usize) -> bool {
        if steps_len <= 1 {
            return false;
        }
        let Some(id) = self.focused else { return false };
        let Some(&coord) = self.loc.get(&id) else { return false };
        let Some(cell) = self.cells.get_mut(&coord) else { return false };
        cell.maximized = false;
        cell.manual_override = true;
        if horizontal {
            cell.custom_width_frac = None;
        } else {
            cell.custom_height_frac = None;
        }
        let field = if horizontal {
            &mut cell.width_step
        } else {
            &mut cell.height_step
        };
        *field = (*field + 1) % steps_len;
        true
    }

    /// Niri `ConsumeIntoCell`: pull an adjacent cell's active window into the focused cell's stack.
    pub fn consume_into_cell(&mut self, preferred_dir: Option<Direction>) -> bool {
        let Some(focused_id) = self.focused else { return false };
        let Some(&origin) = self.loc.get(&focused_id) else { return false };

        let candidate_dirs = match preferred_dir {
            Some(d) => vec![d],
            None => vec![Direction::Right, Direction::Left, Direction::Down, Direction::Up],
        };

        let mut donor_coord = None;
        for dir in candidate_dirs {
            if let Some(c) = self.best_directional_cell(origin, dir) {
                // Prefer immediate neighbor first, otherwise nearest in that direction.
                let step = origin.step(dir);
                if self.cells.contains_key(&step) {
                    donor_coord = Some(step);
                } else {
                    donor_coord = Some(c);
                }
                break;
            }
        }

        let Some(donor_coord) = donor_coord else {
            return false;
        };

        let pulled_id = {
            let Some(donor) = self.cells.get_mut(&donor_coord) else {
                return false;
            };
            let idx = donor.active_idx;
            let pulled = donor.windows.remove(idx);
            if donor.windows.is_empty() {
                self.cells.remove(&donor_coord);
            } else if donor.active_idx >= donor.windows.len() {
                donor.active_idx = donor.windows.len() - 1;
            }
            pulled
        };

        if let Some(target_cell) = self.cells.get_mut(&origin) {
            target_cell.windows.push(pulled_id);
            self.loc.insert(pulled_id, origin);
        }
        true
    }

    /// Niri `ExpelFromCell`: push the focused window out of a multi-window cell into an open neighbor cell.
    pub fn expel_from_cell(&mut self, dir: Direction) -> bool {
        let Some(focused_id) = self.focused else { return false };
        let Some(&origin) = self.loc.get(&focused_id) else { return false };

        let (w_step, h_step) = {
            let Some(cell) = self.cells.get_mut(&origin) else {
                return false;
            };
            if cell.windows.len() <= 1 {
                return false;
            }
            let idx = cell.active_idx;
            cell.windows.remove(idx);
            if cell.active_idx >= cell.windows.len() {
                cell.active_idx = cell.windows.len() - 1;
            }
            (cell.width_step, cell.height_step)
        };

        let mut dest = origin.step(dir);
        while self.cells.contains_key(&dest) {
            dest = dest.step(dir);
        }
        self.cells.insert(dest, Cell::new(focused_id, w_step, h_step));
        self.loc.insert(focused_id, dest);
        self.camera_coord = dest;
        self.camera_pan_offset = (0.0, 0.0);
        self.camera_manual_pan = false;
        true
    }

    pub fn toggle_stack_mode(&mut self) -> bool {
        let Some(id) = self.focused else { return false };
        let Some(&coord) = self.loc.get(&id) else { return false };
        let Some(cell) = self.cells.get_mut(&coord) else { return false };
        cell.stack_mode = match cell.stack_mode {
            StackMode::Split => StackMode::Tabbed,
            StackMode::Tabbed => StackMode::Split,
        };
        true
    }

    pub fn toggle_maximize_cell(&mut self) -> bool {
        let Some(id) = self.focused else { return false };
        let Some(&coord) = self.loc.get(&id) else { return false };
        let Some(cell) = self.cells.get_mut(&coord) else { return false };
        cell.maximized = !cell.maximized;
        self.follow_focused();
        true
    }

    pub fn toggle_floating(&mut self, id: WindowId, config: &LayoutConfig) -> Option<bool> {
        if let Some(pos) = self.floating.iter().position(|&w| w == id) {
            self.floating.remove(pos);
            self.insert_window(id, false, None, None, config);
            Some(false)
        } else if self.loc.contains_key(&id) {
            self.remove_window(id);
            self.floating.push(id);
            self.focused = Some(id);
            Some(true)
        } else {
            None
        }
    }

    pub fn jump_to(&mut self, coord: Coord, window_id: Option<WindowId>) -> Option<WindowId> {
        self.camera_anim.reset();
        if let Some(id) = window_id {
            if self.set_focus(id) {
                self.follow_focused();
                return Some(id);
            }
        }
        if let Some(cell) = self.cells.get(&coord) {
            let id = cell.active_window();
            self.set_focus(id);
            self.follow_focused();
            return Some(id);
        }
        self.camera_coord = coord;
        self.camera_pan_offset = (0.0, 0.0);
        self.camera_manual_pan = true;
        None
    }

    /// Directly place or swap `window_id` into target 2D grid coordinate `dest` (used by mouse drag & drop in Overview).
    pub fn place_window_at(
        &mut self,
        window_id: WindowId,
        dest: Coord,
        stack: bool,
        preserve_holes: bool,
        config: &LayoutConfig,
    ) -> bool {
        // Support dragging a floating window straight into a grid cell.
        if let Some(pos) = self.floating.iter().position(|&w| w == window_id) {
            self.floating.remove(pos);
            let w_step = config.default_width_step.min(config.width_steps.len().saturating_sub(1));
            let h_step = config.default_height_step.min(config.height_steps.len().saturating_sub(1));
            if let Some(dest_cell) = self.cells.get_mut(&dest) {
                dest_cell.windows.push(window_id);
                dest_cell.active_idx = dest_cell.windows.len() - 1;
                dest_cell.stack_mode = StackMode::Split;
            } else {
                self.cells.insert(dest, Cell::new(window_id, w_step, h_step));
            }
            self.loc.insert(window_id, dest);
            self.focused = Some(window_id);
            self.camera_coord = dest;
            self.camera_pan_offset = (0.0, 0.0);
            self.camera_manual_pan = false;
            return true;
        }

        let Some(&origin) = self.loc.get(&window_id) else {
            return false;
        };
        if origin == dest {
            return false;
        }

        if self.cells.contains_key(&dest) {
            if stack {
                // Remove from origin cell and push onto dest cell's vertical stack.
                let vacated_empty = {
                    let Some(orig) = self.cells.get_mut(&origin) else {
                        return false;
                    };
                    if let Some(idx) = orig.windows.iter().position(|&w| w == window_id) {
                        orig.windows.remove(idx);
                        if orig.windows.is_empty() {
                            self.cells.remove(&origin);
                            true
                        } else {
                            if orig.active_idx >= orig.windows.len() {
                                orig.active_idx = orig.windows.len() - 1;
                            }
                            false
                        }
                    } else {
                        false
                    }
                };
                if let Some(dest_cell) = self.cells.get_mut(&dest) {
                    dest_cell.windows.push(window_id);
                    dest_cell.active_idx = dest_cell.windows.len() - 1;
                    dest_cell.stack_mode = StackMode::Split;
                }
                self.loc.insert(window_id, dest);
                if vacated_empty && (!preserve_holes || config.auto_compact_empty) {
                    self.compact_empty_bands_at(origin, config.auto_compact_empty);
                }
            } else {
                // Swap `window_id` in `origin` with active window of `dest`.
                let orig_idx = self.cells[&origin]
                    .windows
                    .iter()
                    .position(|&w| w == window_id)
                    .unwrap_or(0);
                let orig_single = self.cells[&origin].windows.len() == 1;
                let orig_snap = self.cells[&origin].snap_state();

                let dest_single = self.cells[&dest].windows.len() == 1;
                let dest_snap = self.cells[&dest].snap_state();
                let other_id = self.cells[&dest].active_window();

                if let Some(c) = self.cells.get_mut(&origin) {
                    c.windows[orig_idx] = other_id;
                    c.active_idx = orig_idx;
                    if orig_single && dest_single {
                        c.apply_snap(dest_snap);
                    }
                }
                if let Some(c) = self.cells.get_mut(&dest) {
                    let idx = c.active_idx;
                    c.windows[idx] = window_id;
                    if orig_single && dest_single {
                        c.apply_snap(orig_snap);
                    }
                }
                self.loc.insert(window_id, dest);
                self.loc.insert(other_id, origin);
            }
        } else {
            // Move into empty `dest`.
            let (snap, anim_w, anim_h) = {
                let Some(orig) = self.cells.get_mut(&origin) else {
                    return false;
                };
                let snap = orig.snap_state();
                let anim_w = orig.anim_w;
                let anim_h = orig.anim_h;
                if let Some(idx) = orig.windows.iter().position(|&w| w == window_id) {
                    orig.windows.remove(idx);
                }
                if orig.windows.is_empty() {
                    self.cells.remove(&origin);
                } else if orig.active_idx >= orig.windows.len() {
                    orig.active_idx = orig.windows.len() - 1;
                }
                (snap, anim_w, anim_h)
            };

            let mut new_cell = Cell::new(window_id, snap.width_step, snap.height_step);
            new_cell.apply_snap(snap);
            new_cell.anim_w = anim_w;
            new_cell.anim_h = anim_h;
            self.cells.insert(dest, new_cell);
            self.loc.insert(window_id, dest);

            if !preserve_holes || config.auto_compact_empty {
                self.compact_empty_bands_at(origin, config.auto_compact_empty);
            }
        }

        let actual_dest = self.loc.get(&window_id).copied().unwrap_or(dest);
        self.focused = Some(window_id);
        self.camera_coord = actual_dest;
        self.camera_pan_offset = (0.0, 0.0);
        self.camera_manual_pan = false;
        true
    }

    /// Split `window_id` horizontally with `target_win_id`, snapping both into the same view side-by-side (50% each).
    pub fn split_window_horizontal(
        &mut self,
        window_id: WindowId,
        target_win_id: WindowId,
        on_left: bool,
        config: &LayoutConfig,
    ) -> bool {
        if window_id == target_win_id {
            return false;
        }
        if !self.contains_window(window_id) || !self.contains_window(target_win_id) {
            return false;
        }

        // 1. Remove window_id from wherever it currently is.
        if let Some(pos) = self.floating.iter().position(|&w| w == window_id) {
            self.floating.remove(pos);
        } else if let Some(coord) = self.loc.remove(&window_id) {
            if let Some(cell) = self.cells.get_mut(&coord) {
                if let Some(idx) = cell.windows.iter().position(|&w| w == window_id) {
                    cell.windows.remove(idx);
                    if cell.windows.is_empty() {
                        self.cells.remove(&coord);
                        if config.auto_compact_empty {
                            self.compact_empty_bands_at(coord, true);
                        }
                    } else if cell.active_idx >= cell.windows.len() {
                        cell.active_idx = cell.windows.len() - 1;
                    }
                }
            }
        }

        // 2. Locate target_win_id now.
        let Some(&target_coord) = self.loc.get(&target_win_id) else {
            return false;
        };

        // 3. Add window_id to target_coord's cell side-by-side horizontally.
        if let Some(cell) = self.cells.get_mut(&target_coord) {
            let target_idx = cell
                .windows
                .iter()
                .position(|&w| w == target_win_id)
                .unwrap_or(0);
            let insert_idx = if on_left { target_idx } else { target_idx + 1 };
            cell.windows.insert(insert_idx, window_id);
            cell.active_idx = insert_idx;
            cell.stack_mode = StackMode::Split;
            cell.custom_width_frac = None;
            cell.custom_height_frac = None;
        } else {
            let w_step = config.default_width_step.min(config.width_steps.len().saturating_sub(1));
            let h_step = config.default_height_step.min(config.height_steps.len().saturating_sub(1));
            self.cells.insert(target_coord, Cell::new(window_id, w_step, h_step));
        }

        self.loc.insert(window_id, target_coord);
        self.focused = Some(window_id);
        self.camera_coord = target_coord;
        self.camera_pan_offset = (0.0, 0.0);
        self.camera_manual_pan = false;
        true
    }

    /// Automatically repair any cells containing multiple vertically-stacked windows,
    /// unpacking them into clean horizontal side-by-side columns.
    pub fn unpack_stacked_cells(&mut self, config: &LayoutConfig) -> bool {
        let multi_coords: Vec<Coord> = self
            .cells
            .iter()
            .filter(|(_, c)| c.windows.len() > 1)
            .map(|(&coord, _)| coord)
            .collect();
        if multi_coords.is_empty() {
            return false;
        }

        let w_step = config.default_width_step.min(config.width_steps.len().saturating_sub(1));
        let h_step = config.default_height_step.min(config.height_steps.len().saturating_sub(1));

        for coord in multi_coords {
            let Some(cell) = self.cells.get_mut(&coord) else {
                continue;
            };
            let remaining: Vec<WindowId> = cell.windows.drain(1..).collect();
            cell.active_idx = 0;
            cell.custom_width_frac = Some(0.5);
            cell.custom_height_frac = Some(1.0);

            for (idx, win_id) in remaining.into_iter().enumerate() {
                let new_col = coord.col + 1 + idx as i32;
                self.shift_cols_right_from(new_col);
                let new_coord = Coord::new(new_col, coord.row);
                let mut new_cell = Cell::new(win_id, w_step, h_step);
                new_cell.custom_width_frac = Some(0.5);
                new_cell.custom_height_frac = Some(1.0);
                self.cells.insert(new_coord, new_cell);
                self.loc.insert(win_id, new_coord);
            }
        }
        true
    }

    /// Reset all cells to clean preset geometries (clearing manual overrides and snaps).
    pub fn retile(&mut self, config: &LayoutConfig) {
        let def_w = config.default_width_step.min(config.width_steps.len().saturating_sub(1));
        let def_h = config.default_height_step.min(config.height_steps.len().saturating_sub(1));
        for cell in self.cells.values_mut() {
            cell.custom_width_frac = None;
            cell.custom_height_frac = None;
            cell.manual_override = false;
            cell.maximized = false;
            cell.height_step = def_h;
            if cell.windows.len() <= 1 {
                cell.width_step = def_w;
            }
        }
    }

    /// Recompute band geometry, update camera spring target, step spring physics, and return window placements.
    ///
    /// Returns `(placements, still_animating)`.
    #[allow(clippy::too_many_arguments)]
    pub fn compute_placements(
        &mut self,
        area: IRect,
        other_monitors: &[IRect],
        config: &LayoutConfig,
        animate: bool,
        animate_resize: bool,
        dt_secs: f64,
        stiffness: f64,
        damping_ratio: f64,
        snap_epsilon: f64,
    ) -> (Vec<WindowPlacement>, bool) {
        if self.cells.is_empty() {
            self.last_anchor = None;
            return (Vec::new(), false);
        }

        let area_w = area.w.max(1) as f64;
        let area_h = area.h.max(1) as f64;
        let peek_x = (config.peek_x as f64).clamp(0.0, ((area_w - 1.0) / 2.0).max(0.0));
        let peek_y = (config.peek_y as f64).clamp(0.0, ((area_h - 1.0) / 2.0).max(0.0));
        let focused_peek_x = (config.focused_peek_x as f64).clamp(0.0, ((area_w - 1.0) / 2.0).max(0.0));
        let focused_peek_y = (config.focused_peek_y as f64).clamp(0.0, ((area_h - 1.0) / 2.0).max(0.0));
        let max_cell_w = (area_w - peek_x * 2.0).max(1.0);
        let max_cell_h = (area_h - peek_y * 2.0).max(1.0);
        let gap_x = config.gap_x.max(0) as f64;
        let gap_y = config.gap_y.max(0) as f64;

        let single_cell_expand = config.expand_single_window && self.cells.len() == 1;
        let margin = config.single_window_margin.max(0) as f64;
        let focused_coord = self.focused.and_then(|fid| self.loc.get(&fid).copied());

        let default_w = max_cell_w * config.width_steps[config.default_width_step];
        let default_h = max_cell_h * config.height_steps[config.default_height_step];

        let mut min_col = self.camera_coord.col;
        let mut max_col = self.camera_coord.col;
        let mut min_row = self.camera_coord.row;
        let mut max_row = self.camera_coord.row;

        let mut cell_sizes: HashMap<Coord, (f64, f64)> = HashMap::with_capacity(self.cells.len());
        let mut col_widths: HashMap<i32, f64> = HashMap::new();
        let mut row_heights: HashMap<i32, f64> = HashMap::new();
        let mut size_animating = false;
        let cam_initialized = self.camera_anim.initialized();

        for (&coord, cell) in &mut self.cells {
            let wf = cell
                .custom_width_frac
                .unwrap_or_else(|| {
                    config.width_steps[cell.width_step.min(config.width_steps.len() - 1)]
                })
                .clamp(0.10, 1.0);
            let hf = cell
                .custom_height_frac
                .unwrap_or_else(|| {
                    config.height_steps[cell.height_step.min(config.height_steps.len() - 1)]
                })
                .clamp(0.10, 1.0);
            let preset_size = (max_cell_w * wf, max_cell_h * hf);

            let (target_w, target_h) = if cell.maximized
                || (single_cell_expand && !cell.manual_override)
            {
                ((area_w - margin * 2.0).max(1.0), (area_h - margin * 2.0).max(1.0))
            } else if config.auto_expand_focused
                && focused_coord == Some(coord)
                && !cell.manual_override
            {
                (
                    (area_w - focused_peek_x * 2.0).max(1.0),
                    (area_h - focused_peek_y * 2.0).max(1.0),
                )
            } else {
                preset_size
            };

            let (w, h) = if animate && animate_resize {
                if !cell.anim_w.initialized {
                    cell.anim_w.snap_to(if !cam_initialized { target_w } else { preset_size.0 });
                }
                if !cell.anim_h.initialized {
                    cell.anim_h.snap_to(if !cam_initialized { target_h } else { preset_size.1 });
                }
                cell.anim_w.set_target(target_w, true);
                cell.anim_h.set_target(target_h, true);
                let aw = cell.anim_w.step(dt_secs, stiffness, damping_ratio, snap_epsilon);
                let ah = cell.anim_h.step(dt_secs, stiffness, damping_ratio, snap_epsilon);
                size_animating |= aw || ah;
                (cell.anim_w.current.max(1.0), cell.anim_h.current.max(1.0))
            } else {
                cell.anim_w.snap_to(target_w);
                cell.anim_h.snap_to(target_h);
                (target_w, target_h)
            };

            cell_sizes.insert(coord, (w, h));
            col_widths
                .entry(coord.col)
                .and_modify(|m| *m = m.max(w))
                .or_insert(w);
            row_heights
                .entry(coord.row)
                .and_modify(|m| *m = m.max(h))
                .or_insert(h);
            min_col = min_col.min(coord.col);
            max_col = max_col.max(coord.col);
            min_row = min_row.min(coord.row);
            max_row = max_row.max(coord.row);
        }

        // Include origin (0, 0) in band span so world coordinate (0, 0) is stable.
        min_col = min_col.min(0);
        max_col = max_col.max(0);
        min_row = min_row.min(0);
        max_row = max_row.max(0);

        for c in min_col..=max_col {
            col_widths.entry(c).or_insert(default_w);
        }
        for r in min_row..=max_row {
            row_heights.entry(r).or_insert(default_h);
        }

        // Compute world-space band centers anchored at (col=0, row=0) -> (0.0, 0.0).
        let mut col_centers: HashMap<i32, f64> = HashMap::new();
        col_centers.insert(0, 0.0);
        for c in 1..=max_col {
            let prev = col_centers[&(c - 1)];
            col_centers.insert(c, prev + col_widths[&(c - 1)] / 2.0 + gap_x + col_widths[&c] / 2.0);
        }
        for c in (min_col..0).rev() {
            let next = col_centers[&(c + 1)];
            col_centers.insert(c, next - col_widths[&(c + 1)] / 2.0 - gap_x - col_widths[&c] / 2.0);
        }

        let mut row_centers: HashMap<i32, f64> = HashMap::new();
        row_centers.insert(0, 0.0);
        for r in 1..=max_row {
            let prev = row_centers[&(r - 1)];
            row_centers.insert(r, prev + row_heights[&(r - 1)] / 2.0 + gap_y + row_heights[&r] / 2.0);
        }
        for r in (min_row..0).rev() {
            let next = row_centers[&(r + 1)];
            row_centers.insert(r, next - row_heights[&(r + 1)] / 2.0 - gap_y - row_heights[&r] / 2.0);
        }

        let anchor_x = col_centers[&self.camera_coord.col];
        let anchor_y = row_centers[&self.camera_coord.row];

        // Keep screen position locked when an off-screen column/row changes size without camera change,
        // or smoothly preserve old screen position when transitioning to a new camera coord.
        if let Some((prev_coord, prev_x, prev_y)) = self.last_anchor {
            if prev_coord == self.camera_coord {
                self.camera_anim.shift_world(anchor_x - prev_x, anchor_y - prev_y);
            } else if let (Some(&new_of_old_x), Some(&new_of_old_y)) =
                (col_centers.get(&prev_coord.col), row_centers.get(&prev_coord.row))
            {
                self.camera_anim
                    .shift_world(new_of_old_x - prev_x, new_of_old_y - prev_y);
            }
        }
        self.last_anchor = Some((self.camera_coord, anchor_x, anchor_y));

        let (target_cam_x, target_cam_y) = if self.camera_manual_pan
            || matches!(config.focus_centering, FocusCentering::Always)
            || !self.camera_anim.initialized()
        {
            (
                anchor_x + self.camera_pan_offset.0,
                anchor_y + self.camera_pan_offset.1,
            )
        } else {
            // FocusCentering::OnOverflow: keep current target unless focused cell + peek overflows viewport.
            let (cell_w, cell_h) = cell_sizes
                .get(&self.camera_coord)
                .copied()
                .unwrap_or((col_widths[&self.camera_coord.col], row_heights[&self.camera_coord.row]));
            let mut tx = self.camera_anim.target_x();
            let mut ty = self.camera_anim.target_y();

            let usable_half_w = (area_w / 2.0 - peek_x).max(cell_w / 2.0);
            let cell_left = anchor_x - cell_w / 2.0;
            let cell_right = anchor_x + cell_w / 2.0;
            if cell_left < tx - usable_half_w {
                tx = cell_left + usable_half_w;
            } else if cell_right > tx + usable_half_w {
                tx = cell_right - usable_half_w;
            }

            let usable_half_h = (area_h / 2.0 - peek_y).max(cell_h / 2.0);
            let cell_top = anchor_y - cell_h / 2.0;
            let cell_bottom = anchor_y + cell_h / 2.0;
            if cell_top < ty - usable_half_h {
                ty = cell_top + usable_half_h;
            } else if cell_bottom > ty + usable_half_h {
                ty = cell_bottom - usable_half_h;
            }

            (tx, ty)
        };

        self.camera_anim.set_target(target_cam_x, target_cam_y, animate);
        let cam_animating = if animate {
            self.camera_anim
                .step(dt_secs, stiffness, damping_ratio, snap_epsilon)
        } else {
            false
        };
        let still_animating = cam_animating || size_animating;

        let vp_center_x = area.x as f64 + area_w / 2.0;
        let vp_center_y = area.y as f64 + area_h / 2.0;
        let cam_x = self.camera_anim.current_x();
        let cam_y = self.camera_anim.current_y();

        let mut placements = Vec::with_capacity(self.loc.len());
        self.last_viewport = Some(area);
        self.last_cell_rects.clear();

        for (&coord, cell) in &self.cells {
            let (cw, ch) = cell_sizes[&coord];
            let screen_cx = vp_center_x + (col_centers[&coord.col] - cam_x);
            let screen_cy = vp_center_y + (row_centers[&coord.row] - cam_y);
            let cell_x = (screen_cx - cw / 2.0).round() as i32;
            let cell_y = (screen_cy - ch / 2.0).round() as i32;
            let cell_w = cw.round() as i32;
            let cell_h = ch.round() as i32;
            let mut cell_rect = IRect::new(cell_x, cell_y, cell_w.max(1), cell_h.max(1));
            self.last_cell_rects.insert(coord, cell_rect);

            let cell_visible = cell_rect.intersects(area);
            let overlaps_other = !other_monitors.is_empty()
                && other_monitors.iter().any(|m| cell_rect.intersects(*m));
            let clip_to = if config.clip_cross_monitor_edges && cell_visible && overlaps_other {
                Some(area)
            } else {
                None
            };

            // When bordering adjacent displays in extend mode, clamp target rect strictly within
            // this display's work area so that coordinates NEVER spill over onto the other monitor.
            if clip_to.is_some() {
                if let Some(inter) = cell_rect.intersection(area) {
                    cell_rect = inter;
                }
            }

            let stack_len = cell.windows.len();
            match cell.stack_mode {
                StackMode::Tabbed if stack_len > 1 => {
                    for (idx, &win_id) in cell.windows.iter().enumerate() {
                        placements.push(WindowPlacement {
                            window_id: win_id,
                            rect: cell_rect,
                            visible: cell_visible && idx == cell.active_idx,
                            clip_to,
                            coord,
                            stack_index: idx,
                            stack_len,
                        });
                    }
                }
                _ => {
                    // Split horizontally across available width so snapped apps form one full view side-by-side
                    let gap = config.stack_gap.max(0);
                    let total_gap = gap * (stack_len as i32 - 1).max(0);
                    let avail_w = (cell_rect.w - total_gap).max(stack_len as i32);
                    let base_w = avail_w / stack_len as i32;
                    let rem_w = avail_w % stack_len as i32;
                    let mut cur_x = cell_rect.x;
                    for (idx, &win_id) in cell.windows.iter().enumerate() {
                        let extra = if (idx as i32) < rem_w { 1 } else { 0 };
                        let win_w = (base_w + extra).max(1);
                        let mut win_rect = IRect::new(cur_x, cell_rect.y, win_w, cell_rect.h);
                        let win_visible = win_rect.intersects(area);
                        let win_clip = clip_to;
                        if win_clip.is_some() {
                            if let Some(inter) = win_rect.intersection(area) {
                                win_rect = inter;
                            }
                        }
                        placements.push(WindowPlacement {
                            window_id: win_id,
                            rect: win_rect,
                            visible: win_visible,
                            clip_to: win_clip,
                            coord,
                            stack_index: idx,
                            stack_len,
                        });
                        cur_x += win_w + gap;
                    }
                }
            }
        }

        (placements, still_animating)
    }

    /// Fallback resize-based overview when `overview_thumbnails = false` (`omalattice` style):
    /// physically resizes workspace windows into an interactive overview grid.
    pub fn compute_overview_resize_placements(
        &self,
        area: IRect,
        config: &LayoutConfig,
    ) -> Vec<WindowPlacement> {
        let mut out = Vec::new();
        if self.cells.is_empty() && self.floating.is_empty() {
            return out;
        }
        let mut min_col = self.camera_coord.col;
        let mut max_col = self.camera_coord.col;
        let mut min_row = self.camera_coord.row;
        let mut max_row = self.camera_coord.row;
        if let Some(first) = self.cells.keys().next().copied() {
            min_col = first.col;
            max_col = first.col;
            min_row = first.row;
            max_row = first.row;
            for &c in self.cells.keys() {
                min_col = min_col.min(c.col);
                max_col = max_col.max(c.col);
                min_row = min_row.min(c.row);
                max_row = max_row.max(c.row);
            }
        }
        let float_row = if self.cells.is_empty() {
            min_row
        } else {
            max_row + 1
        };
        if !self.floating.is_empty() {
            max_col = max_col.max(min_col + self.floating.len() as i32 - 1);
            max_row = float_row;
        }

        let cols = (max_col - min_col + 1).max(1);
        let rows = (max_row - min_row + 1).max(1);
        let gap_x = config.gap_x.max(12);
        let gap_y = config.gap_y.max(12);
        let pad_x = 32;
        let pad_y = 32;
        let avail_w = (area.w - pad_x * 2).max(100);
        let avail_h = (area.h - pad_y * 2).max(100);
        let cw = ((avail_w - gap_x * (cols - 1)) / cols).max(100);
        let ch = ((avail_h - gap_y * (rows - 1)) / rows).max(80);
        let total_w = cw * cols + gap_x * (cols - 1);
        let total_h = ch * rows + gap_y * (rows - 1);
        let ox = area.x + (area.w - total_w) / 2;
        let oy = area.y + (area.h - total_h) / 2;

        for (&coord, cell) in &self.cells {
            let x = ox + (coord.col - min_col) * (cw + gap_x);
            let y = oy + (coord.row - min_row) * (ch + gap_y);
            let r = IRect::new(x, y, cw, ch);
            let active = cell.active_window();
            for (idx, &win_id) in cell.windows.iter().enumerate() {
                out.push(WindowPlacement {
                    window_id: win_id,
                    rect: r,
                    visible: win_id == active,
                    clip_to: None,
                    coord,
                    stack_index: idx,
                    stack_len: cell.windows.len(),
                });
            }
        }
        for (i, &win_id) in self.floating.iter().enumerate() {
            let x = ox + (i as i32) * (cw + gap_x);
            let y = oy + (float_row - min_row) * (ch + gap_y);
            out.push(WindowPlacement {
                window_id: win_id,
                rect: IRect::new(x, y, cw, ch),
                visible: true,
                clip_to: None,
                coord: Coord::new(min_col + i as i32, float_row),
                stack_index: 0,
                stack_len: 1,
            });
        }
        out
    }
}
