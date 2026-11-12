use crate::win32::to_wide;
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem::size_of;
use std::time::Instant;
use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

pub const WM_INPUT_MSG: u32 = 0x00FF;
const RIM_TYPEHID: u32 = 2;
const RID_INPUT: u32 = 0x1000_0003;
const RIDEV_INPUTSINK: u32 = 0x0000_0100;
const RIDI_PREPARSEDDATA: u32 = 0x2000_0005;

const HIDP_INPUT: i32 = 0;
const HID_USAGE_PAGE_GENERIC: u16 = 0x01;
const HID_USAGE_GENERIC_X: u16 = 0x30;
const HID_USAGE_GENERIC_Y: u16 = 0x31;
const HID_USAGE_PAGE_DIGITIZER: u16 = 0x0D;
const HID_USAGE_DIGITIZER_TOUCH_PAD: u16 = 0x05;
const HID_USAGE_DIGITIZER_CONTACT_COUNT: u16 = 0x54;

#[repr(C)]
struct RawInputDevice {
    us_usage_page: u16,
    us_usage: u16,
    dw_flags: u32,
    hwnd_target: HWND,
}

#[repr(C)]
struct RawInputHeader {
    dw_type: u32,
    dw_size: u32,
    h_device: usize,
    w_param: WPARAM,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterRawInputDevices(
        raw_input_devices: *const RawInputDevice,
        num_devices: u32,
        cb_size: u32,
    ) -> i32;
    fn GetRawInputData(
        hraw_input: isize,
        ui_command: u32,
        p_data: *mut c_void,
        pcb_size: *mut u32,
        cb_size_header: u32,
    ) -> u32;
    fn GetRawInputDeviceInfoW(
        h_device: usize,
        ui_command: u32,
        p_data: *mut c_void,
        pcb_size: *mut u32,
    ) -> u32;
}

type FnHidPGetUsageValue = unsafe extern "system" fn(
    report_type: i32,
    usage_page: u16,
    link_collection: u16,
    usage: u16,
    usage_value: *mut u32,
    preparsed_data: *const c_void,
    report: *const u8,
    report_length: u32,
) -> i32;

type FnHidPGetScaledUsageValue = unsafe extern "system" fn(
    report_type: i32,
    usage_page: u16,
    link_collection: u16,
    usage: u16,
    usage_value: *mut i32,
    preparsed_data: *const c_void,
    report: *const u8,
    report_length: u32,
) -> i32;

struct HidApi {
    _mod: usize,
    get_usage_value: FnHidPGetUsageValue,
    get_scaled_usage_value: FnHidPGetScaledUsageValue,
}

