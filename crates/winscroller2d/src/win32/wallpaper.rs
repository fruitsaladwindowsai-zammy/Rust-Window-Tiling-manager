use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::SystemTime;
use windows_sys::Win32::Foundation::HMODULE;
use windows_sys::Win32::Graphics::Gdi::{COLOR_DESKTOP, GetSysColor};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::UI::WindowsAndMessaging::{SPI_GETDESKWALLPAPER, SystemParametersInfoW};
use winscroller2d_core::OverviewBlurMode;

use crate::win32::to_wide;

const PIXEL_FORMAT_32BPP_PARGB: i32 = 925707;
const INTERP_HIGH_QUALITY_BICUBIC: i32 = 7;
const PIXEL_OFFSET_HALF: i32 = 4;
const UNIT_PIXEL: i32 = 2;

#[repr(C)]
struct GdiplusStartupInput {
    gdiplus_version: u32,
    debug_event_callback: *const c_void,
    suppress_background_thread: i32,
    suppress_external_codecs: i32,
}

type FnGdiplusStartup = unsafe extern "system" fn(
    token: *mut usize,
    input: *const GdiplusStartupInput,
    output: *mut c_void,
) -> i32;
type FnGdipCreateBitmapFromFile =
    unsafe extern "system" fn(filename: *const u16, bitmap: *mut *mut c_void) -> i32;
type FnGdipCreateBitmapFromScan0 = unsafe extern "system" fn(
    width: i32,
    height: i32,
    stride: i32,
    format: i32,
    scan0: *mut u8,
    bitmap: *mut *mut c_void,
) -> i32;
type FnGdipGetImageDimensionU =
    unsafe extern "system" fn(image: *mut c_void, value: *mut u32) -> i32;
type FnGdipDisposeImage = unsafe extern "system" fn(image: *mut c_void) -> i32;
type FnGdipGetImageGraphicsContext =
    unsafe extern "system" fn(image: *mut c_void, graphics: *mut *mut c_void) -> i32;
type FnGdipDeleteGraphics = unsafe extern "system" fn(graphics: *mut c_void) -> i32;
type FnGdipSetMode = unsafe extern "system" fn(graphics: *mut c_void, mode: i32) -> i32;
type FnGdipDrawImageRectRectI = unsafe extern "system" fn(
    graphics: *mut c_void,
    image: *mut c_void,
    dstx: i32,
    dsty: i32,
    dstwidth: i32,
    dstheight: i32,
    srcx: i32,
    srcy: i32,
    srcwidth: i32,
    srcheight: i32,
    srcunit: i32,
    image_attributes: *const c_void,
    callback: *const c_void,
    callback_data: *const c_void,
) -> i32;

struct GdiplusApi {
    _module: usize,
    _token: usize,
    create_bitmap_from_file: FnGdipCreateBitmapFromFile,
    create_bitmap_from_scan0: FnGdipCreateBitmapFromScan0,
    get_image_width: FnGdipGetImageDimensionU,
    get_image_height: FnGdipGetImageDimensionU,
    dispose_image: FnGdipDisposeImage,
    get_image_graphics_context: FnGdipGetImageGraphicsContext,
    delete_graphics: FnGdipDeleteGraphics,
    set_interpolation_mode: FnGdipSetMode,
    set_pixel_offset_mode: FnGdipSetMode,
    draw_image_rect_rect_i: FnGdipDrawImageRectRectI,
}

unsafe impl Send for GdiplusApi {}
unsafe impl Sync for GdiplusApi {}

static GDIPLUS: OnceLock<Option<GdiplusApi>> = OnceLock::new();

unsafe fn load_proc<T>(module: HMODULE, name: &[u8]) -> Option<T> {
    let addr = unsafe { GetProcAddress(module, name.as_ptr()) }?;
    Some(unsafe { std::mem::transmute_copy(&addr) })
}

