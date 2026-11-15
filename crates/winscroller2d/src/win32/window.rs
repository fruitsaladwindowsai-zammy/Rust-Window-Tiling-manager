use crate::win32::dpi::{
    get_extended_frame_margins, set_dwm_border_color, FrameMargins, DWMWA_COLOR_DEFAULT,
};
use crate::win32::{from_wide, to_wide};
use std::collections::HashMap;
use std::mem::{size_of, zeroed};
use std::time::{Duration, Instant};
use winscroller2d_core::{IRect, WindowCommand, WindowId, WindowInfo};
use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, RECT, TRUE};
use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DwmSetWindowAttribute, DWMWA_CLOAK, DWMWA_CLOAKED};
use windows_sys::Win32::Graphics::Gdi::{
    CreateRectRgn, DeleteObject, GetMonitorInfoW, MonitorFromWindow, SetWindowRgn, HRGN,
    MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, BeginDeferWindowPos, BringWindowToTop, DeferWindowPos,
    EndDeferWindowPos, EnumWindows, GetAncestor, GetClassNameW, GetForegroundWindow, GetWindow,
    GetWindowLongPtrW, GetWindowPlacement, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId,
    IsIconic, IsWindow, IsWindowVisible, IsZoomed, PostMessageW, SetCursorPos, SetForegroundWindow,
    ShowWindow, ASFW_ANY, GA_ROOT, GWL_EXSTYLE, GWL_STYLE, GW_OWNER, HDWP, SWP_ASYNCWINDOWPOS,
    SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SetWindowPos,
    SW_RESTORE, WINDOWPLACEMENT, WM_CLOSE, WS_CAPTION, WS_CHILD, WS_EX_APPWINDOW,
    WS_EX_TOOLWINDOW, WS_THICKFRAME,
};

const IGNORED_CLASSES: &[&str] = &[
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "Progman",
    "WorkerW",
    "Windows.UI.Core.CoreWindow",
    "XamlExplorerHostIslandWindow",
    "ForegroundStaging",
    "ApplicationManager_DesktopShellWindow",
    "TaskListThumbnailWnd",
    "MultitaskingViewFrame",
    "NotifyIconOverflowWindow",
    "TopLevelWindowForOverflowXamlIsland",
    "Shell_InputSwitchTopLevelWindow",
    "IME",
    "MSCTFIME UI",
    "WinScroller2dControllerWindow",
    "WinScroller2dOverviewWindow",
    "WinScroller2dOverviewIcons",
    "WinScroller2dSettingsWindow",
    "tooltips_class32",
    "#32768",
    "ComboLBox",
    "SysShadow",
];

pub fn hwnd_to_id(hwnd: HWND) -> WindowId {
    hwnd as usize as u64
}

pub fn id_to_hwnd(id: WindowId) -> HWND {
    id as usize as HWND
}

pub fn is_valid_window(id: WindowId) -> bool {
    let hwnd = id_to_hwnd(id);
    unsafe { IsWindow(hwnd) != 0 }
}

#[allow(dead_code)]
pub fn is_minimized(id: WindowId) -> bool {
    unsafe { IsIconic(id_to_hwnd(id)) != 0 }
}

pub fn get_foreground_window_id() -> Option<WindowId> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        None
    } else {
        Some(hwnd_to_id(hwnd))
    }
}

pub fn focus_os_window(id: WindowId) {
    let hwnd = id_to_hwnd(id);
    unsafe {
        if IsWindow(hwnd) == 0 {
            return;
        }
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        AllowSetForegroundWindow(ASFW_ANY);
        BringWindowToTop(hwnd);
        if SetForegroundWindow(hwnd) == 0 {
            let fg = GetForegroundWindow();
            let fg_tid = if !fg.is_null() {
                GetWindowThreadProcessId(fg, std::ptr::null_mut())
            } else {
                0
            };
            let target_tid = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
            let cur_tid = GetCurrentThreadId();
            let attach_fg = fg_tid != 0 && fg_tid != cur_tid;
            let attach_target =
                target_tid != 0 && target_tid != cur_tid && target_tid != fg_tid;
            if attach_fg {
                AttachThreadInput(cur_tid, fg_tid, 1);
            }
            if attach_target {
                AttachThreadInput(cur_tid, target_tid, 1);
            }
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
            if attach_target {
                AttachThreadInput(cur_tid, target_tid, 0);
            }
            if attach_fg {
                AttachThreadInput(cur_tid, fg_tid, 0);
            }
        }
    }
}