impl HidApi {
    fn load() -> Option<Self> {
        unsafe {
            let dll = to_wide("hid.dll");
            let m = LoadLibraryW(dll.as_ptr());
            if m.is_null() {
                return None;
            }
            let a1 = GetProcAddress(m, b"HidP_GetUsageValue\0".as_ptr())?;
            let a2 = GetProcAddress(m, b"HidP_GetScaledUsageValue\0".as_ptr())?;
            Some(Self {
                _mod: m as usize,
                get_usage_value: std::mem::transmute_copy(&a1),
                get_scaled_usage_value: std::mem::transmute_copy(&a2),
            })
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GestureEvent {
    PanDelta { dx: f64, dy: f64 },
    PanCommit { vx: f64, vy: f64 },
    OverviewToggle { open: bool },
}

pub struct TouchpadTracker {
    hid: Option<HidApi>,
    // Stored as `Vec<u64>` so buffered HID preparsed data always satisfies 8-byte alignment.
    preparsed_cache: HashMap<usize, Vec<u64>>,
    active_fingers: u32,
    last_xy: Option<(i32, i32)>,
    last_time: Instant,
    vel: (f64, f64),
    panning_3f: bool,
    swipe4_accum_y: f64,
    swipe4_cooldown_until: Instant,
}

impl Default for TouchpadTracker {
    fn default() -> Self {
        Self {
            hid: HidApi::load(),
            preparsed_cache: HashMap::new(),
            active_fingers: 0,
            last_xy: None,
            last_time: Instant::now(),
            vel: (0.0, 0.0),
            panning_3f: false,
            swipe4_accum_y: 0.0,
            swipe4_cooldown_until: Instant::now(),
        }
    }
}

impl TouchpadTracker {
    pub fn register_sink(&self, hwnd: HWND) -> bool {
        let rid = RawInputDevice {
            us_usage_page: HID_USAGE_PAGE_DIGITIZER,
            us_usage: HID_USAGE_DIGITIZER_TOUCH_PAD,
            dw_flags: RIDEV_INPUTSINK,
            hwnd_target: hwnd,
        };
        unsafe { RegisterRawInputDevices(&rid, 1, size_of::<RawInputDevice>() as u32) != 0 }
    }

    pub fn handle_raw_input(
        &mut self,
        lparam: LPARAM,
        sensitivity: f64,
        three_finger_pan_enabled: bool,
    ) -> Option<GestureEvent> {
        let mut size: u32 = 0;
        let hdr_sz = size_of::<RawInputHeader>() as u32;
        unsafe {
            GetRawInputData(
                lparam as isize,
                RID_INPUT,
                std::ptr::null_mut(),
                &mut size,
                hdr_sz,
            );
        }
        if size < hdr_sz + 8 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        let written = unsafe {
            GetRawInputData(
                lparam as isize,
                RID_INPUT,
                buf.as_mut_ptr() as *mut c_void,
                &mut size,
                hdr_sz,
            )
        };
        if written == u32::MAX || written < hdr_sz + 8 {
            return None;
        }

        let header = unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const RawInputHeader) };
        if header.dw_type != RIM_TYPEHID {
            return None;
        }

        let hid_offset = size_of::<RawInputHeader>();
        if buf.len() < hid_offset + 8 {
            return None;
        }
        let dw_size_hid = u32::from_ne_bytes(buf[hid_offset..hid_offset + 4].try_into().ok()?);
        let dw_count = u32::from_ne_bytes(buf[hid_offset + 4..hid_offset + 8].try_into().ok()?);
        if dw_size_hid == 0 || dw_count == 0 {
            return None;
        }
        let report_start = hid_offset + 8;
        let report_end = report_start + (dw_size_hid as usize);
        if buf.len() < report_end {
            return None;
        }
        let report = &buf[report_start..report_end];

        let preparsed_ptr = self.preparsed_for(header.h_device)?;
        let hid = self.hid.as_ref()?;

        let mut contacts: u32 = 0;
        let status = unsafe {
            (hid.get_usage_value)(
                HIDP_INPUT,
                HID_USAGE_PAGE_DIGITIZER,
                0,
                HID_USAGE_DIGITIZER_CONTACT_COUNT,
                &mut contacts,
                preparsed_ptr,
                report.as_ptr(),
                dw_size_hid,
            )
        };
        if status < 0 {
            return None;
        }

        // Read primary finger position from LinkCollection 1 (falling back to 0).
        let (x, y) = read_touchpad_xy(hid, preparsed_ptr, report)?;

        if contacts == 0 {
            self.active_fingers = 0;
            self.last_xy = None;
            self.swipe4_accum_y = 0.0;
            if self.panning_3f {
                self.panning_3f = false;
                let (vx, vy) = self.vel;
                self.vel = (0.0, 0.0);
                return Some(GestureEvent::PanCommit { vx, vy });
            }
            return None;
        }

        if contacts != self.active_fingers {
            let was_3f = self.panning_3f;
            self.active_fingers = contacts;
            self.last_xy = Some((x, y));
            self.last_time = Instant::now();
            self.swipe4_accum_y = 0.0;
            if was_3f && contacts < 3 {
                self.panning_3f = false;
                let (vx, vy) = self.vel;
                self.vel = (0.0, 0.0);
                return Some(GestureEvent::PanCommit { vx, vy });
            }
            return None;
        }

        let Some((px, py)) = self.last_xy else {
            self.last_xy = Some((x, y));
            self.last_time = Instant::now();
            return None;
        };
        self.last_xy = Some((x, y));

        let now = Instant::now();
        let dt = (now - self.last_time).as_secs_f64().clamp(0.004, 0.1);
        self.last_time = now;

        // Typical Precision Touchpad physical/scaled units map cleanly at ~0.18px per unit.
        let scale = 0.18 * sensitivity.clamp(0.2, 5.0);
        let dx = (x - px) as f64 * scale;
        let dy = (y - py) as f64 * scale;

        if contacts == 3 && three_finger_pan_enabled {
            self.panning_3f = true;
            let inst_vx = dx / dt;
            let inst_vy = dy / dt;
            self.vel = (self.vel.0 * 0.5 + inst_vx * 0.5, self.vel.1 * 0.5 + inst_vy * 0.5);
            if dx.abs() > 0.1 || dy.abs() > 0.1 {
                return Some(GestureEvent::PanDelta { dx, dy });
            }
        } else if contacts == 4 {
            self.swipe4_accum_y += dy;
            if now >= self.swipe4_cooldown_until && self.swipe4_accum_y.abs() >= 72.0 {
                let open = self.swipe4_accum_y < 0.0; // Swipe up opens, swipe down closes.
                self.swipe4_accum_y = 0.0;
                self.swipe4_cooldown_until = now + std::time::Duration::from_millis(450);
                return Some(GestureEvent::OverviewToggle { open });
            }
        }

        None
    }

    fn preparsed_for(&mut self, h_device: usize) -> Option<*const c_void> {
        if !self.preparsed_cache.contains_key(&h_device) {
            let mut sz: u32 = 0;
            unsafe {
                GetRawInputDeviceInfoW(h_device, RIDI_PREPARSEDDATA, std::ptr::null_mut(), &mut sz);
            }
            if sz == 0 || sz > 65536 {
                return None;
            }
            let words = (sz as usize + 7) / 8;
            let mut data = vec![0u64; words];
            let mut buf_bytes = sz;
            let res = unsafe {
                GetRawInputDeviceInfoW(
                    h_device,
                    RIDI_PREPARSEDDATA,
                    data.as_mut_ptr() as *mut c_void,
                    &mut buf_bytes,
                )
            };
            if res == u32::MAX {
                return None;
            }
            self.preparsed_cache.insert(h_device, data);
        }
        self.preparsed_cache
            .get(&h_device)
            .map(|v| v.as_ptr() as *const c_void)
    }
}

fn read_touchpad_xy(hid: &HidApi, preparsed: *const c_void, report: &[u8]) -> Option<(i32, i32)> {
    for link in [1u16, 0u16] {
        let mut sx = 0i32;
        let mut sy = 0i32;
        let ok_x = unsafe {
            (hid.get_scaled_usage_value)(
                HIDP_INPUT,
                HID_USAGE_PAGE_GENERIC,
                link,
                HID_USAGE_GENERIC_X,
                &mut sx,
                preparsed,
                report.as_ptr(),
                report.len() as u32,
            )
        };
        let ok_y = unsafe {
            (hid.get_scaled_usage_value)(
                HIDP_INPUT,
                HID_USAGE_PAGE_GENERIC,
                link,
                HID_USAGE_GENERIC_Y,
                &mut sy,
                preparsed,
                report.as_ptr(),
                report.len() as u32,
            )
        };
        if ok_x >= 0 && ok_y >= 0 {
            return Some((sx, sy));
        }

        let mut ux = 0u32;
        let mut uy = 0u32;
        let rx = unsafe {
            (hid.get_usage_value)(
                HIDP_INPUT,
                HID_USAGE_PAGE_GENERIC,
                link,
                HID_USAGE_GENERIC_X,
                &mut ux,
                preparsed,
                report.as_ptr(),
                report.len() as u32,
            )
        };
        let ry = unsafe {
            (hid.get_usage_value)(
                HIDP_INPUT,
                HID_USAGE_PAGE_GENERIC,
                link,
                HID_USAGE_GENERIC_Y,
                &mut uy,
                preparsed,
                report.as_ptr(),
                report.len() as u32,
            )
        };
        if rx >= 0 && ry >= 0 {
            return Some((ux as i32, uy as i32));
        }
    }
    None
}