fn gdiplus() -> Option<&'static GdiplusApi> {
    GDIPLUS
        .get_or_init(|| unsafe {
            let dll = to_wide("gdiplus.dll");
            let module = LoadLibraryW(dll.as_ptr());
            if module.is_null() {
                return None;
            }
            let startup: FnGdiplusStartup = load_proc(module, b"GdiplusStartup\0")?;
            let create_bitmap_from_file: FnGdipCreateBitmapFromFile =
                load_proc(module, b"GdipCreateBitmapFromFile\0")?;
            let create_bitmap_from_scan0: FnGdipCreateBitmapFromScan0 =
                load_proc(module, b"GdipCreateBitmapFromScan0\0")?;
            let get_image_width: FnGdipGetImageDimensionU =
                load_proc(module, b"GdipGetImageWidth\0")?;
            let get_image_height: FnGdipGetImageDimensionU =
                load_proc(module, b"GdipGetImageHeight\0")?;
            let dispose_image: FnGdipDisposeImage = load_proc(module, b"GdipDisposeImage\0")?;
            let get_image_graphics_context: FnGdipGetImageGraphicsContext =
                load_proc(module, b"GdipGetImageGraphicsContext\0")?;
            let delete_graphics: FnGdipDeleteGraphics = load_proc(module, b"GdipDeleteGraphics\0")?;
            let set_interpolation_mode: FnGdipSetMode =
                load_proc(module, b"GdipSetInterpolationMode\0")?;
            let set_pixel_offset_mode: FnGdipSetMode =
                load_proc(module, b"GdipSetPixelOffsetMode\0")?;
            let draw_image_rect_rect_i: FnGdipDrawImageRectRectI =
                load_proc(module, b"GdipDrawImageRectRectI\0")?;

            let mut token: usize = 0;
            let input = GdiplusStartupInput {
                gdiplus_version: 1,
                debug_event_callback: std::ptr::null(),
                suppress_background_thread: 0,
                suppress_external_codecs: 0,
            };
            if startup(&mut token, &input, std::ptr::null_mut()) != 0 {
                return None;
            }
            Some(GdiplusApi {
                _module: module as usize,
                _token: token,
                create_bitmap_from_file,
                create_bitmap_from_scan0,
                get_image_width,
                get_image_height,
                dispose_image,
                get_image_graphics_context,
                delete_graphics,
                set_interpolation_mode,
                set_pixel_offset_mode,
                draw_image_rect_rect_i,
            })
        })
        .as_ref()
}

pub fn query_wallpaper_path() -> Option<PathBuf> {
    let mut buf = [0u16; 520];
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buf.len() as u32,
            buf.as_mut_ptr() as *mut c_void,
            0,
        )
    };
    if ok != 0 {
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        if len > 0 {
            let p = PathBuf::from(String::from_utf16_lossy(&buf[..len]));
            if p.exists() {
                return Some(p);
            }
        }
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        let fallback = PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Themes")
            .join("TranscodedWallpaper");
        if fallback.exists() {
            return Some(fallback);
        }
    }
    None
}

