use crate::win32::{WM_APP_TRAY, to_wide};
use std::mem::{size_of, zeroed};
use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, GetSystemMetrics, HICON,
    IDI_APPLICATION, IMAGE_ICON, LR_SHARED, LoadIconW, LoadImageW, MF_CHECKED, MF_SEPARATOR,
    MF_STRING, MF_UNCHECKED, SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, SetForegroundWindow,
    TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD, TrackPopupMenu, WM_LBUTTONDBLCLK, WM_RBUTTONUP,
};

pub const APP_ICON_RES_ID: usize = 1;

/// Loads the embedded WinScroller 2D brand icon (`winscroller2d.ico` resource ID 1),
/// falling back to the standard Windows application icon if unavailable.
pub fn load_brand_icon(small: bool) -> HICON {
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let (cx, cy) = if small {
            (GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON))
        } else {
            (GetSystemMetrics(SM_CXICON), GetSystemMetrics(SM_CYICON))
        };
        let handle = LoadImageW(
            hinst,
            APP_ICON_RES_ID as *const u16,
            IMAGE_ICON,
            cx,
            cy,
            LR_SHARED,
        );
        if !handle.is_null() {
            handle as HICON
        } else {
            LoadIconW(std::ptr::null_mut(), IDI_APPLICATION)
        }
    }
}

pub const TRAY_CMD_OVERVIEW: usize = 1001;
pub const TRAY_CMD_PAUSE: usize = 1002;
pub const TRAY_CMD_RETILE: usize = 1003;
pub const TRAY_CMD_OPEN_CONFIG: usize = 1004;
pub const TRAY_CMD_RELOAD_CONFIG: usize = 1005;
pub const TRAY_CMD_EXIT: usize = 1006;
pub const TRAY_CMD_SETTINGS: usize = 1007;
pub const TRAY_CMD_TOGGLE_WS_LAYOUT: usize = 1008;

/// Shows the tray context menu (or handles double-click) without borrowing `AppState`
/// across the modal `TrackPopupMenu` loop, avoiding timer/WinEvent `RefCell` re-entrancy.
pub fn popup_tray_menu(
    hwnd: HWND,
    lparam: isize,
    paused: bool,
    ws_floating: bool,
) -> Option<usize> {
    let msg = (lparam & 0xFFFF) as u32;
    if msg == WM_LBUTTONDBLCLK {
        return Some(TRAY_CMD_OVERVIEW);
    }
    if msg != WM_RBUTTONUP {
        return None;
    }

    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return None;
        }
        let append = |id: usize, label: &str, flags: u32| {
            let w = to_wide(label);
            AppendMenuW(menu, flags, id, w.as_ptr());
        };

        append(TRAY_CMD_OVERVIEW, "2D Grid Overview\tWin+O", MF_STRING);
        append(TRAY_CMD_SETTINGS, "Settings...\tWin+Alt+S", MF_STRING);
        append(
            TRAY_CMD_TOGGLE_WS_LAYOUT,
            "Workspace 2D Lattice Mode\tWin+Alt+L",
            MF_STRING
                | if !ws_floating {
                    MF_CHECKED
                } else {
                    MF_UNCHECKED
                },
        );
        append(
            TRAY_CMD_PAUSE,
            if paused {
                "Resume Tiling\tWin+Shift+P"
            } else {
                "Pause Tiling\tWin+Shift+P"
            },
            MF_STRING | if paused { MF_CHECKED } else { MF_UNCHECKED },
        );
        append(
            TRAY_CMD_RETILE,
            "Retile / Sync Windows\tWin+Shift+R",
            MF_STRING,
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        append(
            TRAY_CMD_OPEN_CONFIG,
            "Open Config (winscroller2d.toml)",
            MF_STRING,
        );
        append(TRAY_CMD_RELOAD_CONFIG, "Reload Config", MF_STRING);
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        append(TRAY_CMD_EXIT, "Exit && Restore Windows", MF_STRING);

        let mut pt: POINT = zeroed();
        GetCursorPos(&mut pt);
        SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RETURNCMD,
            pt.x,
            pt.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        DestroyMenu(menu);
        if cmd > 0 { Some(cmd as usize) } else { None }
    }
}

pub struct TrayIcon {
    hwnd: HWND,
    id: u32,
    added: bool,
}

impl TrayIcon {
    pub fn create(hwnd: HWND) -> Self {
        let mut tray = Self {
            hwnd,
            id: 1,
            added: false,
        };
        let mut data = tray.base_data("WinScroller 2D — Active");
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = WM_APP_TRAY;
        data.hIcon = load_brand_icon(true);
        if unsafe { Shell_NotifyIconW(NIM_ADD, &data) } != 0 {
            tray.added = true;
        }
        tray
    }

    pub fn update_status(&self, paused: bool) {
        if !self.added {
            return;
        }
        let tip = if paused {
            "WinScroller 2D — Paused (Win+Shift+P to resume)"
        } else {
            "WinScroller 2D — Active"
        };
        let mut data = self.base_data(tip);
        data.uFlags = NIF_TIP;
        unsafe {
            Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }

    fn base_data(&self, tip: &str) -> NOTIFYICONDATAW {
        let mut data: NOTIFYICONDATAW = unsafe { zeroed() };
        data.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = self.hwnd;
        data.uID = self.id;
        let wide = to_wide(tip);
        let max_len = data.szTip.len().saturating_sub(1);
        let copy_len = wide.len().min(max_len);
        data.szTip[..copy_len].copy_from_slice(&wide[..copy_len]);
        data
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        if self.added {
            let data = self.base_data("");
            unsafe {
                Shell_NotifyIconW(NIM_DELETE, &data);
            }
        }
    }
}