pub fn center_cursor_in_window(id: WindowId) {
    let hwnd = id_to_hwnd(id);
    unsafe {
        if IsWindow(hwnd) == 0 {
            return;
        }
        let mut rc: RECT = zeroed();
        if GetWindowRect(hwnd, &mut rc) != 0 && rc.right > rc.left && rc.bottom > rc.top {
            let cx = rc.left + (rc.right - rc.left) / 2;
            let cy = rc.top + (rc.bottom - rc.top) / 2;
            let _ = SetCursorPos(cx, cy);
        }
    }
}

pub fn root_window_from_point(x: i32, y: i32) -> Option<WindowId> {
    let pt = windows_sys::Win32::Foundation::POINT { x, y };
    let hit = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::WindowFromPoint(pt) };
    if hit.is_null() {
        return None;
    }
    let root = unsafe { GetAncestor(hit, GA_ROOT) };
    let resolved = if root.is_null() { hit } else { root };
    Some(hwnd_to_id(resolved))
}

pub fn close_os_window(id: WindowId) {
    let hwnd = id_to_hwnd(id);
    unsafe {
        if IsWindow(hwnd) != 0 {
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
        }
    }
}

pub fn read_window_title(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        String::new()
    } else {
        from_wide(&buf[..len as usize])
    }
}

pub fn read_class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
    if len <= 0 {
        String::new()
    } else {
        from_wide(&buf[..len as usize])
    }
}

pub fn read_process_name(hwnd: HWND) -> String {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return String::new();
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return String::new();
        }
        let mut buf = [0u16; 512];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);
        if ok == 0 || size == 0 {
            return String::new();
        }
        let full = from_wide(&buf[..size as usize]);
        full.rsplit(['\\', '/']).next().unwrap_or(&full).to_string()
    }
}

pub fn is_manageable_window(hwnd: HWND, own_pid: u32, already_cloaked_by_wm: bool) -> bool {
    unsafe {
        if hwnd.is_null() || IsWindow(hwnd) == 0 {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 || pid == own_pid {
            return false;
        }
        if IsWindowVisible(hwnd) == 0 && IsIconic(hwnd) == 0 {
            return false;
        }

        // Only ignore windows cloaked by the shell because they are on another OS virtual desktop.
        // DWM_CLOAKED_SHELL = 0x0000_0002.
        // Background UWP apps (DWM_CLOAKED_APP = 1) or transient cloaked states should still be manageable.
        if !already_cloaked_by_wm {
            let mut cloaked: u32 = 0;
            let hr = DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED as u32,
                &mut cloaked as *mut u32 as *mut _,
                size_of::<u32>() as u32,
            );
            if hr == 0 && (cloaked & 0x0000_0002) != 0 {
                return false;
            }
        }

        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        if (style & WS_CHILD) != 0 {
            return false;
        }

        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let is_app_window = (ex_style & WS_EX_APPWINDOW) != 0;
        if !is_app_window {
            if (ex_style & WS_EX_TOOLWINDOW) != 0 {
                return false;
            }
            let owner = GetWindow(hwnd, GW_OWNER);
            if !owner.is_null() && IsWindowVisible(owner) != 0 {
                // If owner is visible, only manage if window has caption/border styles.
                let has_caption = (style & WS_CAPTION) != 0;
                let has_thick = (style & WS_THICKFRAME) != 0;
                if !has_caption && !has_thick {
                    return false;
                }
            }
        }

        let class_name = read_class_name(hwnd);
        if IGNORED_CLASSES.iter().any(|c| *c == class_name) {
            return false;
        }

        let proc_name = read_process_name(hwnd);
        if proc_name.eq_ignore_ascii_case("ScreenClippingHost.exe") {
            return false;
        }
        if proc_name.eq_ignore_ascii_case("ShareX.exe") {
            let title = read_window_title(hwnd);
            if title.eq_ignore_ascii_case("Region capture")
                || class_name.contains("RegionCapture")
                || (ex_style & windows_sys::Win32::UI::WindowsAndMessaging::WS_EX_TRANSPARENT) != 0
            {
                return false;
            }
        }

        let mut rc: RECT = zeroed();
        if GetWindowRect(hwnd, &mut rc) == 0 {
            return false;
        }
        let mut w = rc.right - rc.left;
        let mut h = rc.bottom - rc.top;
        if IsIconic(hwnd) != 0 || w < 40 || h < 40 {
            let mut wp: WINDOWPLACEMENT = zeroed();
            wp.length = size_of::<WINDOWPLACEMENT>() as u32;
            if GetWindowPlacement(hwnd, &mut wp) != 0 {
                let r = wp.rcNormalPosition;
                w = r.right - r.left;
                h = r.bottom - r.top;
            }
        }
        if w < 40 || h < 40 {
            return false;
        }

        true
    }
}