fn desktop_bg_pixel() -> u32 {
    let colorref = unsafe { GetSysColor(COLOR_DESKTOP) };
    let r = colorref & 0xFF;
    let g = (colorref >> 8) & 0xFF;
    let b = (colorref >> 16) & 0xFF;
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

pub struct WallpaperCache {
    w: i32,
    h: i32,
    path: Option<PathBuf>,
    mtime: Option<SystemTime>,
    desktop_rgb: u32,
    sharp: Vec<u32>,
    blur: Vec<u32>,
}

impl Default for WallpaperCache {
    fn default() -> Self {
        Self {
            w: 0,
            h: 0,
            path: None,
            mtime: None,
            desktop_rgb: 0,
            sharp: Vec::new(),
            blur: Vec::new(),
        }
    }
}

impl WallpaperCache {
    pub fn clear(&mut self) {
        self.sharp.clear();
        self.sharp.shrink_to_fit();
        self.blur.clear();
        self.blur.shrink_to_fit();
        self.w = 0;
        self.h = 0;
        self.path = None;
        self.mtime = None;
    }

    pub fn ensure(&mut self, w: i32, h: i32) {
        let w = w.max(1);
        let h = h.max(1);
        let path = query_wallpaper_path();
        let mtime = path
            .as_ref()
            .and_then(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
        let desktop_rgb = desktop_bg_pixel();

        if self.w == w
            && self.h == h
            && self.path == path
            && self.mtime == mtime
            && (path.is_some() || self.desktop_rgb == desktop_rgb)
            && self.sharp.len() == (w * h) as usize
            && self.blur.len() == (w * h) as usize
        {
            return;
        }

        self.w = w;
        self.h = h;
        self.path = path.clone();
        self.mtime = mtime;
        self.desktop_rgb = desktop_rgb;

        let loaded = path
            .as_deref()
            .and_then(|p| decode_cover_wallpaper(p, w, h, desktop_rgb));
        self.sharp = loaded.unwrap_or_else(|| make_solid_gradient_bg(w, h, desktop_rgb));
        self.blur = build_gaussian_blur(&self.sharp, w, h);
    }

    pub fn blit_to(&self, dst: &mut [u32], blur_t: f64, mode: OverviewBlurMode, frost_alpha: u8) {
        let len = dst.len().min(self.sharp.len()).min(self.blur.len());
        if len == 0 {
            return;
        }
        let bt = blur_t.clamp(0.0, 1.0);
        if bt <= 0.005 || matches!(mode, OverviewBlurMode::None) {
            dst[..len].copy_from_slice(&self.sharp[..len]);
            return;
        }

        let tint_alpha = match mode {
            OverviewBlurMode::PureBlur | OverviewBlurMode::None => 0u32,
            OverviewBlurMode::AcrylicClear => (frost_alpha as u32).min(48),
        };
        let eff_tint = ((tint_alpha as f64) * bt).round() as u32;

        if bt >= 0.995 && eff_tint == 0 {
            dst[..len].copy_from_slice(&self.blur[..len]);
            return;
        }

        let b_w = (bt * 256.0).round() as u32;
        let s_w = 256 - b_w;

        if eff_tint == 0 {
            for i in 0..len {
                let sp = self.sharp[i];
                let bp = self.blur[i];
                let r = (((sp >> 16) & 0xFF) * s_w + ((bp >> 16) & 0xFF) * b_w) >> 8;
                let g = (((sp >> 8) & 0xFF) * s_w + ((bp >> 8) & 0xFF) * b_w) >> 8;
                let b = ((sp & 0xFF) * s_w + (bp & 0xFF) * b_w) >> 8;
                dst[i] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            }
        } else {
            let inv_t = 255 - eff_tint;
            let white_add = 255 * eff_tint;
            for i in 0..len {
                let sp = self.sharp[i];
                let bp = self.blur[i];
                let r0 = (((sp >> 16) & 0xFF) * s_w + ((bp >> 16) & 0xFF) * b_w) >> 8;
                let g0 = (((sp >> 8) & 0xFF) * s_w + ((bp >> 8) & 0xFF) * b_w) >> 8;
                let b0 = ((sp & 0xFF) * s_w + (bp & 0xFF) * b_w) >> 8;
                let r = (r0 * inv_t + white_add) / 255;
                let g = (g0 * inv_t + white_add) / 255;
                let b = (b0 * inv_t + white_add) / 255;
                dst[i] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
            }
        }
    }
}

fn decode_cover_wallpaper(
    path: &std::path::Path,
    w: i32,
    h: i32,
    bg_pixel: u32,
) -> Option<Vec<u32>> {
    let api = gdiplus()?;
    let path_str = path.to_str()?;
    let path_w = to_wide(path_str);

    let mut src_img: *mut c_void = std::ptr::null_mut();
    if unsafe { (api.create_bitmap_from_file)(path_w.as_ptr(), &mut src_img) } != 0
        || src_img.is_null()
    {
        return None;
    }

    let mut iw = 0u32;
    let mut ih = 0u32;
    let dim_ok = unsafe {
        (api.get_image_width)(src_img, &mut iw) == 0
            && (api.get_image_height)(src_img, &mut ih) == 0
    };
    if !dim_ok || iw == 0 || ih == 0 {
        unsafe {
            (api.dispose_image)(src_img);
        }
        return None;
    }

    let iw_f = iw as f64;
    let ih_f = ih as f64;
    let target_ratio = w as f64 / h as f64;
    let img_ratio = iw_f / ih_f;
    let (sx, sy, sw, sh) = if img_ratio > target_ratio {
        let crop_w = (ih_f * target_ratio).round().clamp(1.0, iw_f) as i32;
        let offset_x = ((iw as i32 - crop_w) / 2).max(0);
        (offset_x, 0, crop_w, ih as i32)
    } else {
        let crop_h = (iw_f / target_ratio).round().clamp(1.0, ih_f) as i32;
        let offset_y = ((ih as i32 - crop_h) / 2).max(0);
        (0, offset_y, iw as i32, crop_h)
    };

    let mut out = vec![bg_pixel; (w * h) as usize];
    let mut dst_img: *mut c_void = std::ptr::null_mut();
    let create_res = unsafe {
        (api.create_bitmap_from_scan0)(
            w,
            h,
            w * 4,
            PIXEL_FORMAT_32BPP_PARGB,
            out.as_mut_ptr() as *mut u8,
            &mut dst_img,
        )
    };
    if create_res != 0 || dst_img.is_null() {
        unsafe {
            (api.dispose_image)(src_img);
        }
        return None;
    }

    let mut gfx: *mut c_void = std::ptr::null_mut();
    if unsafe { (api.get_image_graphics_context)(dst_img, &mut gfx) } == 0 && !gfx.is_null() {
        unsafe {
            (api.set_interpolation_mode)(gfx, INTERP_HIGH_QUALITY_BICUBIC);
            (api.set_pixel_offset_mode)(gfx, PIXEL_OFFSET_HALF);
            (api.draw_image_rect_rect_i)(
                gfx,
                src_img,
                0,
                0,
                w,
                h,
                sx,
                sy,
                sw,
                sh,
                UNIT_PIXEL,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            );
            (api.delete_graphics)(gfx);
        }
    } else {
        unsafe {
            (api.dispose_image)(dst_img);
            (api.dispose_image)(src_img);
        }
        return None;
    }

    unsafe {
        (api.dispose_image)(dst_img);
        (api.dispose_image)(src_img);
    }

    for px in &mut out {
        *px |= 0xFF00_0000;
    }
    Some(out)
}

fn make_solid_gradient_bg(w: i32, h: i32, base: u32) -> Vec<u32> {
    let br = ((base >> 16) & 0xFF) as i32;
    let bg = ((base >> 8) & 0xFF) as i32;
    let bb = (base & 0xFF) as i32;
    let mut out = vec![base; (w * h) as usize];
    for y in 0..h {
        let fy = (y as f64 / h.max(1) as f64) - 0.5;
        let row = (y * w) as usize;
        for x in 0..w {
            let fx = (x as f64 / w.max(1) as f64) - 0.5;
            let boost = ((1.0 - (fx * fx + fy * fy) * 1.4).max(0.0) * 18.0).round() as i32;
            let r = (br + boost).clamp(0, 255) as u32;
            let g = (bg + boost).clamp(0, 255) as u32;
            let b = (bb + boost).clamp(0, 255) as u32;
            out[row + x as usize] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        }
    }
    out
}

fn build_gaussian_blur(src: &[u32], w: i32, h: i32) -> Vec<u32> {
    let bw = (w / 4).max(160).min(w);
    let bh = (h / 4).max(90).min(h);
    let mut small = downsample_box(src, w, h, bw, bh);
    let mut scratch = vec![0u32; small.len()];

    // 3 separable box-blur passes approximate a true isotropic Gaussian blur cleanly.
    let radius = (bw / 72).clamp(5, 11);
    for _ in 0..3 {
        box_blur_h(&small, &mut scratch, bw, bh, radius);
        box_blur_v(&scratch, &mut small, bw, bh, radius);
    }

    upscale_bilinear_dither(&small, bw, bh, w, h)
}

fn downsample_box(src: &[u32], sw: i32, sh: i32, dw: i32, dh: i32) -> Vec<u32> {
    let mut out = vec![0u32; (dw * dh) as usize];
    for dy in 0..dh {
        let y0 = (dy * sh) / dh;
        let y1 = (((dy + 1) * sh) / dh).max(y0 + 1).min(sh);
        let row_out = (dy * dw) as usize;
        for dx in 0..dw {
            let x0 = (dx * sw) / dw;
            let x1 = (((dx + 1) * sw) / dw).max(x0 + 1).min(sw);
            let mut sr = 0u32;
            let mut sg = 0u32;
            let mut sb = 0u32;
            let mut cnt = 0u32;
            for sy in y0..y1 {
                let row_in = (sy * sw) as usize;
                for sx in x0..x1 {
                    let px = src[row_in + sx as usize];
                    sr += (px >> 16) & 0xFF;
                    sg += (px >> 8) & 0xFF;
                    sb += px & 0xFF;
                    cnt += 1;
                }
            }
            let r = (sr + cnt / 2) / cnt;
            let g = (sg + cnt / 2) / cnt;
            let b = (sb + cnt / 2) / cnt;
            out[row_out + dx as usize] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        }
    }
    out
}

fn box_blur_h(src: &[u32], dst: &mut [u32], w: i32, h: i32, r: i32) {
    let win = (r * 2 + 1) as u32;
    let half = win / 2;
    for y in 0..h {
        let row = (y * w) as usize;
        let mut sr = 0u32;
        let mut sg = 0u32;
        let mut sb = 0u32;
        for dx in -r..=r {
            let px = src[row + dx.clamp(0, w - 1) as usize];
            sr += (px >> 16) & 0xFF;
            sg += (px >> 8) & 0xFF;
            sb += px & 0xFF;
        }
        for x in 0..w {
            let cr = (sr + half) / win;
            let cg = (sg + half) / win;
            let cb = (sb + half) / win;
            dst[row + x as usize] = 0xFF00_0000 | (cr << 16) | (cg << 8) | cb;

            let add_p = src[row + (x + r + 1).clamp(0, w - 1) as usize];
            let sub_p = src[row + (x - r).clamp(0, w - 1) as usize];
            sr = sr + ((add_p >> 16) & 0xFF) - ((sub_p >> 16) & 0xFF);
            sg = sg + ((add_p >> 8) & 0xFF) - ((sub_p >> 8) & 0xFF);
            sb = sb + (add_p & 0xFF) - (sub_p & 0xFF);
        }
    }
}

fn box_blur_v(src: &[u32], dst: &mut [u32], w: i32, h: i32, r: i32) {
    let win = (r * 2 + 1) as u32;
    let half = win / 2;
    for x in 0..w {
        let mut sr = 0u32;
        let mut sg = 0u32;
        let mut sb = 0u32;
        for dy in -r..=r {
            let py = dy.clamp(0, h - 1);
            let px = src[(py * w + x) as usize];
            sr += (px >> 16) & 0xFF;
            sg += (px >> 8) & 0xFF;
            sb += px & 0xFF;
        }
        for y in 0..h {
            let cr = (sr + half) / win;
            let cg = (sg + half) / win;
            let cb = (sb + half) / win;
            dst[(y * w + x) as usize] = 0xFF00_0000 | (cr << 16) | (cg << 8) | cb;

            let add_y = (y + r + 1).clamp(0, h - 1);
            let sub_y = (y - r).clamp(0, h - 1);
            let add_p = src[(add_y * w + x) as usize];
            let sub_p = src[(sub_y * w + x) as usize];
            sr = sr + ((add_p >> 16) & 0xFF) - ((sub_p >> 16) & 0xFF);
            sg = sg + ((add_p >> 8) & 0xFF) - ((sub_p >> 8) & 0xFF);
            sb = sb + (add_p & 0xFF) - (sub_p & 0xFF);
        }
    }
}

fn upscale_bilinear_dither(src: &[u32], sw: i32, sh: i32, dw: i32, dh: i32) -> Vec<u32> {
    const BAYER: [[i32; 2]; 2] = [[0, 2], [3, 1]];
    let mut dst = vec![0u32; (dw * dh) as usize];
    let sx_scale = (sw as f64) / (dw.max(1) as f64);
    let sy_scale = (sh as f64) / (dh.max(1) as f64);

    let mut xs: Vec<(usize, usize, i32)> = Vec::with_capacity(dw as usize);
    for dx in 0..dw {
        let fx = ((dx as f64 + 0.5) * sx_scale - 0.5).clamp(0.0, (sw - 1) as f64);
        let x0 = fx.floor() as i32;
        let x1 = (x0 + 1).min(sw - 1);
        let tx = ((fx - x0 as f64) * 256.0).round() as i32;
        xs.push((x0 as usize, x1 as usize, tx));
    }

    for dy in 0..dh {
        let fy = ((dy as f64 + 0.5) * sy_scale - 0.5).clamp(0.0, (sh - 1) as f64);
        let y0 = fy.floor() as i32;
        let y1 = (y0 + 1).min(sh - 1);
        let ty = ((fy - y0 as f64) * 256.0).round() as i32;
        let ity = 256 - ty;

        let row0 = (y0 * sw) as usize;
        let row1 = (y1 * sw) as usize;
        let row_out = (dy * dw) as usize;
        let d_row = BAYER[(dy & 1) as usize];

        for dx in 0..dw {
            let (x0, x1, tx) = xs[dx as usize];
            let itx = 256 - tx;
            let p00 = src[row0 + x0];
            let p10 = src[row0 + x1];
            let p01 = src[row1 + x0];
            let p11 = src[row1 + x1];

            let w00 = itx * ity;
            let w10 = tx * ity;
            let w01 = itx * ty;
            let w11 = tx * ty;
            let dither = d_row[(dx & 1) as usize];

            let r = ((((p00 >> 16) & 0xFF) as i32 * w00
                + ((p10 >> 16) & 0xFF) as i32 * w10
                + ((p01 >> 16) & 0xFF) as i32 * w01
                + ((p11 >> 16) & 0xFF) as i32 * w11
                + (dither << 14))
                >> 16)
                .clamp(0, 255) as u32;
            let g = ((((p00 >> 8) & 0xFF) as i32 * w00
                + ((p10 >> 8) & 0xFF) as i32 * w10
                + ((p01 >> 8) & 0xFF) as i32 * w01
                + ((p11 >> 8) & 0xFF) as i32 * w11
                + (dither << 14))
                >> 16)
                .clamp(0, 255) as u32;
            let b = (((p00 & 0xFF) as i32 * w00
                + (p10 & 0xFF) as i32 * w10
                + (p01 & 0xFF) as i32 * w01
                + (p11 & 0xFF) as i32 * w11
                + (dither << 14))
                >> 16)
                .clamp(0, 255) as u32;

            dst[row_out + dx as usize] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
        }
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mean_luma(pixels: &[u32]) -> f64 {
        let sum: u64 = pixels
            .iter()
            .map(|&p| (((p >> 16) & 0xFF) + ((p >> 8) & 0xFF) + (p & 0xFF)) as u64)
            .sum();
        sum as f64 / (pixels.len() as f64 * 3.0)
    }

    #[test]
    fn gaussian_blur_preserves_brightness_without_white_or_dark_veil() {
        let w = 320;
        let h = 180;
        let mut cache = WallpaperCache::default();
        cache.ensure(w, h);
        assert_eq!(cache.sharp.len(), (w * h) as usize);
        assert_eq!(cache.blur.len(), (w * h) as usize);

        let mut out = vec![0u32; (w * h) as usize];
        cache.blit_to(&mut out, 1.0, OverviewBlurMode::PureBlur, 0);

        assert!(out.iter().all(|&p| (p >> 24) == 0xFF));
        let sharp_mean = mean_luma(&cache.sharp);
        let blur_mean = mean_luma(&out);
        assert!(
            (sharp_mean - blur_mean).abs() < 4.0,
            "pure wallpaper blur shifted brightness: sharp={sharp_mean:.2}, blur={blur_mean:.2}"
        );

        if query_wallpaper_path().is_some() {
            // Confirm blurred wallpaper is real wallpaper pixels rather than solid white/black.
            let min_luma = out
                .iter()
                .map(|&p| (((p >> 16) & 0xFF) + ((p >> 8) & 0xFF) + (p & 0xFF)) / 3)
                .min()
                .unwrap_or(0);
            let max_luma = out
                .iter()
                .map(|&p| (((p >> 16) & 0xFF) + ((p >> 8) & 0xFF) + (p & 0xFF)) / 3)
                .max()
                .unwrap_or(0);
            assert!(
                max_luma > min_luma,
                "expected real blurred wallpaper variation, got flat luma={min_luma}"
            );
            assert!(
                blur_mean < 245.0,
                "wallpaper unexpectedly washed to white: {blur_mean}"
            );
        }
    }
}
