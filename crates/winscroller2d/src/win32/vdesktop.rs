use crate::win32::{to_wide, WM_APP_VDESKTOP_CHANGED};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::{BOOL, HWND};
use windows_sys::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegNotifyChangeKeyValue, RegOpenKeyExW, RegQueryValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_NOTIFY, KEY_READ, REG_BINARY, REG_NOTIFY_CHANGE_LAST_SET,
};
use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;

const VDESKTOP_REG_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\VirtualDesktops";

const CLSID_VIRTUAL_DESKTOP_MANAGER: GUID = GUID {
    data1: 0xAA50_9086,
    data2: 0x5CA9,
    data3: 0x4C25,
    data4: [0x8F, 0x95, 0x58, 0x9D, 0x3C, 0x07, 0xB4, 0x8A],
};

const IID_IVIRTUAL_DESKTOP_MANAGER: GUID = GUID {
    data1: 0xA5CD_92FF,
    data2: 0x29BE,
    data3: 0x454C,
    data4: [0x8D, 0x04, 0xD8, 0x28, 0x79, 0xFB, 0x3F, 0x1B],
};

#[repr(C)]
struct IVirtualDesktopManagerVtbl {
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,
    is_window_on_current_virtual_desktop:
        unsafe extern "system" fn(*mut c_void, HWND, *mut BOOL) -> i32,
    get_window_desktop_id: unsafe extern "system" fn(*mut c_void, HWND, *mut GUID) -> i32,
    move_window_to_desktop: unsafe extern "system" fn(*mut c_void, HWND, *const GUID) -> i32,
}

#[repr(C)]
struct IVirtualDesktopManager {
    vtbl: *const IVirtualDesktopManagerVtbl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsDesktopState {
    pub desktops: Vec<String>,
    pub current: String,
}

/// Manages COM `IVirtualDesktopManager` queries and Explorer `VirtualDesktops` registry state.
pub struct VirtualDesktopTracker {
    com_inited: bool,
    vdm: *mut IVirtualDesktopManager,
    watcher_stop: Arc<AtomicBool>,
    _watcher: Option<JoinHandle<()>>,
}

impl VirtualDesktopTracker {
    pub fn new(ctrl_hwnd: HWND) -> Self {
        let mut com_inited = false;
        let mut vdm: *mut IVirtualDesktopManager = std::ptr::null_mut();

        unsafe {
            let hr_init = CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32);
            // S_OK (0) or S_FALSE (1) require matching CoUninitialize; RPC_E_CHANGED_MODE does not.
            if hr_init == 0 || hr_init == 1 {
                com_inited = true;
            }
            let mut ptr: *mut c_void = std::ptr::null_mut();
            let hr = CoCreateInstance(
                &CLSID_VIRTUAL_DESKTOP_MANAGER,
                std::ptr::null_mut(),
                CLSCTX_ALL,
                &IID_IVIRTUAL_DESKTOP_MANAGER,
                &mut ptr,
            );
            if hr >= 0 && !ptr.is_null() {
                vdm = ptr as *mut IVirtualDesktopManager;
            }
        }

        let watcher_stop = Arc::new(AtomicBool::new(false));
        let watcher = if !ctrl_hwnd.is_null() {
            spawn_reg_watcher(ctrl_hwnd, Arc::clone(&watcher_stop))
        } else {
            None
        };

        Self {
            com_inited,
            vdm,
            watcher_stop,
            _watcher: watcher,
        }
    }

    pub fn current_state(&self) -> OsDesktopState {
        read_registry_desktop_state()
    }

    pub fn is_window_on_current_desktop(&self, hwnd: HWND) -> Option<bool> {
        if self.vdm.is_null() || hwnd.is_null() {
            return None;
        }
        let mut on_cur: BOOL = 0;
        let hr = unsafe {
            let vtbl = &*(*self.vdm).vtbl;
            (vtbl.is_window_on_current_virtual_desktop)(
                self.vdm as *mut c_void,
                hwnd,
                &mut on_cur,
            )
        };
        if hr >= 0 {
            Some(on_cur != 0)
        } else {
            None
        }
    }

    pub fn window_desktop_id(&self, hwnd: HWND) -> Option<String> {
        if self.vdm.is_null() || hwnd.is_null() {
            return None;
        }
        let mut guid = GUID {
            data1: 0,
            data2: 0,
            data3: 0,
            data4: [0; 8],
        };
        let hr = unsafe {
            let vtbl = &*(*self.vdm).vtbl;
            (vtbl.get_window_desktop_id)(self.vdm as *mut c_void, hwnd, &mut guid)
        };
        if hr >= 0 && !is_zero_guid(&guid) {
            Some(guid_to_string(&guid))
        } else {
            None
        }
    }
}

impl Drop for VirtualDesktopTracker {
    fn drop(&mut self) {
        self.watcher_stop.store(true, Ordering::Relaxed);
        if !self.vdm.is_null() {
            unsafe {
                let vtbl = &*(*self.vdm).vtbl;
                (vtbl.release)(self.vdm as *mut c_void);
            }
            self.vdm = std::ptr::null_mut();
        }
        if self.com_inited {
            unsafe {
                CoUninitialize();
            }
            self.com_inited = false;
        }
    }
}

