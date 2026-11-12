use crate::win32::window::root_window_from_point;
use std::mem::zeroed;
use std::time::Instant;
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
use winscroller2d_core::WindowId;

pub struct HoverFocusTracker {
    last_pt: Option<(i32, i32)>,
    candidate: Option<WindowId>,
    candidate_since: Instant,
    armed: bool,
}

impl Default for HoverFocusTracker {
    fn default() -> Self {
        Self {
            last_pt: None,
            candidate: None,
            candidate_since: Instant::now(),
            armed: false,
        }
    }
}

impl HoverFocusTracker {
    pub fn reset(&mut self) {
        self.candidate = None;
        self.armed = false;
    }

    /// Polls cursor position and returns `Some(win_id)` when pointer has rested on another
    /// managed window for at least `delay_ms` milliseconds without any mouse button held.
    pub fn poll<F>(
        &mut self,
        delay_ms: u64,
        focused_win: Option<WindowId>,
        is_manageable_active: F,
    ) -> Option<WindowId>
    where
        F: Fn(WindowId) -> bool,
    {
        unsafe {
            let down = ((GetAsyncKeyState(VK_LBUTTON as i32)
                | GetAsyncKeyState(VK_RBUTTON as i32)
                | GetAsyncKeyState(VK_MBUTTON as i32)) as u16
                & 0x8000)
                != 0;
            if down {
                self.candidate = None;
                return None;
            }
        }

        let mut pt: POINT = unsafe { zeroed() };
        if unsafe { GetCursorPos(&mut pt) } == 0 {
            return None;
        }
        let pos = (pt.x, pt.y);
        let moved = self
            .last_pt
            .is_some_and(|prev| (prev.0 - pos.0).abs() > 2 || (prev.1 - pos.1).abs() > 2);
        self.last_pt = Some(pos);

        // Require actual mouse motion after an overview close or reset before auto-focusing peeks.
        if !self.armed {
            if moved {
                self.armed = true;
            } else {
                return None;
            }
        }

        let Some(hit_id) = root_window_from_point(pos.0, pos.1) else {
            self.candidate = None;
            return None;
        };

        if Some(hit_id) == focused_win || !is_manageable_active(hit_id) {
            self.candidate = None;
            return None;
        }

        if self.candidate != Some(hit_id) {
            self.candidate = Some(hit_id);
            self.candidate_since = Instant::now();
            if delay_ms == 0 {
                self.candidate = None;
                return Some(hit_id);
            }
            return None;
        }

        if moved {
            // Reset dwell timer whenever cursor is actively gliding across an edge peek.
            self.candidate_since = Instant::now();
            return None;
        }

        if self.candidate_since.elapsed().as_millis() as u64 >= delay_ms {
            self.candidate = None;
            self.armed = false;
            return Some(hit_id);
        }

        None
    }
}