#[allow(dead_code)]
pub fn read_visible_window_rect(hwnd: HWND) -> Option<IRect> {
    unsafe {
        if IsWindow(hwnd) == 0 {
            return None;
        }
        let mut rc: RECT = zeroed();
        if GetWindowRect(hwnd, &mut rc) == 0 {
            return None;
        }
        let m = get_extended_frame_margins(hwnd);
        Some(IRect::new(
            rc.left + m.left,
            rc.top + m.top,
            ((rc.right - rc.left) - m.left - m.right).max(1),
            ((rc.bottom - rc.top) - m.top - m.bottom).max(1),
        ))
    }
}

#[allow(dead_code)]
pub fn is_window_zoomed(hwnd: HWND) -> bool {
    unsafe { IsWindow(hwnd) != 0 && IsZoomed(hwnd) != 0 }
}

pub fn read_window_info(hwnd: HWND) -> Option<WindowInfo> {
    unsafe {
        let mut rc: RECT = zeroed();
        if GetWindowRect(hwnd, &mut rc) == 0 {
            return None;
        }
        let mut x = rc.left;
        let mut y = rc.top;
        let mut w = rc.right - rc.left;
        let mut h = rc.bottom - rc.top;

        if IsIconic(hwnd) != 0 || w < 40 || h < 40 {
            let mut wp: WINDOWPLACEMENT = zeroed();
            wp.length = size_of::<WINDOWPLACEMENT>() as u32;
            if GetWindowPlacement(hwnd, &mut wp) != 0 {
                let r = wp.rcNormalPosition;
                if r.right - r.left >= 40 && r.bottom - r.top >= 40 {
                    x = r.left;
                    y = r.top;
                    w = r.right - r.left;
                    h = r.bottom - r.top;
                }
            }
        }

        let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        Some(WindowInfo {
            id: hwnd_to_id(hwnd),
            title: read_window_title(hwnd),
            process_name: read_process_name(hwnd),
            class_name: read_class_name(hwnd),
            rect: IRect::new(x, y, w.max(40), h.max(40)),
            monitor_id: (!hmon.is_null()).then_some(hmon as usize as u64),
            desktop_id: None,
        })
    }
}

pub fn enumerate_manageable_windows(own_pid: u32) -> Vec<WindowInfo> {
    enumerate_manageable_windows_opts(own_pid, false)
}

/// Enumerates all manageable top-level windows across OS virtual desktops on startup
/// (`include_shell_cloaked = true` includes DWM_CLOAKED_SHELL windows hidden on other desktops).
pub fn enumerate_manageable_windows_opts(own_pid: u32, include_shell_cloaked: bool) -> Vec<WindowInfo> {
    struct Ctx {
        own_pid: u32,
        include_shell_cloaked: bool,
        out: Vec<WindowInfo>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = unsafe { &mut *(lparam as *mut Ctx) };
        let ok = if ctx.include_shell_cloaked {
            is_manageable_window(hwnd, ctx.own_pid, true) && !is_non_shell_cloaked(hwnd)
        } else {
            is_manageable_window(hwnd, ctx.own_pid, false)
        };
        if ok {
            if let Some(info) = read_window_info(hwnd) {
                ctx.out.push(info);
            }
        }
        TRUE
    }

    let mut ctx = Ctx {
        own_pid,
        include_shell_cloaked,
        out: Vec::new(),
    };
    unsafe {
        EnumWindows(Some(enum_proc), (&mut ctx as *mut Ctx) as LPARAM);
    }
    ctx.out
}

