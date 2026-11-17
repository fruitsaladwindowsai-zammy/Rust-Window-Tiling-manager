use crate::win32::from_wide;
use std::mem::{size_of, zeroed};
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, TRUE};
use windows_sys::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute, DwmSetWindowAttribute,
};
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW,
};
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
use winscroller2d_core::{IRect, MonitorInfo};

pub const DWMWA_COLOR_DEFAULT: u32 = 0xFFFF_FFFF;

pub fn enable_per_monitor_dpi_v2() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub fn enumerate_monitors() -> Vec<MonitorInfo> {
    let mut list: Vec<MonitorInfo> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(monitor_enum_proc),
            (&mut list as *mut Vec<MonitorInfo>) as LPARAM,
        );
    }
    // Sort left-to-right, top-to-bottom for predictable monitor navigation.
    list.sort_by_key(|m| (m.bounds.x, m.bounds.y));
    list
}

unsafe extern "system" fn monitor_enum_proc(
    hmon: HMONITOR,
    _hdc: HDC,
    _rc: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let list = unsafe { &mut *(lparam as *mut Vec<MonitorInfo>) };
    let mut info: MONITORINFOEXW = unsafe { zeroed() };
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    if unsafe { GetMonitorInfoW(hmon, &mut info as *mut MONITORINFOEXW as *mut _) } != 0 {
        let rc_mon = info.monitorInfo.rcMonitor;
        let rc_work = info.monitorInfo.rcWork;
        list.push(MonitorInfo {
            id: hmon as usize as u64,
            name: from_wide(&info.szDevice),
            bounds: IRect::new(
                rc_mon.left,
                rc_mon.top,
                rc_mon.right - rc_mon.left,
                rc_mon.bottom - rc_mon.top,
            ),
            work_area: IRect::new(
                rc_work.left,
                rc_work.top,
                rc_work.right - rc_work.left,
                rc_work.bottom - rc_work.top,
            ),
        });
    }
    TRUE
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FrameMargins {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// Measures the invisible DWM drop-shadow padding around `hwnd` so `SetWindowPos`
/// places the visible window frame cleanly on the target layout rect.
pub fn get_extended_frame_margins(hwnd: HWND) -> FrameMargins {
    unsafe {
        let mut win_rect: RECT = zeroed();
        let mut ext_rect: RECT = zeroed();
        if GetWindowRect(hwnd, &mut win_rect) == 0 {
            return FrameMargins::default();
        }
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS as u32,
            &mut ext_rect as *mut RECT as *mut _,
            size_of::<RECT>() as u32,
        );
        if hr != 0 {
            return FrameMargins::default();
        }
        FrameMargins {
            left: (ext_rect.left - win_rect.left).clamp(0, 32),
            top: (ext_rect.top - win_rect.top).clamp(0, 32),
            right: (win_rect.right - ext_rect.right).clamp(0, 32),
            bottom: (win_rect.bottom - ext_rect.bottom).clamp(0, 32),
        }
    }
}

pub fn set_dwm_border_color(hwnd: HWND, colorref: u32) {
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR as u32,
            &colorref as *const u32 as *const _,
            size_of::<u32>() as u32,
        );
    }
}
