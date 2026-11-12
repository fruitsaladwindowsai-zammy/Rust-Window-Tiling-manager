use crate::win32::WM_APP_WIN_EVENT;
use std::sync::atomic::{AtomicIsize, Ordering};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PostMessageW, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE,
    EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_SHOW, EVENT_OBJECT_UNCLOAKED, EVENT_SYSTEM_FOREGROUND,
    EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MOVESIZEEND, OBJID_WINDOW,
    WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
};

static TARGET_HWND: AtomicIsize = AtomicIsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum WinEventKind {
    Show = EVENT_OBJECT_SHOW,
    Hide = EVENT_OBJECT_HIDE,
    Destroy = EVENT_OBJECT_DESTROY,
    Foreground = EVENT_SYSTEM_FOREGROUND,
    MinimizeStart = EVENT_SYSTEM_MINIMIZESTART,
    MinimizeEnd = EVENT_SYSTEM_MINIMIZEEND,
    MoveSizeEnd = EVENT_SYSTEM_MOVESIZEEND,
    NameChange = EVENT_OBJECT_NAMECHANGE,
    Cloaked = EVENT_OBJECT_CLOAKED,
    Uncloaked = EVENT_OBJECT_UNCLOAKED,
}

impl WinEventKind {
    pub fn from_raw(event: u32) -> Option<Self> {
        match event {
            EVENT_OBJECT_SHOW => Some(Self::Show),
            EVENT_OBJECT_HIDE => Some(Self::Hide),
            EVENT_OBJECT_DESTROY => Some(Self::Destroy),
            EVENT_SYSTEM_FOREGROUND => Some(Self::Foreground),
            EVENT_SYSTEM_MINIMIZESTART => Some(Self::MinimizeStart),
            EVENT_SYSTEM_MINIMIZEEND => Some(Self::MinimizeEnd),
            EVENT_SYSTEM_MOVESIZEEND => Some(Self::MoveSizeEnd),
            EVENT_OBJECT_NAMECHANGE => Some(Self::NameChange),
            EVENT_OBJECT_CLOAKED => Some(Self::Cloaked),
            EVENT_OBJECT_UNCLOAKED => Some(Self::Uncloaked),
            _ => None,
        }
    }
}

pub struct WinEventListener {
    hooks: Vec<HWINEVENTHOOK>,
}

impl WinEventListener {
    pub fn install(target_hwnd: HWND) -> Self {
        TARGET_HWND.store(target_hwnd as isize, Ordering::SeqCst);
        let mut hooks = Vec::new();
        let ranges = [
            (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
            (EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZEEND),
            (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
            (EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE),
            (EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_NAMECHANGE),
            (EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED),
        ];
        for (min_ev, max_ev) in ranges {
            let hook = unsafe {
                SetWinEventHook(
                    min_ev,
                    max_ev,
                    std::ptr::null_mut(),
                    Some(win_event_proc),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                )
            };
            if !hook.is_null() {
                hooks.push(hook);
            }
        }
        Self { hooks }
    }
}

impl Drop for WinEventListener {
    fn drop(&mut self) {
        for hook in self.hooks.drain(..) {
            unsafe {
                UnhookWinEvent(hook);
            }
        }
        TARGET_HWND.store(0, Ordering::SeqCst);
    }
}

unsafe extern "system" fn win_event_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _event_thread: u32,
    _event_time: u32,
) {
    if id_object != OBJID_WINDOW || id_child != 0 || hwnd.is_null() {
        return;
    }
    if WinEventKind::from_raw(event).is_none() {
        return;
    }
    let target = TARGET_HWND.load(Ordering::Relaxed) as HWND;
    if !target.is_null() {
        unsafe {
            PostMessageW(target, WM_APP_WIN_EVENT, event as usize, hwnd as isize);
        }
    }
}