fn is_non_shell_cloaked(hwnd: HWND) -> bool {
    const DWM_CLOAKED_SHELL: u32 = 0x0000_0002;
    let mut cloaked: u32 = 0;
    let hr = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED as u32,
            &mut cloaked as *mut u32 as *mut _,
            size_of::<u32>() as u32,
        )
    };
    hr == 0 && cloaked != 0 && (cloaked & DWM_CLOAKED_SHELL) == 0
}

#[derive(Debug, Clone, Default)]
struct AppliedState {
    rect: Option<IRect>,
    target_rect: Option<IRect>,
    last_applied_at: Option<Instant>,
    cloaked: bool,
    clipped: Option<IRect>,
    margins: Option<FrameMargins>,
    border_color: Option<u32>,
}

/// Applies computed placements via batched `DeferWindowPos`, `DWMWA_CLOAK`,
/// `SetWindowRgn` edge clipping, and Windows 11 `DWMWA_BORDER_COLOR`.
pub struct WindowApplier {
    states: HashMap<WindowId, AppliedState>,
}

impl WindowApplier {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn last_target_rect(&self, id: WindowId) -> Option<IRect> {
        self.states.get(&id).and_then(|s| s.target_rect)
    }

    #[allow(dead_code)]
    pub fn settled_target_rect(&self, id: WindowId, min_age: Duration) -> Option<(IRect, bool)> {
        let s = self.states.get(&id)?;
        let rect = s.target_rect?;
        let ready = s
            .last_applied_at
            .is_some_and(|t| t.elapsed() >= min_age);
        ready.then_some((rect, s.cloaked))
    }

    pub fn forget(&mut self, id: WindowId) {
        if let Some(prev) = self.states.remove(&id) {
            let hwnd = id_to_hwnd(id);
            unsafe {
                if IsWindow(hwnd) != 0 {
                    if prev.cloaked {
                        set_cloak(hwnd, false);
                    }
                    if prev.clipped.is_some() {
                        SetWindowRgn(hwnd, std::ptr::null_mut(), 1);
                    }
                    if prev.border_color.is_some() {
                        set_dwm_border_color(hwnd, DWMWA_COLOR_DEFAULT);
                    }
                }
            }
        }
    }

