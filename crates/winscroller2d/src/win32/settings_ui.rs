use crate::win32::overview::{draw_rounded_card, draw_text_argb, CachedSurface};
use crate::win32::{to_wide, WM_APP_APPLY_LIVE_CONFIG};
use std::cell::RefCell;
use std::mem::{size_of, zeroed};
use std::sync::{Mutex, OnceLock};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, EndPaint, InvalidateRect, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, PAINTSTRUCT, SRCCOPY,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, VK_BACK, VK_ESCAPE, VK_RETURN, VK_TAB,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, BringWindowToTop, CreateWindowExW, DefWindowProcW, DestroyWindow,
    LoadCursorW, PostMessageW, RegisterClassExW, SetForegroundWindow, SetWindowPos, ShowWindow,
    ASFW_ANY, HWND_TOPMOST, IDC_ARROW, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, WM_CHAR,
    WM_ERASEBKGND, WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PAINT,
    WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use winscroller2d_core::{Config, IRect, NewWindowPlacement, OverviewBlurMode};

const SETTINGS_CLASS: &str = "WinScroller2dSettingsWindow";
const WIN_W: i32 = 460;
const WIN_H: i32 = 608;
const HEADER_H: i32 = 42;
const ROW_H: i32 = 32;
const START_Y: i32 = 50;

static PENDING_LIVE_CONFIG: OnceLock<Mutex<Option<Config>>> = OnceLock::new();

fn live_queue() -> &'static Mutex<Option<Config>> {
    PENDING_LIVE_CONFIG.get_or_init(|| Mutex::new(None))
}

pub fn take_pending_live_config() -> Option<Config> {
    live_queue().lock().ok().and_then(|mut g| g.take())
}

