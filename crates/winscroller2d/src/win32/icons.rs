use crate::win32::to_wide;
use std::mem::{size_of, zeroed};
use windows_sys::Win32::Foundation::{CloseHandle, HWND};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, HBITMAP, HDC, SelectObject,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows_sys::Win32::UI::Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    DI_NORMAL, DestroyIcon, DrawIconEx, GCLP_HICON, GCLP_HICONSM, GetClassLongPtrW,
    GetWindowThreadProcessId, HICON, ICON_BIG, ICON_SMALL, ICON_SMALL2, SMTO_ABORTIFHUNG,
    SendMessageTimeoutW, WM_GETICON,
};
use winscroller2d_core::IRect;

const EXTRACT_SIZE: i32 = 48;

#[derive(Clone)]
pub struct IconImage {
    pub w: i32,
    pub h: i32,
    pub pixels: Vec<u32>,
}

struct ScratchDib {
    hdc: HDC,
    bmp: HBITMAP,
    old: isize,
    bits: *mut u32,
    len: usize,
}

impl ScratchDib {
    unsafe fn new(w: i32, h: i32) -> Option<Self> {
        let hdc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
        if hdc.is_null() {
            return None;
        }
        let mut bmi: BITMAPINFO = unsafe { zeroed() };
        bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut ptr: *mut core::ffi::c_void = std::ptr::null_mut();
        let bmp = unsafe {
            CreateDIBSection(hdc, &bmi, DIB_RGB_COLORS, &mut ptr, std::ptr::null_mut(), 0)
        };
        if bmp.is_null() || ptr.is_null() {
            unsafe {
                DeleteDC(hdc);
            }
            return None;
        }
        let old = unsafe { SelectObject(hdc, bmp as _) as isize };
        Some(Self {
            hdc,
            bmp,
            old,
            bits: ptr as *mut u32,
            len: (w * h) as usize,
        })
    }

    unsafe fn slice_mut(&mut self) -> &mut [u32] {
        unsafe { std::slice::from_raw_parts_mut(self.bits, self.len) }
    }
}

impl Drop for ScratchDib {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.hdc, self.old as _);
            DeleteObject(self.bmp as _);
            DeleteDC(self.hdc);
        }
    }
}

fn query_msg_icon(hwnd: HWND, kind: usize) -> Option<HICON> {
    let mut res: usize = 0;
    let ok =
        unsafe { SendMessageTimeoutW(hwnd, WM_GETICON, kind, 0, SMTO_ABORTIFHUNG, 35, &mut res) };
    if ok != 0 && res != 0 {
        Some(res as HICON)
    } else {
        None
    }
}