    /// Restore all managed windows to clean, visible, and centered positions in monitor work area on exit.
    pub fn restore_all(&mut self, extra_ids: &[WindowId]) {
        let mut ids: Vec<WindowId> = self.states.keys().copied().collect();
        for &id in extra_ids {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        for id in ids {
            let _prev = self.states.remove(&id);
            let hwnd = id_to_hwnd(id);
            unsafe {
                if IsWindow(hwnd) != 0 {
                    set_cloak(hwnd, false);
                    SetWindowRgn(hwnd, std::ptr::null_mut(), 1);
                    set_dwm_border_color(hwnd, DWMWA_COLOR_DEFAULT);

                    if IsIconic(hwnd) == 0 {
                        let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
                        let mut mi: MONITORINFO = zeroed();
                        mi.cbSize = size_of::<MONITORINFO>() as u32;
                        let (wx, wy, ww, wh) = if GetMonitorInfoW(hmon, &mut mi as *mut _ as *mut _) != 0 {
                            (
                                mi.rcWork.left,
                                mi.rcWork.top,
                                (mi.rcWork.right - mi.rcWork.left).max(640),
                                (mi.rcWork.bottom - mi.rcWork.top).max(480),
                            )
                        } else {
                            (0, 0, 1920, 1080)
                        };

                        let mut rc: RECT = zeroed();
                        GetWindowRect(hwnd, &mut rc);
                        let cur_w = rc.right - rc.left;
                        let cur_h = rc.bottom - rc.top;

                        let target_w = if cur_w >= 400 && cur_w <= ww {
                            cur_w
                        } else {
                            (ww * 82 / 100).clamp(640, ww)
                        };
                        let target_h = if cur_h >= 300 && cur_h <= wh {
                            cur_h
                        } else {
                            (wh * 82 / 100).clamp(480, wh)
                        };

                        let target_x = wx + (ww - target_w) / 2;
                        let target_y = wy + (wh - target_h) / 2;

                        SetWindowPos(
                            hwnd,
                            std::ptr::null_mut(),
                            target_x,
                            target_y,
                            target_w,
                            target_h,
                            SWP_NOACTIVATE | SWP_NOZORDER | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
                        );
                    }
                }
            }
        }
    }

    pub fn apply(
        &mut self,
        commands: &[WindowCommand],
        border_colors: Option<(u32, u32)>,
    ) -> Vec<(WindowId, bool)> {
        let mut cloak_updates = Vec::new();
        let mut to_uncloak: Vec<(WindowId, HWND)> = Vec::new();
        let move_count = commands
            .iter()
            .filter(|c| c.visible && c.rect.is_some())
            .count();

        unsafe {
            let mut hdwp: HDWP = if move_count > 0 {
                BeginDeferWindowPos(move_count as i32)
            } else {
                std::ptr::null_mut()
            };

            for cmd in commands {
                let hwnd = id_to_hwnd(cmd.window_id);
                if IsWindow(hwnd) == 0 {
                    continue;
                }

                let state = self.states.entry(cmd.window_id).or_default();

                if !cmd.visible {
                    if !state.cloaked {
                        set_cloak(hwnd, true);
                        state.cloaked = true;
                        cloak_updates.push((cmd.window_id, true));
                    }
                    continue;
                }

                // Restore if maximized before positioning tiled rects.
                if cmd.rect.is_some() && !cmd.is_floating && IsZoomed(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                }

                if IsIconic(hwnd) != 0 {
                    continue;
                }

                if let Some(target) = cmd.rect {
                    let m = if let Some(cached) = state.margins {
                        cached
                    } else if !state.cloaked {
                        let measured = get_extended_frame_margins(hwnd);
                        state.margins = Some(measured);
                        measured
                    } else {
                        FrameMargins::default()
                    };
                    let mut actual = IRect::new(
                        target.x - m.left,
                        target.y - m.top,
                        (target.w + m.left + m.right).max(1),
                        (target.h + m.top + m.bottom).max(1),
                    );
                    if let Some(clip_screen) = cmd.clip_to {
                        let eff_x = (target.x - m.left).max(clip_screen.x);
                        let eff_y = (target.y - m.top).max(clip_screen.y);
                        let eff_r = (target.right() + m.right).min(clip_screen.right());
                        let eff_b = (target.bottom() + m.bottom).min(clip_screen.bottom());
                        actual = IRect::new(
                            eff_x,
                            eff_y,
                            (eff_r - eff_x).max(1),
                            (eff_b - eff_y).max(1),
                        );
                    }

                    let needs_move = state.rect != Some(actual);
                    if needs_move {
                        let size_unchanged = state
                            .rect
                            .is_some_and(|prev| prev.w == actual.w && prev.h == actual.h);
                        let mut flags = SWP_NOZORDER | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS;
                        if size_unchanged {
                            flags |= SWP_NOSIZE;
                        }
                        if !hdwp.is_null() {
                            let next = DeferWindowPos(
                                hdwp,
                                hwnd,
                                std::ptr::null_mut(),
                                actual.x,
                                actual.y,
                                actual.w,
                                actual.h,
                                flags,
                            );
                            if !next.is_null() {
                                hdwp = next;
                            } else {
                                hdwp = std::ptr::null_mut();
                            }
                        }
                        if hdwp.is_null() {
                            windows_sys::Win32::UI::WindowsAndMessaging::SetWindowPos(
                                hwnd,
                                std::ptr::null_mut(),
                                actual.x,
                                actual.y,
                                actual.w,
                                actual.h,
                                flags,
                            );
                        }
                        state.rect = Some(actual);
                        state.last_applied_at = Some(Instant::now());
                    }
                    state.target_rect = Some(target);
                    if state.last_applied_at.is_none() {
                        state.last_applied_at = Some(Instant::now());
                    }

                    // Cross-monitor edge clipping so peek strips never spill onto adjacent screens.
                    let clip_dirty = state.clipped != cmd.clip_to || (needs_move && cmd.clip_to.is_some());
                    if clip_dirty {
                        match cmd.clip_to {
                            Some(clip_screen) => {
                                let local_left = (clip_screen.x - actual.x).max(0);
                                let local_top = (clip_screen.y - actual.y).max(0);
                                let local_right = (clip_screen.right() - actual.x).min(actual.w);
                                let local_bottom = (clip_screen.bottom() - actual.y).min(actual.h);
                                let rgn: HRGN = CreateRectRgn(
                                    local_left,
                                    local_top,
                                    local_right.max(local_left),
                                    local_bottom.max(local_top),
                                );
                                if !rgn.is_null() && SetWindowRgn(hwnd, rgn, 1) == 0 {
                                    DeleteObject(rgn as _);
                                }
                            }
                            None => {
                                SetWindowRgn(hwnd, std::ptr::null_mut(), 1);
                            }
                        }
                        state.clipped = cmd.clip_to;
                    }
                }

                if state.cloaked {
                    state.cloaked = false;
                    to_uncloak.push((cmd.window_id, hwnd));
                }

                // Windows 11 native colored focus border.
                let desired_border = border_colors.map(|(active, inactive)| {
                    if cmd.is_focused {
                        active
                    } else {
                        inactive
                    }
                });
                if state.border_color != desired_border {
                    set_dwm_border_color(hwnd, desired_border.unwrap_or(DWMWA_COLOR_DEFAULT));
                    state.border_color = desired_border;
                }
            }

            if !hdwp.is_null() {
                EndDeferWindowPos(hdwp);
            }

            // Verify post-positioning bounds against clip_to.
            // On Windows 10/11 DWM, DirectComposition applications (Chrome, Edge, VS Code, Discord, etc.)
            // bypass SetWindowRgn. If an application's WM_GETMINMAXINFO minimum track size forces it to
            // spill over the monitor boundary into an adjacent display, cloak it immediately so it NEVER
            // bleeds onto the adjacent monitor.
            for cmd in commands {
                if !cmd.visible || cmd.rect.is_none() {
                    continue;
                }
                if let Some(clip_screen) = cmd.clip_to {
                    let hwnd = id_to_hwnd(cmd.window_id);
                    let mut post_rc: RECT = zeroed();
                    if GetWindowRect(hwnd, &mut post_rc) != 0 {
                        let spills = post_rc.right > clip_screen.right() + 8
                            || post_rc.left < clip_screen.x - 8
                            || post_rc.bottom > clip_screen.bottom() + 8
                            || post_rc.top < clip_screen.y - 8;
                        if spills {
                            set_cloak(hwnd, true);
                            if let Some(state) = self.states.get_mut(&cmd.window_id) {
                                state.cloaked = true;
                            }
                            cloak_updates.push((cmd.window_id, true));
                            to_uncloak.retain(|(wid, _)| *wid != cmd.window_id);
                        }
                    }
                }
            }

            // Uncloak only after EndDeferWindowPos has committed the new screen coordinates.
            for (win_id, hwnd) in to_uncloak {
                set_cloak(hwnd, false);
                cloak_updates.push((win_id, false));
            }
        }

        cloak_updates
    }
}

pub fn set_cloak(hwnd: HWND, cloak: bool) {
    let val: i32 = if cloak { 1 } else { 0 };
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CLOAK as u32,
            &val as *const i32 as *const _,
            size_of::<i32>() as u32,
        );
    }
}

pub fn shell_open_file(path: &str) {
    let verb = to_wide("open");
    let file = to_wide(path);
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        );
    }
}
