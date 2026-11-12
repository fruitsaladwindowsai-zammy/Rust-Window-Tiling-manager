pub mod dpi;
pub mod events;
pub mod gestures;
pub mod hotkeys;
pub mod hover;
pub mod icons;
pub mod overview;
pub mod settings_ui;
pub mod tray;
pub mod vdesktop;
pub mod wallpaper;
pub mod window;

use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::Dwm::DwmFlush;
use windows_sys::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};

pub const WM_APP_ACTION: u32 = WM_APP + 1;
pub const WM_APP_WIN_EVENT: u32 = WM_APP + 2;
pub const WM_APP_TRAY: u32 = WM_APP + 3;
pub const WM_APP_IPC_NOTIFY: u32 = WM_APP + 4;
pub const WM_APP_RELOAD_CONFIG: u32 = WM_APP + 5;
pub const WM_APP_OVERVIEW_CMD: u32 = WM_APP + 6;
pub const WM_APP_VSYNC_STEP: u32 = WM_APP + 7;
pub const WM_APP_APPLY_LIVE_CONFIG: u32 = WM_APP + 8;
pub const WM_APP_VDESKTOP_CHANGED: u32 = WM_APP + 9;

pub const TIMER_DISPLAY_SYNC_ID: usize = 2;
pub const TIMER_HOVER_FOCUS_ID: usize = 3;
pub const TIMER_PERSIST_SAVE_ID: usize = 4;

pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn from_wide(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

#[derive(Default)]
struct PacerState {
    active: bool,
    stop: bool,
}

/// Wakes a Win32 window right on each DWM compositor VSync while active, sleeping at 0% CPU when idle.
pub struct VsyncPacer {
    state: Arc<(Mutex<PacerState>, Condvar)>,
    thread: Option<JoinHandle<()>>,
}

impl VsyncPacer {
    pub fn new(hwnd: HWND, msg: u32) -> Self {
        let state = Arc::new((Mutex::new(PacerState::default()), Condvar::new()));
        let state_bg = Arc::clone(&state);
        let hwnd_raw = hwnd as usize;
        let thread = std::thread::Builder::new()
            .name("winscroller2d-vsync".into())
            .spawn(move || {
                let (lock, cvar) = &*state_bg;
                loop {
                    {
                        let mut guard = match lock.lock() {
                            Ok(g) => g,
                            Err(_) => return,
                        };
                        while !guard.active && !guard.stop {
                            guard = match cvar.wait(guard) {
                                Ok(g) => g,
                                Err(_) => return,
                            };
                        }
                        if guard.stop {
                            return;
                        }
                    }

                    let hr = unsafe { DwmFlush() };
                    if hr != 0 {
                        std::thread::sleep(std::time::Duration::from_millis(8));
                    }

                    let still_active = lock.lock().is_ok_and(|g| g.active && !g.stop);
                    if still_active {
                        unsafe {
                            PostMessageW(hwnd_raw as HWND, msg, 0, 0);
                        }
                    }
                }
            })
            .ok();

        Self { state, thread }
    }

    pub fn set_active(&self, active: bool) {
        let (lock, cvar) = &*self.state;
        if let Ok(mut guard) = lock.lock() {
            if guard.active != active {
                guard.active = active;
                if active {
                    cvar.notify_one();
                }
            }
        }
    }

    pub fn is_active(&self) -> bool {
        let (lock, _) = &*self.state;
        lock.lock().is_ok_and(|g| g.active && !g.stop)
    }
}

impl Drop for VsyncPacer {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.state;
        if let Ok(mut guard) = lock.lock() {
            guard.stop = true;
            guard.active = false;
            cvar.notify_all();
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}