fn push_live_config(controller_hwnd: HWND, cfg: &Config) {
    if let Ok(mut g) = live_queue().lock() {
        *g = Some(cfg.clone());
    }
    if !controller_hwnd.is_null() {
        unsafe {
            PostMessageW(controller_hwnd, WM_APP_APPLY_LIVE_CONFIG, 0, 0);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingAction {
    PeekXMinus,
    PeekXPlus,
    PeekYMinus,
    PeekYPlus,
    GapMinus,
    GapPlus,
    CyclePlacement,
    ToggleCompact,
    ToggleAutoExpand,
    StiffMinus,
    StiffPlus,
    DampMinus,
    DampPlus,
    ToggleBlurMode,
    ToggleOverviewThumbs,
    MinWMinus,
    MinWPlus,
    ToggleHoverPeeks,
    HoverDelayMinus,
    HoverDelayPlus,
    ToggleCenterCursor,
    ToggleThreeFingerPan,
    TogglePersist,
    ResetDefaults,
    CloseWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditField {
    PeekX,
    PeekY,
    Gap,
    Stiff,
    Damp,
    MinW,
    HoverDelay,
}

#[derive(Clone, Copy)]
struct HitButton {
    rect: IRect,
    action: SettingAction,
}

#[derive(Clone, Copy)]
struct HitEditBox {
    rect: IRect,
    field: EditField,
}

struct ActiveEdit {
    field: EditField,
    buf: String,
    dirty: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionOutcome {
    None,
    Redraw,
    Close,
}

struct SettingsState {
    controller_hwnd: HWND,
    config: Config,
    buttons: Vec<HitButton>,
    edit_boxes: Vec<HitEditBox>,
    hovered: Option<SettingAction>,
    hovered_edit: Option<EditField>,
    active_edit: Option<ActiveEdit>,
    dragging_bar: Option<(i32, i32)>,
    win_pos: (i32, i32),
    surface: Option<CachedSurface>,
}

thread_local! {
    static SETTINGS_STATE: RefCell<Option<SettingsState>> = const { RefCell::new(None) };
}

fn with_settings<R>(f: impl FnOnce(&mut SettingsState) -> R) -> Option<R> {
    SETTINGS_STATE.with(|cell| {
        let Ok(mut guard) = cell.try_borrow_mut() else {
            return None;
        };
        guard.as_mut().map(f)
    })
}

pub struct SettingsWindow {
    hwnd: HWND,
}

impl SettingsWindow {
    pub fn create(controller_hwnd: HWND) -> Result<Self, String> {
        let hinst = unsafe { GetModuleHandleW(std::ptr::null()) };
        let class_w = to_wide(SETTINGS_CLASS);
        let title_w = to_wide("WinScroller 2D Settings");

        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(settings_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: std::ptr::null_mut(),
            hCursor: unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) },
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_w.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        unsafe {
            RegisterClassExW(&wc);
        }

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class_w.as_ptr(),
                title_w.as_ptr(),
                WS_POPUP,
                120,
                120,
                WIN_W,
                WIN_H,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinst,
                std::ptr::null(),
            )
        };
        if hwnd.is_null() {
            return Err("CreateWindowExW for SettingsWindow failed".into());
        }

        SETTINGS_STATE.with(|cell| {
            if let Ok(mut guard) = cell.try_borrow_mut() {
                *guard = Some(SettingsState {
                    controller_hwnd,
                    config: Config::default(),
                    buttons: Vec::new(),
                    edit_boxes: Vec::new(),
                    hovered: None,
                    hovered_edit: None,
                    active_edit: None,
                    dragging_bar: None,
                    win_pos: (120, 120),
                    surface: None,
                });
            }
        });

        Ok(Self { hwnd })
    }

    pub fn show(&mut self, cfg: &Config, monitor_bounds: IRect) {
        let x = monitor_bounds.x + (monitor_bounds.w - WIN_W) / 2;
        let y = monitor_bounds.y + (monitor_bounds.h - WIN_H) / 2;
        with_settings(|st| {
            st.config = cfg.clone();
            st.win_pos = (x, y);
            st.hovered = None;
            st.hovered_edit = None;
            st.active_edit = None;
        });
        unsafe {
            AllowSetForegroundWindow(ASFW_ANY);
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                x,
                y,
                WIN_W,
                WIN_H,
                SWP_SHOWWINDOW,
            );
            BringWindowToTop(self.hwnd);
            SetForegroundWindow(self.hwnd);
            InvalidateRect(self.hwnd, std::ptr::null(), 0);
        }
    }

    pub fn sync_config(&mut self, cfg: &Config) {
        with_settings(|st| {
            st.config = cfg.clone();
        });
        unsafe {
            InvalidateRect(self.hwnd, std::ptr::null(), 0);
        }
    }

    pub fn hide(&mut self) {
        with_settings(|st| {
            st.active_edit = None;
            st.dragging_bar = None;
        });
        unsafe {
            ShowWindow(self.hwnd, SW_HIDE);
        }
    }
}

impl Drop for SettingsWindow {
    fn drop(&mut self) {
        SETTINGS_STATE.with(|cell| {
            if let Ok(mut guard) = cell.try_borrow_mut() {
                *guard = None;
            }
        });
        if !self.hwnd.is_null() {
            unsafe {
                DestroyWindow(self.hwnd);
            }
        }
    }
}

fn edit_field_text(cfg: &Config, field: EditField) -> String {
    match field {
        EditField::PeekX => format!("{}", cfg.layout.peek_x),
        EditField::PeekY => format!("{}", cfg.layout.peek_y),
        EditField::Gap => format!("{}", cfg.layout.gap_x),
        EditField::Stiff => format!("{:.0}", cfg.animation.stiffness),
        EditField::Damp => format!("{:.2}", cfg.animation.damping_ratio),
        EditField::MinW => format!("{:.0}", cfg.layout.overview_min_width * 100.0),
        EditField::HoverDelay => format!("{}", cfg.layout.hover_focus_delay_ms),
    }
}

fn apply_edit_value(cfg: &mut Config, field: EditField, raw: &str) -> bool {
    let s = raw.trim();
    if s.is_empty() {
        return false;
    }
    match field {
        EditField::PeekX => {
            let Ok(v) = s.parse::<i32>() else { return false };
            cfg.layout.peek_x = v.clamp(0, 256);
        }
        EditField::PeekY => {
            let Ok(v) = s.parse::<i32>() else { return false };
            cfg.layout.peek_y = v.clamp(0, 256);
        }
        EditField::Gap => {
            let Ok(v) = s.parse::<i32>() else { return false };
            let clamped = v.clamp(0, 64);
            cfg.layout.gap_x = clamped;
            cfg.layout.gap_y = clamped;
            cfg.layout.stack_gap = clamped;
        }
        EditField::Stiff => {
            let Ok(v) = s.parse::<f64>() else { return false };
            if !v.is_finite() {
                return false;
            }
            cfg.animation.stiffness = v.clamp(80.0, 600.0);
        }
        EditField::Damp => {
            let Ok(v) = s.parse::<f64>() else { return false };
            if !v.is_finite() {
                return false;
            }
            cfg.animation.damping_ratio = ((v * 100.0).round() / 100.0).clamp(0.50, 1.20);
        }
        EditField::MinW => {
            let Ok(v) = s.parse::<f64>() else { return false };
            if !v.is_finite() {
                return false;
            }
            let frac = (v.clamp(10.0, 50.0) / 100.0 * 100.0).round() / 100.0;
            cfg.layout.overview_min_width = frac;
            cfg.layout.overview_min_height = frac;
        }
        EditField::HoverDelay => {
            let Ok(v) = s.parse::<u64>() else { return false };
            cfg.layout.hover_focus_delay_ms = v.min(2000);
        }
    }
    let _ = cfg.normalize();
    true
}

fn commit_active_edit(st: &mut SettingsState) -> bool {
    let Some(edit) = st.active_edit.take() else {
        return false;
    };
    if edit.dirty && apply_edit_value(&mut st.config, edit.field, &edit.buf) {
        push_live_config(st.controller_hwnd, &st.config);
        return true;
    }
    false
}

fn next_edit_field(cur: EditField) -> EditField {
    match cur {
        EditField::PeekX => EditField::PeekY,
        EditField::PeekY => EditField::Gap,
        EditField::Gap => EditField::Stiff,
        EditField::Stiff => EditField::Damp,
        EditField::Damp => EditField::MinW,
        EditField::MinW => EditField::HoverDelay,
        EditField::HoverDelay => EditField::PeekX,
    }
}

fn apply_setting_action(st: &mut SettingsState, act: SettingAction) -> ActionOutcome {
    let l = &mut st.config.layout;
    let a = &mut st.config.animation;
    let ap = &mut st.config.appearance;
    match act {
        SettingAction::PeekXMinus => l.peek_x = (l.peek_x - 8).max(0),
        SettingAction::PeekXPlus => l.peek_x = (l.peek_x + 8).min(256),
        SettingAction::PeekYMinus => l.peek_y = (l.peek_y - 8).max(0),
        SettingAction::PeekYPlus => l.peek_y = (l.peek_y + 8).min(256),
        SettingAction::GapMinus => {
            l.gap_x = (l.gap_x - 4).max(0);
            l.gap_y = l.gap_x;
            l.stack_gap = l.gap_x;
        }
        SettingAction::GapPlus => {
            l.gap_x = (l.gap_x + 4).min(64);
            l.gap_y = l.gap_x;
            l.stack_gap = l.gap_x;
        }
        SettingAction::CyclePlacement => {
            l.new_window_placement = match l.new_window_placement {
                NewWindowPlacement::RightOfFocus => NewWindowPlacement::Neighbor4Way,
                NewWindowPlacement::Neighbor4Way => NewWindowPlacement::BelowFocus,
                NewWindowPlacement::BelowFocus => NewWindowPlacement::RightmostInRow,
                NewWindowPlacement::RightmostInRow => NewWindowPlacement::StackInCurrentCell,
                NewWindowPlacement::StackInCurrentCell => NewWindowPlacement::RightOfFocus,
            };
        }
        SettingAction::ToggleCompact => l.auto_compact_empty = !l.auto_compact_empty,
        SettingAction::ToggleAutoExpand => l.auto_expand_focused = !l.auto_expand_focused,
        SettingAction::StiffMinus => a.stiffness = (a.stiffness - 20.0).clamp(80.0, 600.0),
        SettingAction::StiffPlus => a.stiffness = (a.stiffness + 20.0).clamp(80.0, 600.0),
        SettingAction::DampMinus => {
            a.damping_ratio = ((a.damping_ratio - 0.04) * 100.0).round() / 100.0;
            a.damping_ratio = a.damping_ratio.clamp(0.50, 1.20);
        }
        SettingAction::DampPlus => {
            a.damping_ratio = ((a.damping_ratio + 0.04) * 100.0).round() / 100.0;
            a.damping_ratio = a.damping_ratio.clamp(0.50, 1.20);
        }
        SettingAction::ToggleBlurMode => {
            ap.blur_mode = match ap.blur_mode {
                OverviewBlurMode::PureBlur => OverviewBlurMode::AcrylicClear,
                OverviewBlurMode::AcrylicClear => OverviewBlurMode::PureBlur,
            };
            ap.overview_alpha = if matches!(ap.blur_mode, OverviewBlurMode::PureBlur) {
                0
            } else {
                10
            };
        }
        SettingAction::ToggleOverviewThumbs => l.overview_thumbnails = !l.overview_thumbnails,
        SettingAction::MinWMinus => {
            l.overview_min_width = (l.overview_min_width - 0.05).clamp(0.10, 0.50);
            l.overview_min_height = l.overview_min_width;
        }
        SettingAction::MinWPlus => {
            l.overview_min_width = (l.overview_min_width + 0.05).clamp(0.10, 0.50);
            l.overview_min_height = l.overview_min_width;
        }
        SettingAction::ToggleHoverPeeks => l.hover_focus_peeks = !l.hover_focus_peeks,
        SettingAction::HoverDelayMinus => {
            l.hover_focus_delay_ms = l.hover_focus_delay_ms.saturating_sub(25);
        }
        SettingAction::HoverDelayPlus => {
            l.hover_focus_delay_ms = (l.hover_focus_delay_ms + 25).min(500);
        }
        SettingAction::ToggleCenterCursor => l.center_cursor_on_peek = !l.center_cursor_on_peek,
        SettingAction::ToggleThreeFingerPan => l.three_finger_pan = !l.three_finger_pan,
        SettingAction::TogglePersist => l.persist_state = !l.persist_state,
        SettingAction::ResetDefaults => {
            let preserve_bindings = st.config.bindings.clone();
            let preserve_rules = st.config.rules.clone();
            st.config = Config::default();
            st.config.bindings = preserve_bindings;
            st.config.rules = preserve_rules;
        }
        SettingAction::CloseWindow => {
            return ActionOutcome::Close;
        }
    }
    let _ = st.config.normalize();
    push_live_config(st.controller_hwnd, &st.config);
    ActionOutcome::Redraw
}

unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            unsafe { paint_settings(hwnd) };
            0
        }
        WM_LBUTTONDOWN => {
            let mx = (lparam & 0xFFFF) as i16 as i32;
            let my = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            unsafe {
                SetForegroundWindow(hwnd);
            }
            let (outcome, start_drag, need_redraw) = with_settings(|st| {
                let had_edit = st.active_edit.is_some();
                let mut redraw = commit_active_edit(st);

                let hit_btn = st
                    .buttons
                    .iter()
                    .find(|b| {
                        mx >= b.rect.x
                            && mx <= b.rect.right()
                            && my >= b.rect.y
                            && my <= b.rect.bottom()
                    })
                    .map(|b| b.action);
                if let Some(act) = hit_btn {
                    return (apply_setting_action(st, act), false, redraw);
                }

                let hit_edit = st
                    .edit_boxes
                    .iter()
                    .find(|e| {
                        mx >= e.rect.x
                            && mx <= e.rect.right()
                            && my >= e.rect.y
                            && my <= e.rect.bottom()
                    })
                    .map(|e| e.field);
                if let Some(field) = hit_edit {
                    let initial = edit_field_text(&st.config, field);
                    st.active_edit = Some(ActiveEdit {
                        field,
                        buf: initial,
                        dirty: false,
                    });
                    return (ActionOutcome::None, false, true);
                }

                if had_edit {
                    redraw = true;
                }
                let drag = if my <= HEADER_H {
                    st.dragging_bar = Some((mx, my));
                    true
                } else {
                    false
                };
                (ActionOutcome::None, drag, redraw)
            })
            .unwrap_or((ActionOutcome::None, false, false));

            if start_drag {
                unsafe {
                    SetCapture(hwnd);
                }
            }
            if outcome == ActionOutcome::Close {
                unsafe {
                    ShowWindow(hwnd, SW_HIDE);
                }
            } else if outcome == ActionOutcome::Redraw || need_redraw {
                unsafe {
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
            }
            0
        }
        WM_MOUSEMOVE => {
            let mx = (lparam & 0xFFFF) as i16 as i32;
            let my = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            let (move_pos, redraw) = with_settings(|st| {
                if let Some((ox, oy)) = st.dragging_bar {
                    let nx = st.win_pos.0 + (mx - ox);
                    let ny = st.win_pos.1 + (my - oy);
                    st.win_pos = (nx, ny);
                    return (Some((nx, ny)), false);
                }
                let next_btn = st
                    .buttons
                    .iter()
                    .find(|b| {
                        mx >= b.rect.x
                            && mx <= b.rect.right()
                            && my >= b.rect.y
                            && my <= b.rect.bottom()
                    })
                    .map(|b| b.action);
                let next_edit = st
                    .edit_boxes
                    .iter()
                    .find(|e| {
                        mx >= e.rect.x
                            && mx <= e.rect.right()
                            && my >= e.rect.y
                            && my <= e.rect.bottom()
                    })
                    .map(|e| e.field);
                let changed = st.hovered != next_btn || st.hovered_edit != next_edit;
                st.hovered = next_btn;
                st.hovered_edit = next_edit;
                (None, changed)
            })
            .unwrap_or((None, false));

            if let Some((nx, ny)) = move_pos {
                unsafe {
                    SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        nx,
                        ny,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER,
                    );
                }
            } else if redraw {
                unsafe {
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
            }
            0
        }
        WM_LBUTTONUP => {
            let released = with_settings(|st| st.dragging_bar.take().is_some()).unwrap_or(false);
            if released {
                unsafe {
                    ReleaseCapture();
                }
            }
            0
        }
        WM_KILLFOCUS => {
            let changed = with_settings(|st| {
                let had = st.active_edit.is_some();
                commit_active_edit(st) || had
            })
            .unwrap_or(false);
            if changed {
                unsafe {
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
            }
            0
        }
        WM_KEYDOWN => {
            let vk = wparam as u16;
            enum KeyRes {
                None,
                Redraw,
                Hide,
            }
            let res = with_settings(|st| {
                if vk == VK_ESCAPE {
                    if st.active_edit.take().is_some() {
                        return KeyRes::Redraw;
                    }
                    return KeyRes::Hide;
                }
                if vk == VK_RETURN {
                    if st.active_edit.is_some() {
                        commit_active_edit(st);
                        return KeyRes::Redraw;
                    }
                    return KeyRes::None;
                }
                if vk == VK_TAB {
                    if let Some(cur) = st.active_edit.as_ref().map(|e| e.field) {
                        commit_active_edit(st);
                        let next = next_edit_field(cur);
                        let initial = edit_field_text(&st.config, next);
                        st.active_edit = Some(ActiveEdit {
                            field: next,
                            buf: initial,
                            dirty: false,
                        });
                        return KeyRes::Redraw;
                    }
                    return KeyRes::None;
                }
                if vk == VK_BACK {
                    if let Some(ref mut edit) = st.active_edit {
                        if !edit.dirty {
                            edit.buf.clear();
                            edit.dirty = true;
                        } else {
                            edit.buf.pop();
                        }
                        return KeyRes::Redraw;
                    }
                }
                KeyRes::None
            })
            .unwrap_or(KeyRes::None);

            match res {
                KeyRes::Hide => unsafe {
                    ShowWindow(hwnd, SW_HIDE);
                },
                KeyRes::Redraw => unsafe {
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                },
                KeyRes::None => {}
            }
            0
        }
        WM_CHAR => {
            if let Some(ch) = char::from_u32(wparam as u32) {
                let redraw = with_settings(|st| {
                    let Some(ref mut edit) = st.active_edit else {
                        return false;
                    };
                    let allow_dot = matches!(edit.field, EditField::Damp)
                        && (!edit.dirty || !edit.buf.contains('.'));
                    if ch.is_ascii_digit() || (ch == '.' && allow_dot) {
                        if !edit.dirty {
                            edit.buf.clear();
                            edit.dirty = true;
                        }
                        if edit.buf.len() < 8 {
                            edit.buf.push(ch);
                        }
                        return true;
                    }
                    false
                })
                .unwrap_or(false);
                if redraw {
                    unsafe {
                        InvalidateRect(hwnd, std::ptr::null(), 0);
                    }
                }
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn paint_settings(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = unsafe { zeroed() };
    let hdc_screen = unsafe { BeginPaint(hwnd, &mut ps) };
    if hdc_screen.is_null() {
        return;
    }

    with_settings(|st| {
        if st.surface.is_none() {
            st.surface = unsafe { CachedSurface::acquire(hdc_screen, WIN_W, WIN_H) };
        }
        let Some(surf) = st.surface.take() else {
            return;
        };

        st.buttons.clear();
        st.edit_boxes.clear();
        let mut buttons_tmp = Vec::new();
        let mut edit_boxes_tmp = Vec::new();
        let l = &st.config.layout;
        let a = &st.config.animation;
        let ap = &st.config.appearance;

        unsafe {
            let pixels = std::slice::from_raw_parts_mut(surf.main.bits, (WIN_W * WIN_H) as usize);
            pixels.fill(0xFF15_1822);
            let scratch = std::slice::from_raw_parts_mut(
                surf.scratch.bits,
                (surf.scratch.w * surf.scratch.h) as usize,
            );

            // Outer frame
            draw_rounded_card(
                pixels,
                WIN_W,
                WIN_H,
                IRect::new(0, 0, WIN_W, WIN_H),
                12,
                (22, 25, 35, 255),
                (72, 86, 122, 255),
                1,
                false,
            );

            // Header bar
            draw_rounded_card(
                pixels,
                WIN_W,
                WIN_H,
                IRect::new(1, 1, WIN_W - 2, HEADER_H),
                10,
                (29, 34, 48, 255),
                (44, 52, 74, 255),
                1,
                false,
            );
            draw_text_argb(
                pixels,
                WIN_W,
                WIN_H,
                surf.scratch.hdc,
                scratch,
                surf.scratch.w,
                surf.scratch.h,
                surf.title_font,
                IRect::new(16, 4, 300, HEADER_H - 8),
                "WinScroller 2D Settings",
                (240, 245, 255),
                255,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
            let close_btn = IRect::new(WIN_W - 36, 9, 24, 24);
            let close_hov = st.hovered == Some(SettingAction::CloseWindow);
            draw_rounded_card(
                pixels,
                WIN_W,
                WIN_H,
                close_btn,
                6,
                if close_hov { (190, 68, 78, 230) } else { (48, 56, 78, 200) },
                (92, 108, 145, 200),
                1,
                false,
            );
            draw_text_argb(
                pixels,
                WIN_W,
                WIN_H,
                surf.scratch.hdc,
                scratch,
                surf.scratch.w,
                surf.scratch.h,
                surf.title_font,
                close_btn,
                "X",
                (240, 245, 255),
                255,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
            buttons_tmp.push(HitButton {
                rect: close_btn,
                action: SettingAction::CloseWindow,
            });

            enum RowKind {
                Stepper(SettingAction, SettingAction, EditField, String),
                Toggle(SettingAction, bool),
                Cycle(SettingAction, String),
            }

            let placement_label = match l.new_window_placement {
                NewWindowPlacement::RightOfFocus => "Right of Focus",
                NewWindowPlacement::Neighbor4Way => "4-Way Neighbor",
                NewWindowPlacement::BelowFocus => "Below Focus",
                NewWindowPlacement::RightmostInRow => "Rightmost in Row",
                NewWindowPlacement::StackInCurrentCell => "Stack in Cell",
            };
            let blur_label = match ap.blur_mode {
                OverviewBlurMode::PureBlur => "Pure Blur",
                OverviewBlurMode::AcrylicClear => "Light Frost",
            };

            let rows = [
                ("Edge Peek X", RowKind::Stepper(SettingAction::PeekXMinus, SettingAction::PeekXPlus, EditField::PeekX, format!("{} px", l.peek_x))),
                ("Edge Peek Y", RowKind::Stepper(SettingAction::PeekYMinus, SettingAction::PeekYPlus, EditField::PeekY, format!("{} px", l.peek_y))),
                ("Cell Gap", RowKind::Stepper(SettingAction::GapMinus, SettingAction::GapPlus, EditField::Gap, format!("{} px", l.gap_x))),
                ("New Window Placement", RowKind::Cycle(SettingAction::CyclePlacement, placement_label.into())),
                ("Auto-Compact Empty Bands", RowKind::Toggle(SettingAction::ToggleCompact, l.auto_compact_empty)),
                ("Auto-Expand Focused", RowKind::Toggle(SettingAction::ToggleAutoExpand, l.auto_expand_focused)),
                ("Spring Stiffness", RowKind::Stepper(SettingAction::StiffMinus, SettingAction::StiffPlus, EditField::Stiff, format!("{:.0}", a.stiffness))),
                ("Spring Damping Ratio", RowKind::Stepper(SettingAction::DampMinus, SettingAction::DampPlus, EditField::Damp, format!("{:.2}", a.damping_ratio))),
                ("Overview Blur Style", RowKind::Cycle(SettingAction::ToggleBlurMode, blur_label.into())),
                ("Overview Live Thumbnails", RowKind::Toggle(SettingAction::ToggleOverviewThumbs, l.overview_thumbnails)),
                ("Overview Min Card Size", RowKind::Stepper(SettingAction::MinWMinus, SettingAction::MinWPlus, EditField::MinW, format!("{:.0}%", l.overview_min_width * 100.0))),
                ("Hover-Focus Peeks", RowKind::Toggle(SettingAction::ToggleHoverPeeks, l.hover_focus_peeks)),
                ("Hover Dwell Delay", RowKind::Stepper(SettingAction::HoverDelayMinus, SettingAction::HoverDelayPlus, EditField::HoverDelay, format!("{} ms", l.hover_focus_delay_ms))),
                ("Center Cursor on Peek", RowKind::Toggle(SettingAction::ToggleCenterCursor, l.center_cursor_on_peek)),
                ("3-Finger Touchpad Pan", RowKind::Toggle(SettingAction::ToggleThreeFingerPan, l.three_finger_pan)),
                ("Persist Workspace Grids", RowKind::Toggle(SettingAction::TogglePersist, l.persist_state)),
            ];

            for (idx, (label, kind)) in rows.into_iter().enumerate() {
                let ry = START_Y + idx as i32 * ROW_H;
                if idx % 2 == 0 {
                    draw_rounded_card(
                        pixels,
                        WIN_W,
                        WIN_H,
                        IRect::new(12, ry, WIN_W - 24, ROW_H - 2),
                        6,
                        (30, 35, 49, 175),
                        (40, 48, 66, 0),
                        1,
                        false,
                    );
                }
                draw_text_argb(
                    pixels,
                    WIN_W,
                    WIN_H,
                    surf.scratch.hdc,
                    scratch,
                    surf.scratch.w,
                    surf.scratch.h,
                    surf.sub_font,
                    IRect::new(20, ry, 230, ROW_H - 2),
                    label,
                    (222, 230, 248),
                    255,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
                );

                match kind {
                    RowKind::Stepper(act_m, act_p, field, val) => {
                        let r_m = IRect::new(WIN_W - 152, ry + 3, 28, ROW_H - 8);
                        let r_v = IRect::new(WIN_W - 122, ry + 3, 72, ROW_H - 8);
                        let r_p = IRect::new(WIN_W - 48, ry + 3, 28, ROW_H - 8);

                        for (rc, sym, act) in [(r_m, "-", act_m), (r_p, "+", act_p)] {
                            let hov = st.hovered == Some(act);
                            draw_rounded_card(
                                pixels,
                                WIN_W,
                                WIN_H,
                                rc,
                                5,
                                if hov { (74, 102, 165, 255) } else { (44, 52, 74, 255) },
                                (100, 122, 175, 210),
                                1,
                                false,
                            );
                            draw_text_argb(
                                pixels,
                                WIN_W,
                                WIN_H,
                                surf.scratch.hdc,
                                scratch,
                                surf.scratch.w,
                                surf.scratch.h,
                                surf.title_font,
                                rc,
                                sym,
                                (245, 248, 255),
                                255,
                                DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                            );
                            buttons_tmp.push(HitButton { rect: rc, action: act });
                        }
                        let active_edit = st.active_edit.as_ref().filter(|e| e.field == field);
                        let edit_hov = st.hovered_edit == Some(field);
                        let (box_bg, box_border, txt_rgb, display_val) = if let Some(ae) = active_edit {
                            let bg = if !ae.dirty {
                                (52, 82, 148, 255)
                            } else {
                                (28, 36, 56, 255)
                            };
                            (
                                bg,
                                (130, 180, 255, 255),
                                (250, 252, 255),
                                format!("{}|", ae.buf),
                            )
                        } else {
                            (
                                if edit_hov { (36, 44, 66, 245) } else { (24, 28, 42, 235) },
                                if edit_hov { (102, 136, 205, 225) } else { (68, 82, 118, 170) },
                                (122, 162, 247),
                                val,
                            )
                        };
                        draw_rounded_card(pixels, WIN_W, WIN_H, r_v, 5, box_bg, box_border, 1, false);
                        draw_text_argb(
                            pixels,
                            WIN_W,
                            WIN_H,
                            surf.scratch.hdc,
                            scratch,
                            surf.scratch.w,
                            surf.scratch.h,
                            surf.sub_font,
                            r_v,
                            &display_val,
                            txt_rgb,
                            255,
                            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                        );
                        edit_boxes_tmp.push(HitEditBox { rect: r_v, field });
                    }
                    RowKind::Toggle(act, on) => {
                        let rc = IRect::new(WIN_W - 88, ry + 4, 68, ROW_H - 10);
                        let hov = st.hovered == Some(act);
                        let (bg, border, txt) = if on {
                            (
                                if hov { (72, 168, 114, 255) } else { (56, 140, 94, 245) },
                                (122, 247, 158, 230),
                                "ON",
                            )
                        } else {
                            (
                                if hov { (64, 72, 96, 255) } else { (42, 48, 66, 245) },
                                (96, 108, 138, 210),
                                "OFF",
                            )
                        };
                        draw_rounded_card(pixels, WIN_W, WIN_H, rc, 10, bg, border, 1, false);
                        draw_text_argb(
                            pixels,
                            WIN_W,
                            WIN_H,
                            surf.scratch.hdc,
                            scratch,
                            surf.scratch.w,
                            surf.scratch.h,
                            surf.sub_font,
                            rc,
                            txt,
                            (245, 250, 255),
                            255,
                            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                        );
                        buttons_tmp.push(HitButton { rect: rc, action: act });
                    }
                    RowKind::Cycle(act, txt) => {
                        let rc = IRect::new(WIN_W - 152, ry + 3, 132, ROW_H - 8);
                        let hov = st.hovered == Some(act);
                        draw_rounded_card(
                            pixels,
                            WIN_W,
                            WIN_H,
                            rc,
                            6,
                            if hov { (66, 92, 152, 255) } else { (44, 54, 80, 255) },
                            (122, 162, 247, 220),
                            1,
                            false,
                        );
                        draw_text_argb(
                            pixels,
                            WIN_W,
                            WIN_H,
                            surf.scratch.hdc,
                            scratch,
                            surf.scratch.w,
                            surf.scratch.h,
                            surf.sub_font,
                            rc,
                            &txt,
                            (235, 244, 255),
                            255,
                            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
                        );
                        buttons_tmp.push(HitButton { rect: rc, action: act });
                    }
                }
            }

            // Footer buttons
            let fy = WIN_H - 40;
            let reset_rc = IRect::new(16, fy, 132, 28);
            let done_rc = IRect::new(WIN_W - 116, fy, 100, 28);
            for (rc, label, act, accent) in [
                (reset_rc, "Reset Defaults", SettingAction::ResetDefaults, false),
                (done_rc, "Done", SettingAction::CloseWindow, true),
            ] {
                let hov = st.hovered == Some(act);
                let fill = if accent {
                    if hov { (110, 152, 242, 255) } else { (86, 128, 218, 255) }
                } else if hov {
                    (66, 76, 104, 255)
                } else {
                    (46, 54, 76, 255)
                };
                draw_rounded_card(pixels, WIN_W, WIN_H, rc, 6, fill, (122, 162, 247, 210), 1, false);
                draw_text_argb(
                    pixels,
                    WIN_W,
                    WIN_H,
                    surf.scratch.hdc,
                    scratch,
                    surf.scratch.w,
                    surf.scratch.h,
                    surf.title_font,
                    rc,
                    label,
                    (250, 252, 255),
                    255,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
                buttons_tmp.push(HitButton { rect: rc, action: act });
            }

            BitBlt(hdc_screen, 0, 0, WIN_W, WIN_H, surf.main.hdc, 0, 0, SRCCOPY);
        }

        st.buttons = buttons_tmp;
        st.edit_boxes = edit_boxes_tmp;
        st.surface = Some(surf);
    });

    unsafe {
        EndPaint(hwnd, &mut ps);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_edit_value_parses_and_clamps_numeric_fields() {
        let mut cfg = Config::default();
        assert!(apply_edit_value(&mut cfg, EditField::PeekX, "64"));
        assert_eq!(cfg.layout.peek_x, 64);

        assert!(apply_edit_value(&mut cfg, EditField::PeekY, "999"));
        assert_eq!(cfg.layout.peek_y, 256);

        assert!(apply_edit_value(&mut cfg, EditField::Gap, "16"));
        assert_eq!(cfg.layout.gap_x, 16);
        assert_eq!(cfg.layout.gap_y, 16);
        assert_eq!(cfg.layout.stack_gap, 16);

        assert!(apply_edit_value(&mut cfg, EditField::Stiff, "320"));
        assert!((cfg.animation.stiffness - 320.0).abs() < 1e-6);

        assert!(apply_edit_value(&mut cfg, EditField::Damp, "0.92"));
        assert!((cfg.animation.damping_ratio - 0.92).abs() < 1e-6);

        assert!(apply_edit_value(&mut cfg, EditField::MinW, "30"));
        assert!((cfg.layout.overview_min_width - 0.30).abs() < 1e-6);
        assert!((cfg.layout.overview_min_height - 0.30).abs() < 1e-6);

        assert!(apply_edit_value(&mut cfg, EditField::HoverDelay, "250"));
        assert_eq!(cfg.layout.hover_focus_delay_ms, 250);

        assert!(!apply_edit_value(&mut cfg, EditField::PeekX, ""));
        assert!(!apply_edit_value(&mut cfg, EditField::Damp, "abc"));
        assert_eq!(edit_field_text(&cfg, EditField::MinW), "30");
    }
}