fn query_process_shell_icon(hwnd: HWND) -> Option<HICON> {
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, &mut pid);
    }
    if pid == 0 {
        return None;
    }
    let proc_handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if proc_handle.is_null() {
        return None;
    }
    let mut buf = [0u16; 520];
    let mut len = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(proc_handle, 0, buf.as_mut_ptr(), &mut len) };
    unsafe {
        CloseHandle(proc_handle);
    }
    if ok == 0 || len == 0 {
        return None;
    }
    let exe = String::from_utf16_lossy(&buf[..len as usize]);
    let exe_w = to_wide(&exe);
    let mut info: SHFILEINFOW = unsafe { zeroed() };
    let ret = unsafe {
        SHGetFileInfoW(
            exe_w.as_ptr(),
            0,
            &mut info,
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if ret != 0 && !info.hIcon.is_null() {
        Some(info.hIcon)
    } else {
        None
    }
}

/// Extracts a high-resolution premultiplied ARGB icon for `hwnd`, supporting modern
/// 32-bit alpha icons, legacy masked icons, and UWP/shell-fallback executables.
pub fn extract_window_icon(hwnd: HWND) -> Option<IconImage> {
    let (hicon, owned) = if let Some(h) = query_msg_icon(hwnd, ICON_BIG as usize)
        .or_else(|| query_msg_icon(hwnd, ICON_SMALL2 as usize))
        .or_else(|| query_msg_icon(hwnd, ICON_SMALL as usize))
    {
        (h, false)
    } else {
        let cls = unsafe {
            let big = GetClassLongPtrW(hwnd, GCLP_HICON) as HICON;
            if !big.is_null() {
                big
            } else {
                GetClassLongPtrW(hwnd, GCLP_HICONSM) as HICON
            }
        };
        if !cls.is_null() {
            (cls, false)
        } else {
            (query_process_shell_icon(hwnd)?, true)
        }
    };

    let result = unsafe { rasterize_hicon(hicon, EXTRACT_SIZE) };
    if owned {
        unsafe {
            DestroyIcon(hicon);
        }
    }
    result
}

unsafe fn rasterize_hicon(hicon: HICON, size: i32) -> Option<IconImage> {
    let mut black = unsafe { ScratchDib::new(size, size)? };
    let mut white = unsafe { ScratchDib::new(size, size)? };

    unsafe {
        black.slice_mut().fill(0x0000_0000);
        white.slice_mut().fill(0x00FF_FFFF);
        let ok_b = DrawIconEx(
            black.hdc,
            0,
            0,
            hicon,
            size,
            size,
            0,
            std::ptr::null_mut(),
            DI_NORMAL,
        );
        let ok_w = DrawIconEx(
            white.hdc,
            0,
            0,
            hicon,
            size,
            size,
            0,
            std::ptr::null_mut(),
            DI_NORMAL,
        );
        if ok_b == 0 && ok_w == 0 {
            return None;
        }
    }

    let b_px = unsafe { black.slice_mut() };
    let w_px = unsafe { white.slice_mut() };
    let mut out = vec![0u32; (size * size) as usize];

    for i in 0..out.len() {
        let bp = b_px[i];
        let wp = w_px[i];
        let br = ((bp >> 16) & 0xFF) as i32;
        let bg = ((bp >> 8) & 0xFF) as i32;
        let bb = (bp & 0xFF) as i32;

        let wr = ((wp >> 16) & 0xFF) as i32;
        let wg = ((wp >> 8) & 0xFF) as i32;
        let wb = (wp & 0xFF) as i32;

        let ar = 255 - (wr - br).clamp(0, 255);
        let ag = 255 - (wg - bg).clamp(0, 255);
        let ab = 255 - (wb - bb).clamp(0, 255);
        let mut a = ((ar + ag + ab + 1) / 3).clamp(0, 255);
        let native_a = ((bp >> 24) & 0xFF) as i32;
        if native_a > a {
            a = native_a;
        }
        if a == 0 {
            out[i] = 0;
        } else {
            let pr = br.clamp(0, a) as u32;
            let pg = bg.clamp(0, a) as u32;
            let pb = bb.clamp(0, a) as u32;
            out[i] = ((a as u32) << 24) | (pr << 16) | (pg << 8) | pb;
        }
    }

    Some(IconImage {
        w: size,
        h: size,
        pixels: out,
    })
}

/// Renders a soft circular ambient shadow beneath a floating icon so it stands out on bright or dark windows.
pub fn draw_soft_shadow_circle(
    dst: &mut [u32],
    dst_w: i32,
    dst_h: i32,
    cx: i32,
    cy: i32,
    radius: i32,
    max_alpha: u8,
) {
    if max_alpha == 0 || radius <= 0 {
        return;
    }
    let x0 = (cx - radius).max(0);
    let x1 = (cx + radius).min(dst_w - 1);
    let y0 = (cy - radius).max(0);
    let y1 = (cy + radius).min(dst_h - 1);
    let r_f = radius as f64;

    for y in y0..=y1 {
        let dy = (y - cy) as f64;
        let row = (y * dst_w) as usize;
        for x in x0..=x1 {
            let dx = (x - cx) as f64;
            let d = (dx * dx + dy * dy).sqrt();
            if d >= r_f {
                continue;
            }
            let t = 1.0 - (d / r_f);
            let a = ((max_alpha as f64) * t * t).round() as u32;
            if a == 0 {
                continue;
            }
            let existing = dst[row + x as usize];
            if existing == 0 {
                dst[row + x as usize] =
                    (a << 24) | ((6 * a / 255) << 16) | ((8 * a / 255) << 8) | (14 * a / 255);
            } else {
                let inv = 255 - a;
                let ea = (existing >> 24) & 0xFF;
                let er = (existing >> 16) & 0xFF;
                let eg = (existing >> 8) & 0xFF;
                let eb = existing & 0xFF;
                let na = (a + (ea * inv + 127) / 255).min(255);
                let nr = ((6 * a + 127) / 255 + (er * inv + 127) / 255).min(na);
                let ng = ((8 * a + 127) / 255 + (eg * inv + 127) / 255).min(na);
                let nb = ((14 * a + 127) / 255 + (eb * inv + 127) / 255).min(na);
                dst[row + x as usize] = (na << 24) | (nr << 16) | (ng << 8) | nb;
            }
        }
    }
}

/// Bilinearly draws a premultiplied ARGB `IconImage` onto `dst` within `rect`, faded by `alpha`.
pub fn draw_icon_scaled(
    dst: &mut [u32],
    dst_w: i32,
    dst_h: i32,
    rect: IRect,
    icon: &IconImage,
    alpha: u8,
) {
    if alpha == 0 || rect.w <= 0 || rect.h <= 0 || icon.w <= 0 || icon.h <= 0 {
        return;
    }
    let sx_scale = (icon.w as f64) / (rect.w as f64);
    let sy_scale = (icon.h as f64) / (rect.h as f64);
    let a_scale = alpha as u32;

    for ly in 0..rect.h {
        let dy = rect.y + ly;
        if dy < 0 || dy >= dst_h {
            continue;
        }
        let fy = ((ly as f64 + 0.5) * sy_scale - 0.5).clamp(0.0, (icon.h - 1) as f64);
        let y0 = fy.floor() as i32;
        let y1 = (y0 + 1).min(icon.h - 1);
        let ty = ((fy - y0 as f64) * 256.0).round() as u32;
        let ity = 256 - ty;

        let row0 = (y0 * icon.w) as usize;
        let row1 = (y1 * icon.w) as usize;
        let d_row = (dy * dst_w) as usize;

        for lx in 0..rect.w {
            let dx = rect.x + lx;
            if dx < 0 || dx >= dst_w {
                continue;
            }
            let fx = ((lx as f64 + 0.5) * sx_scale - 0.5).clamp(0.0, (icon.w - 1) as f64);
            let x0 = fx.floor() as i32;
            let x1 = (x0 + 1).min(icon.w - 1);
            let tx = ((fx - x0 as f64) * 256.0).round() as u32;
            let itx = 256 - tx;

            let w00 = itx * ity;
            let w10 = tx * ity;
            let w01 = itx * ty;
            let w11 = tx * ty;

            let p00 = icon.pixels[row0 + x0 as usize];
            let p10 = icon.pixels[row0 + x1 as usize];
            let p01 = icon.pixels[row1 + x0 as usize];
            let p11 = icon.pixels[row1 + x1 as usize];

            let sa0 = (((p00 >> 24) & 0xFF) * w00
                + ((p10 >> 24) & 0xFF) * w10
                + ((p01 >> 24) & 0xFF) * w01
                + ((p11 >> 24) & 0xFF) * w11)
                >> 16;
            let sa = ((sa0 * a_scale) / 255).min(255);
            if sa == 0 {
                continue;
            }

            let sr0 = (((p00 >> 16) & 0xFF) * w00
                + ((p10 >> 16) & 0xFF) * w10
                + ((p01 >> 16) & 0xFF) * w01
                + ((p11 >> 16) & 0xFF) * w11)
                >> 16;
            let sg0 = (((p00 >> 8) & 0xFF) * w00
                + ((p10 >> 8) & 0xFF) * w10
                + ((p01 >> 8) & 0xFF) * w01
                + ((p11 >> 8) & 0xFF) * w11)
                >> 16;
            let sb0 =
                ((p00 & 0xFF) * w00 + (p10 & 0xFF) * w10 + (p01 & 0xFF) * w01 + (p11 & 0xFF) * w11)
                    >> 16;

            let sr = ((sr0 * a_scale) / 255).min(sa);
            let sg = ((sg0 * a_scale) / 255).min(sa);
            let sb = ((sb0 * a_scale) / 255).min(sa);

            let inv = 255 - sa;
            let dp = dst[d_row + dx as usize];
            let da = (dp >> 24) & 0xFF;
            let dr = (dp >> 16) & 0xFF;
            let dg = (dp >> 8) & 0xFF;
            let db = dp & 0xFF;

            let oa = (sa + (da * inv + 127) / 255).min(255);
            let or = (sr + (dr * inv + 127) / 255).min(oa);
            let og = (sg + (dg * inv + 127) / 255).min(oa);
            let ob = (sb + (db * inv + 127) / 255).min(oa);
            dst[d_row + dx as usize] = (oa << 24) | (or << 16) | (og << 8) | ob;
        }
    }
}