fn spawn_reg_watcher(ctrl_hwnd: HWND, stop: Arc<AtomicBool>) -> Option<JoinHandle<()>> {
    let hwnd_raw = ctrl_hwnd as usize;
    std::thread::Builder::new()
        .name("winscroller2d-vdesktop-watch".into())
        .spawn(move || {
            let subkey = to_wide(VDESKTOP_REG_SUBKEY);
            while !stop.load(Ordering::Relaxed) {
                let mut hkey: HKEY = std::ptr::null_mut();
                let open_rc = unsafe {
                    RegOpenKeyExW(
                        HKEY_CURRENT_USER,
                        subkey.as_ptr(),
                        0,
                        KEY_NOTIFY,
                        &mut hkey,
                    )
                };
                if open_rc != 0 || hkey.is_null() {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    continue;
                }
                while !stop.load(Ordering::Relaxed) {
                    let rc = unsafe {
                        RegNotifyChangeKeyValue(
                            hkey,
                            0,
                            REG_NOTIFY_CHANGE_LAST_SET,
                            std::ptr::null_mut(),
                            0,
                        )
                    };
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    if rc == 0 {
                        unsafe {
                            PostMessageW(hwnd_raw as HWND, WM_APP_VDESKTOP_CHANGED, 0, 0);
                        }
                    } else {
                        break;
                    }
                }
                unsafe {
                    RegCloseKey(hkey);
                }
            }
        })
        .ok()
}

pub fn read_registry_desktop_state() -> OsDesktopState {
    let mut desktops = Vec::new();
    let mut current = String::new();

    let subkey = to_wide(VDESKTOP_REG_SUBKEY);
    let mut hkey: HKEY = std::ptr::null_mut();
    let rc = unsafe {
        RegOpenKeyExW(HKEY_CURRENT_USER, subkey.as_ptr(), 0, KEY_READ, &mut hkey)
    };
    if rc == 0 && !hkey.is_null() {
        if let Some(raw_ids) = reg_read_binary(hkey, "VirtualDesktopIDs") {
            desktops = parse_guid_list(&raw_ids);
        }
        if let Some(raw_cur) = reg_read_binary(hkey, "CurrentVirtualDesktop") {
            if let Some(id) = parse_single_guid(&raw_cur) {
                current = id;
            }
        }
        unsafe {
            RegCloseKey(hkey);
        }
    }

    if current.is_empty() {
        current = desktops
            .first()
            .cloned()
            .unwrap_or_else(|| "default".to_string());
    }
    if !desktops.iter().any(|d| d == &current) {
        desktops.push(current.clone());
    }

    OsDesktopState { desktops, current }
}

fn reg_read_binary(hkey: HKEY, name: &str) -> Option<Vec<u8>> {
    let wname = to_wide(name);
    let mut kind: u32 = 0;
    let mut len: u32 = 0;
    let rc1 = unsafe {
        RegQueryValueExW(
            hkey,
            wname.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            std::ptr::null_mut(),
            &mut len,
        )
    };
    if rc1 != 0 || kind != REG_BINARY || len == 0 || len > 65536 {
        return None;
    }
    let mut buf = vec![0u8; len as usize];
    let mut len2 = len;
    let rc2 = unsafe {
        RegQueryValueExW(
            hkey,
            wname.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            buf.as_mut_ptr(),
            &mut len2,
        )
    };
    if rc2 != 0 || kind != REG_BINARY {
        return None;
    }
    buf.truncate(len2 as usize);
    Some(buf)
}

fn parse_single_guid(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 16 {
        return None;
    }
    let guid = GUID {
        data1: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
        data2: u16::from_le_bytes(bytes[4..6].try_into().ok()?),
        data3: u16::from_le_bytes(bytes[6..8].try_into().ok()?),
        data4: bytes[8..16].try_into().ok()?,
    };
    if is_zero_guid(&guid) {
        return None;
    }
    Some(guid_to_string(&guid))
}

fn parse_guid_list(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for chunk in bytes.chunks_exact(16) {
        if let Some(id) = parse_single_guid(chunk) {
            if !out.iter().any(|x| x == &id) {
                out.push(id);
            }
        }
    }
    out
}

fn is_zero_guid(g: &GUID) -> bool {
    g.data1 == 0 && g.data2 == 0 && g.data3 == 0 && g.data4 == [0; 8]
}

fn guid_to_string(g: &GUID) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        g.data1,
        g.data2,
        g.data3,
        g.data4[0],
        g.data4[1],
        g.data4[2],
        g.data4[3],
        g.data4[4],
        g.data4[5],
        g.data4[6],
        g.data4[7]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_registry_guid_bytes_consistently() {
        let mut raw = [0u8; 32];
        raw[0..4].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        raw[4..6].copy_from_slice(&0x9ABCu16.to_le_bytes());
        raw[6..8].copy_from_slice(&0xDEF0u16.to_le_bytes());
        raw[8..16].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        // Second GUID identical to verify deduplication.
        let copy = raw[0..16].to_vec();
        raw[16..32].copy_from_slice(&copy);

        let list = parse_guid_list(&raw);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], "12345678-9abc-def0-0102-030405060708");
    }
}
