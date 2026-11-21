use crate::win32::icons::{
    IconImage, draw_icon_scaled, draw_soft_shadow_circle, extract_window_icon,
};
use crate::win32::wallpaper::WallpaperCache;
use crate::win32::{VsyncPacer, WM_APP_OVERVIEW_CMD, WM_APP_VSYNC_STEP, to_wide};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::mem::{size_of, zeroed};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, TRUE, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{
    DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY, DWM_TNP_RECTDESTINATION,
    DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE, DwmFlush, DwmQueryThumbnailSourceSize,
    DwmRegisterThumbnail, DwmUnregisterThumbnail, DwmUpdateThumbnailProperties,
};
use windows_sys::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, ANTIALIASED_QUALITY, BI_RGB, BITMAPINFO, BITMAPINFOHEADER,
    BLENDFUNCTION, BeginPaint, BitBlt, CLIP_DEFAULT_PRECIS, CreateCompatibleDC, CreateDIBSection,
    CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH, DIB_RGB_COLORS, DT_CENTER, DT_END_ELLIPSIS,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, EndPaint, FW_BOLD,
    FW_NORMAL, HBITMAP, HDC, HFONT, InvalidateRect, OUT_DEFAULT_PRECIS, PAINTSTRUCT,
    RDW_INVALIDATE, RDW_UPDATENOW, RedrawWindow, SRCCOPY, SelectObject, SetBkMode, SetTextColor,
    TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_RETURN,
    VK_RIGHT, VK_SHIFT, VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, CreateWindowExW, DefWindowProcW, DestroyWindow, GetForegroundWindow,
    GetWindowRect, GetWindowThreadProcessId, HWND_TOPMOST, IDC_ARROW, IsIconic, IsWindow,
    LoadCursorW, PostMessageW, RegisterClassExW, SW_HIDE, SW_SHOW, SWP_NOACTIVATE,
    SWP_SHOWWINDOW, SetForegroundWindow, SetWindowPos, ShowWindow, ULW_ALPHA, UpdateLayeredWindow,
    WM_CAPTURECHANGED, WM_ERASEBKGND, WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT, WM_RBUTTONDOWN, WNDCLASSEXW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use winscroller2d_core::{IRect, OverviewBlurMode, Spring1D, WorkspaceSnapshot};

const OVERVIEW_CLASS: &str = "WinScroller2dOverviewWindow";
const OVERVIEW_ICONS_CLASS: &str = "WinScroller2dOverviewIcons";

#[derive(Debug, Clone, Copy)]
pub struct OverviewOptions {
    pub blur_mode: OverviewBlurMode,
    pub alpha: u8,
    pub stiffness: f64,
    pub damping_ratio: f64,
    pub min_w_frac: f64,
    pub min_h_frac: f64,
    pub autoscroll_speed: f64,
}

impl Default for OverviewOptions {
    fn default() -> Self {
        Self {
            blur_mode: OverviewBlurMode::PureBlur,
            alpha: 0,
            stiffness: 260.0,
            damping_ratio: 0.72,
            min_w_frac: 0.25,
            min_h_frac: 0.25,
            autoscroll_speed: 900.0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum OverviewCommand {
    Place {
        window_id: u64,
        col: i32,
        row: i32,
        stack: bool,
    },
    SplitHorizontal {
        window_id: u64,
        target_window_id: u64,
        on_left: bool,
    },
    CloseWindow {
        window_id: u64,
    },
    FinishClose {
        jump: Option<(i32, i32, Option<u64>)>,
    },
}

static COMMAND_QUEUE: OnceLock<Mutex<VecDeque<OverviewCommand>>> = OnceLock::new();

fn queue() -> &'static Mutex<VecDeque<OverviewCommand>> {
    COMMAND_QUEUE.get_or_init(|| Mutex::new(VecDeque::new()))
}

pub fn drain_overview_commands() -> Vec<OverviewCommand> {
    let mut out = Vec::new();
    if let Ok(mut q) = queue().lock() {
        while let Some(cmd) = q.pop_front() {
            out.push(cmd);
        }
    }
    out
}

fn emit_command(controller_hwnd: HWND, cmd: OverviewCommand) {
    if let Ok(mut q) = queue().lock() {
        q.push_back(cmd);
    }
    if !controller_hwnd.is_null() {
        unsafe {
            PostMessageW(controller_hwnd, WM_APP_OVERVIEW_CMD, 0, 0);
        }
    }
}

#[derive(Clone)]
struct CellBox {
    rect: IRect,
    col: i32,
    row: i32,
    window_id: Option<u64>,
    stack_len: usize,
    focused: bool,
}

#[derive(Clone, Default)]
struct CardSpring {
    x: Spring1D,
    y: Spring1D,
    w: Spring1D,
    h: Spring1D,
    start_rect: Option<IRect>,
}

impl CardSpring {
    fn current_rect(&self) -> IRect {
        IRect::new(
            self.x.current.round() as i32,
            self.y.current.round() as i32,
            (self.w.current.round() as i32).max(16),
            (self.h.current.round() as i32).max(16),
        )
    }

    fn set_target(&mut self, r: IRect, animate: bool) {
        self.x.set_target(r.x as f64, animate);
        self.y.set_target(r.y as f64, animate);
        self.w.set_target(r.w as f64, animate);
        self.h.set_target(r.h as f64, animate);
    }

    fn step(&mut self, dt: f64, stiffness: f64, damping: f64) -> bool {
        let ax = self.x.step(dt, stiffness, damping, 0.35);
        let ay = self.y.step(dt, stiffness, damping, 0.35);
        let aw = self.w.step(dt, stiffness, damping, 0.35);
        let ah = self.h.step(dt, stiffness, damping, 0.35);
        ax || ay || aw || ah
    }
}

#[derive(Clone)]
struct DragState {
    window_id: u64,
    source_col: i32,
    source_row: i32,
    start_x: i32,
    start_y: i32,
    cur_x: i32,
    cur_y: i32,
    active: bool,
}

pub(crate) struct DibSection {
    pub(crate) hdc: HDC,
    bmp: HBITMAP,
    old_bmp: isize,
    pub(crate) bits: *mut u32,
    pub(crate) w: i32,
    pub(crate) h: i32,
}

impl DibSection {
    pub(crate) unsafe fn acquire(hdc_screen: HDC, w: i32, h: i32) -> Option<Self> {
        let hdc = unsafe { CreateCompatibleDC(hdc_screen) };
        if hdc.is_null() {
            return None;
        }
        let Some((bmp, bits)) = (unsafe { create_argb_dib(hdc_screen, w, h) }) else {
            unsafe {
                DeleteDC(hdc);
            }
            return None;
        };
        let old_bmp = unsafe { SelectObject(hdc, bmp as _) as isize };
        Some(Self {
            hdc,
            bmp,
            old_bmp,
            bits,
            w,
            h,
        })
    }
}

impl Drop for DibSection {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.hdc, self.old_bmp as _);
            DeleteObject(self.bmp as _);
            DeleteDC(self.hdc);
        }
    }
}

/// Reusable 32-bit ARGB offscreen surface + scratch text buffer so painting allocates 0 bytes per frame.
pub(crate) struct CachedSurface {
    pub(crate) main: DibSection,
    pub(crate) scratch: DibSection,
    pub(crate) title_font: HFONT,
    pub(crate) sub_font: HFONT,
}

impl CachedSurface {
    pub(crate) unsafe fn acquire(hdc_screen: HDC, w: i32, h: i32) -> Option<Self> {
        let main = unsafe { DibSection::acquire(hdc_screen, w, h)? };
        let scratch = unsafe { DibSection::acquire(hdc_screen, w.min(640).max(256), 48)? };
        let title_font = unsafe { make_font(15, true) };
        let sub_font = unsafe { make_font(13, false) };

        Some(Self {
            main,
            scratch,
            title_font,
            sub_font,
        })
    }
}

impl Drop for CachedSurface {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.title_font as _);
            DeleteObject(self.sub_font as _);
        }
    }
}

struct OverviewState {
    controller_hwnd: HWND,
    overlay_hwnd: HWND,
    icon_hwnd: HWND,
    snapshot: Option<WorkspaceSnapshot>,
    bounds: IRect,
    return_rect: IRect,
    boxes: Vec<CellBox>,
    thumbs: HashMap<u64, isize>,
    thumb_sizes: HashMap<u64, SIZE>,
    last_thumb_props: HashMap<u64, (RECT, u8)>,
    icons: HashMap<u64, IconImage>,
    hero_window_id: Option<u64>,
    card_springs: HashMap<u64, CardSpring>,
    hover_springs: HashMap<(i32, i32), Spring1D>,
    close_springs: HashMap<(i32, i32), Spring1D>,
    open_spring: Spring1D,
    empty_fade_spring: Spring1D,
    scroll_x: Spring1D,
    scroll_y: Spring1D,
    max_scroll: (f64, f64),
    content_size: (i32, i32),
    closing_jump: Option<Option<(i32, i32, Option<u64>)>>,
    opts: OverviewOptions,
    pacer: VsyncPacer,
    last_tick: Instant,
    cursor: (i32, i32),
    hover_coord: Option<(i32, i32)>,
    hover_close_win: Option<u64>,
    drag: Option<DragState>,
    pending_empty_click: Option<(i32, i32)>,
    pending_close: Option<(i32, i32, u64)>,
    wallpaper: WallpaperCache,
    surface: Option<DibSection>,
    icon_surface: Option<CachedSurface>,
}

thread_local! {
    static OVERVIEW_STATE: RefCell<Option<OverviewState>> = const { RefCell::new(None) };
}

fn with_overview<R>(f: impl FnOnce(&mut OverviewState) -> R) -> Option<R> {
    OVERVIEW_STATE.with(|cell| {
        let Ok(mut guard) = cell.try_borrow_mut() else {
            return None;
        };
        guard.as_mut().map(f)
    })
}

pub struct OverviewWindow {
    hwnd: HWND,
    icon_hwnd: HWND,
    visible: bool,
}

impl OverviewWindow {
    pub fn create(controller_hwnd: HWND) -> Result<Self, String> {
        let hinst = unsafe { GetModuleHandleW(std::ptr::null()) };
        let class_w = to_wide(OVERVIEW_CLASS);
        let title_w = to_wide("WinScroller 2D Overview");

        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(overview_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: std::ptr::null_mut(),
            hCursor: unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) },
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_w.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        unsafe {
            RegisterClassExW(&wc);
        }

        let icon_class_w = to_wide(OVERVIEW_ICONS_CLASS);
        let icon_wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(icons_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: icon_class_w.as_ptr(),
            hIconSm: std::ptr::null_mut(),
        };
        unsafe {
            RegisterClassExW(&icon_wc);
        }

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class_w.as_ptr(),
                title_w.as_ptr(),
                WS_POPUP,
                0,
                0,
                1280,
                720,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinst,
                std::ptr::null(),
            )
        };
        if hwnd.is_null() {
            return Err("Failed to create 2D Overview window".into());
        }

        let icon_title_w = to_wide("WinScroller 2D Overview Icons");
        let icon_hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE,
                icon_class_w.as_ptr(),
                icon_title_w.as_ptr(),
                WS_POPUP,
                0,
                0,
                1280,
                720,
                hwnd,
                std::ptr::null_mut(),
                hinst,
                std::ptr::null(),
            )
        };
        if icon_hwnd.is_null() {
            unsafe {
                DestroyWindow(hwnd);
            }
            return Err("Failed to create Overview icon overlay window".into());
        }

        let pacer = VsyncPacer::new(hwnd, WM_APP_VSYNC_STEP);

        OVERVIEW_STATE.with(|cell| {
            if let Ok(mut guard) = cell.try_borrow_mut() {
                *guard = Some(OverviewState {
                    controller_hwnd,
                    overlay_hwnd: hwnd,
                    icon_hwnd,
                    snapshot: None,
                    bounds: IRect::new(0, 0, 1280, 720),
                    return_rect: IRect::new(8, 8, 1264, 704),
                    boxes: Vec::new(),
                    thumbs: HashMap::new(),
                    thumb_sizes: HashMap::new(),
                    last_thumb_props: HashMap::new(),
                    icons: HashMap::new(),
                    hero_window_id: None,
                    card_springs: HashMap::new(),
                    hover_springs: HashMap::new(),
                    close_springs: HashMap::new(),
                    open_spring: Spring1D::default(),
                    empty_fade_spring: Spring1D::default(),
                    scroll_x: Spring1D::default(),
                    scroll_y: Spring1D::default(),
                    max_scroll: (0.0, 0.0),
                    content_size: (1280, 720),
                    closing_jump: None,
                    opts: OverviewOptions::default(),
                    pacer,
                    last_tick: Instant::now(),
                    cursor: (0, 0),
                    hover_coord: None,
                    hover_close_win: None,
                    drag: None,
                    pending_empty_click: None,
                    pending_close: None,
                    wallpaper: WallpaperCache::default(),
                    surface: None,
                    icon_surface: None,
                });
            }
        });

        Ok(Self {
            hwnd,
            icon_hwnd,
            visible: false,
        })
    }

    /// Phase 1: Register live DWM thumbnails and capture start rects BEFORE desktop windows are cloaked.
    pub fn prepare_show(
        &mut self,
        monitor_bounds: IRect,
        return_rect: IRect,
        ws: WorkspaceSnapshot,
        opts: OverviewOptions,
    ) {
        let mut start_rects: HashMap<u64, IRect> = HashMap::new();
        for w in &ws.windows {
            let hwnd = w.id as usize as HWND;
            let mut rc: RECT = unsafe { zeroed() };
            let r = if !w.cloaked && unsafe { GetWindowRect(hwnd, &mut rc) != 0 } {
                IRect::new(
                    rc.left - monitor_bounds.x,
                    rc.top - monitor_bounds.y,
                    (rc.right - rc.left).max(32),
                    (rc.bottom - rc.top).max(32),
                )
            } else {
                let dx = (w.col - ws.camera_col) * (monitor_bounds.w / 2);
                let dy = (w.row - ws.camera_row) * (monitor_bounds.h / 2);
                IRect::new(
                    dx + return_rect.x,
                    dy + return_rect.y,
                    return_rect.w,
                    return_rect.h,
                )
            };
            start_rects.insert(w.id, r);
        }

        let hero_win = ws
            .windows
            .iter()
            .find(|w| w.focused)
            .or_else(|| {
                ws.windows
                    .iter()
                    .find(|w| w.col == ws.camera_col && w.row == ws.camera_row)
            })
            .or_else(|| ws.windows.first())
            .map(|w| w.id);

        with_overview(|st| {
            st.cursor = (ws.camera_col, ws.camera_row);
            st.bounds = monitor_bounds;
            st.return_rect = return_rect;
            st.snapshot = Some(ws);
            st.drag = None;
            st.pending_empty_click = None;
            st.pending_close = None;
            st.hover_coord = None;
            st.hover_close_win = None;
            st.closing_jump = None;
            st.hero_window_id = hero_win;
            st.opts = OverviewOptions {
                stiffness: opts.stiffness.max(10.0),
                damping_ratio: opts.damping_ratio.clamp(0.25, 2.0),
                min_w_frac: opts.min_w_frac.clamp(0.05, 0.60),
                min_h_frac: opts.min_h_frac.clamp(0.05, 0.60),
                autoscroll_speed: opts.autoscroll_speed.clamp(0.0, 5000.0),
                ..opts
            };
            st.last_tick = Instant::now();
            st.card_springs.clear();
            st.hover_springs.clear();
            st.close_springs.clear();
            st.icons.clear();

            rebuild_boxes(st);
            st.scroll_x.snap_to(0.0);
            st.scroll_y.snap_to(0.0);
            ensure_coord_visible(st, st.cursor, false);

            let sx = st.scroll_x.current.round() as i32;
            let sy = st.scroll_y.current.round() as i32;
            for b in &st.boxes {
                let mut hs = Spring1D::default();
                hs.snap_to(1.0);
                st.hover_springs.insert((b.col, b.row), hs);
                if let Some(win_id) = b.window_id {
                    let mut cs = CardSpring::default();
                    cs.set_target(b.rect, false);
                    if Some(win_id) == hero_win {
                        // Active hero window animates backward from full screen (`return_rect`) to reveal all others.
                        let virt_start = start_rects
                            .get(&win_id)
                            .copied()
                            .map(|sr| IRect::new(sr.x + sx, sr.y + sy, sr.w, sr.h))
                            .unwrap_or(IRect::new(
                                return_rect.x + sx,
                                return_rect.y + sy,
                                return_rect.w,
                                return_rect.h,
                            ));
                        cs.start_rect = Some(virt_start);
                    } else {
                        // Sibling windows emerge gently forward from slightly behind center.
                        cs.start_rect = Some(scale_around_center(b.rect, 0.80));
                    }
                    st.card_springs.insert(win_id, cs);
                }
            }
            sync_hover_targets(st);
            sync_icons(st);
            st.wallpaper.ensure(monitor_bounds.w, monitor_bounds.h);
            st.empty_fade_spring.snap_to(0.0);
            st.open_spring.snap_to(0.0);
            st.open_spring.set_target(1.0, true);
            sync_thumbnails(st);
            update_animated_thumbnails(st);
        });
        unsafe {
            // Position self.hwnd over the monitor bounds immediately with SWP_NOACTIVATE | SWP_SHOWWINDOW
            // so desktop apps uncloaking in apply_frame_now are completely obscured behind Overview.
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                monitor_bounds.x,
                monitor_bounds.y,
                monitor_bounds.w,
                monitor_bounds.h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }

    /// Phase 2: Show the blurred wallpaper overview AFTER desktop apps have been positioned.
    pub fn reveal(&mut self, monitor_bounds: IRect) {
        with_overview(|st| {
            st.wallpaper.ensure(monitor_bounds.w, monitor_bounds.h);
            st.last_tick = Instant::now();
            st.pacer.set_active(true);
            sync_thumbnails(st);
            update_animated_thumbnails(st);
        });
        unsafe {
            let _ = DwmFlush();
            SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                monitor_bounds.x,
                monitor_bounds.y,
                monitor_bounds.w,
                monitor_bounds.h,
                SWP_NOACTIVATE,
            );
            ShowWindow(self.hwnd, SW_SHOW);
            force_foreground(self.hwnd);
            SetWindowPos(
                self.icon_hwnd,
                HWND_TOPMOST,
                monitor_bounds.x,
                monitor_bounds.y,
                monitor_bounds.w,
                monitor_bounds.h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            RedrawWindow(
                self.hwnd,
                std::ptr::null(),
                std::ptr::null_mut(),
                RDW_INVALIDATE | RDW_UPDATENOW,
            );
        }
        self.visible = true;
    }

    pub fn update_options(&mut self, opts: OverviewOptions) {
        with_overview(|st| {
            st.opts = OverviewOptions {
                stiffness: opts.stiffness.max(10.0),
                damping_ratio: opts.damping_ratio.clamp(0.25, 2.0),
                min_w_frac: opts.min_w_frac.clamp(0.05, 0.60),
                min_h_frac: opts.min_h_frac.clamp(0.05, 0.60),
                autoscroll_speed: opts.autoscroll_speed.clamp(0.0, 5000.0),
                ..opts
            };
        });
        if self.visible {
            unsafe {
                InvalidateRect(self.hwnd, std::ptr::null(), 0);
            }
        }
    }

    pub fn scroll_by(&mut self, dx: f64, dy: f64) {
        if !self.visible {
            return;
        }
        with_overview(|st| {
            apply_scroll_delta(st, dx, dy, true);
        });
        unsafe {
            InvalidateRect(self.hwnd, std::ptr::null(), 0);
        }
    }

    pub fn refresh(&mut self, ws: WorkspaceSnapshot, opts: OverviewOptions) {
        if !self.visible {
            return;
        }
        with_overview(|st| {
            st.cursor = (ws.camera_col, ws.camera_row);
            st.snapshot = Some(ws);
            st.drag = None;
            st.pending_empty_click = None;
            st.pending_close = None;
            st.hover_close_win = None;
            st.opts = OverviewOptions {
                stiffness: opts.stiffness.max(10.0),
                damping_ratio: opts.damping_ratio.clamp(0.25, 2.0),
                min_w_frac: opts.min_w_frac.clamp(0.05, 0.60),
                min_h_frac: opts.min_h_frac.clamp(0.05, 0.60),
                autoscroll_speed: opts.autoscroll_speed.clamp(0.0, 5000.0),
                ..opts
            };
            st.last_tick = Instant::now();

            rebuild_boxes_and_animate(st);
            ensure_coord_visible(st, st.cursor, true);
            sync_icons(st);
            sync_thumbnails(st);
            update_animated_thumbnails(st);
            st.pacer.set_active(true);
        });
        unsafe {
            InvalidateRect(self.hwnd, std::ptr::null(), 0);
        }
    }

    pub fn begin_close(&mut self, jump: Option<(i32, i32, Option<u64>)>) {
        if !self.visible {
            return;
        }
        with_overview(|st| {
            start_closing(st, jump);
        });
    }

    pub fn hide(&mut self) {
        if self.visible {
            with_overview(|st| {
                st.drag = None;
                st.pending_empty_click = None;
                st.pending_close = None;
                st.hover_close_win = None;
                st.closing_jump = None;
                st.pacer.set_active(false);
                clear_thumbnails(st);
                st.surface = None;
                st.icon_surface = None;
                st.wallpaper.clear();
                st.icons.clear();
                st.card_springs.clear();
                st.hover_springs.clear();
                st.close_springs.clear();
            });
            unsafe {
                ReleaseCapture();
                ShowWindow(self.icon_hwnd, SW_HIDE);
                ShowWindow(self.hwnd, SW_HIDE);
            }
            self.visible = false;
        }
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }
}

impl Drop for OverviewWindow {
    fn drop(&mut self) {
        OVERVIEW_STATE.with(|cell| {
            if let Ok(mut guard) = cell.try_borrow_mut() {
                if let Some(st) = guard.as_mut() {
                    st.pacer.set_active(false);
                    clear_thumbnails(st);
                    st.surface = None;
                    st.icon_surface = None;
                }
                *guard = None;
            }
        });
        if !self.icon_hwnd.is_null() {
            unsafe {
                DestroyWindow(self.icon_hwnd);
            }
        }
        if !self.hwnd.is_null() {
            unsafe {
                DestroyWindow(self.hwnd);
            }
        }
    }
}

unsafe extern "system" fn icons_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn sync_icons(st: &mut OverviewState) {
    for b in &st.boxes {
        if let Some(win_id) = b.window_id {
            if !st.icons.contains_key(&win_id) {
                let src_hwnd = win_id as usize as HWND;
                if let Some(icon) = extract_window_icon(src_hwnd) {
                    st.icons.insert(win_id, icon);
                }
            }
        }
    }
}

fn scale_around_center(r: IRect, scale: f64) -> IRect {
    if (scale - 1.0).abs() <= 0.001 {
        return r;
    }
    let nw = ((r.w as f64) * scale).round() as i32;
    let nh = ((r.h as f64) * scale).round() as i32;
    let nx = r.x + (r.w - nw) / 2;
    let ny = r.y + (r.h - nh) / 2;
    IRect::new(nx, ny, nw.max(16), nh.max(16))
}

fn highlighted_coord(st: &OverviewState) -> Option<(i32, i32)> {
    if st.closing_jump.is_some() || st.drag.as_ref().is_some_and(|d| d.active) {
        None
    } else {
        st.hover_coord.or(Some(st.cursor))
    }
}

fn sync_hover_targets(st: &mut OverviewState) {
    let highlight = highlighted_coord(st);
    let is_hover = st.hover_coord.is_some();
    for b in &st.boxes {
        let coord = (b.col, b.row);
        let is_hl = highlight == Some(coord);
        let target = if is_hl {
            if is_hover { 1.038 } else { 1.022 }
        } else {
            1.0
        };
        let sp = st.hover_springs.entry(coord).or_insert_with(|| {
            let mut s = Spring1D::default();
            s.snap_to(1.0);
            s
        });
        sp.set_target(target, st.closing_jump.is_none());

        let close_target = if b.window_id.is_some() && is_hl {
            1.0
        } else {
            0.0
        };
        let csp = st.close_springs.entry(coord).or_insert_with(|| {
            let mut s = Spring1D::default();
            s.snap_to(0.0);
            s
        });
        csp.set_target(close_target, true);
    }
}

fn resolve_highlight_win_id(st: &OverviewState) -> Option<u64> {
    st.boxes
        .iter()
        .find(|b| (b.col, b.row) == st.cursor && b.window_id.is_some())
        .or_else(|| st.boxes.iter().find(|b| b.focused && b.window_id.is_some()))
        .and_then(|b| b.window_id)
        .or(st.hero_window_id)
}

fn start_closing(st: &mut OverviewState, jump: Option<(i32, i32, Option<u64>)>) {
    if st.closing_jump.is_some() {
        return;
    }
    let hero_win = jump
        .and_then(|(_, _, w)| w)
        .or_else(|| resolve_highlight_win_id(st));
    if let Some(wid) = hero_win {
        promote_thumbnail_to_front(st, wid);
        if let Some(cs) = st.card_springs.get_mut(&wid) {
            cs.start_rect = Some(st.return_rect);
        }
    }
    st.closing_jump = Some(jump);
    st.empty_fade_spring.set_target(0.0, true);
    sync_hover_targets(st);
    st.last_tick = Instant::now();
    st.open_spring.set_target(0.0, true);
    st.pacer.set_active(true);
}

unsafe fn force_foreground(hwnd: HWND) {
    unsafe {
        let fg = GetForegroundWindow();
        if fg == hwnd {
            return;
        }
        let fg_tid = if !fg.is_null() {
            GetWindowThreadProcessId(fg, std::ptr::null_mut())
        } else {
            0
        };
        let cur_tid = GetCurrentThreadId();
        if fg_tid != 0 && fg_tid != cur_tid {
            AttachThreadInput(fg_tid, cur_tid, TRUE);
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
            AttachThreadInput(fg_tid, cur_tid, 0);
        } else {
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
        }
    }
}

fn clear_thumbnails(st: &mut OverviewState) {
    for (_, thumb) in st.thumbs.drain() {
        unsafe {
            DwmUnregisterThumbnail(thumb);
        }
    }
    st.thumb_sizes.clear();
    st.last_thumb_props.clear();
}

fn register_one_thumbnail(st: &mut OverviewState, win_id: u64) -> Option<isize> {
    let dest_hwnd = st.overlay_hwnd;
    if dest_hwnd.is_null() {
        return None;
    }
    let src_hwnd = win_id as usize as HWND;
    if unsafe { IsWindow(src_hwnd) == 0 } {
        return None;
    }
    // Do not register thumbnails for truly minimized (iconic) windows:
    // iconic windows show an ugly 32x32 stretched icon in DWM, so they receive the prominent app icon instead.
    if unsafe { IsIconic(src_hwnd) != 0 } {
        return None;
    }
    let mut thumb: isize = 0;
    let hr = unsafe { DwmRegisterThumbnail(dest_hwnd, src_hwnd, &mut thumb) };
    if hr != 0 || thumb == 0 {
        return None;
    }
    let mut src_size: SIZE = unsafe { zeroed() };
    if unsafe { DwmQueryThumbnailSourceSize(thumb, &mut src_size) } == 0 {
        if src_size.cx >= 120 && src_size.cy >= 80 {
            st.thumb_sizes.insert(win_id, src_size);
        } else if src_size.cx > 0 && src_size.cy > 0 && (src_size.cx < 120 || src_size.cy < 80) {
            // Truly tiny placeholder or helper window - unregister to avoid stretching
            unsafe {
                DwmUnregisterThumbnail(thumb);
            }
            return None;
        }
        // If src_size is (0, 0), DWM is still asynchronously generating the thumbnail source:
        // keep the thumbnail registered and query src_size on subsequent frames in update_animated_thumbnails.
    }
    st.thumbs.insert(win_id, thumb);
    Some(thumb)
}

fn sync_thumbnails(st: &mut OverviewState) {
    clear_thumbnails(st);
    if st.overlay_hwnd.is_null() {
        return;
    }
    let ids: Vec<u64> = st.boxes.iter().filter_map(|b| b.window_id).collect();
    // Register all background windows first, and register hero window LAST so DWM paints it on top!
    for &win_id in &ids {
        if Some(win_id) != st.hero_window_id {
            register_one_thumbnail(st, win_id);
        }
    }
    if let Some(hero_id) = st.hero_window_id {
        if ids.contains(&hero_id) {
            register_one_thumbnail(st, hero_id);
        }
    }
}

fn promote_thumbnail_to_front(st: &mut OverviewState, win_id: u64) {
    if st.hero_window_id == Some(win_id) && st.thumbs.contains_key(&win_id) {
        return;
    }
    st.hero_window_id = Some(win_id);
    if let Some(old_thumb) = st.thumbs.remove(&win_id) {
        unsafe {
            DwmUnregisterThumbnail(old_thumb);
        }
    }
    if let Some(new_thumb) = register_one_thumbnail(st, win_id) {
        if let Some(&(dest, _)) = st.last_thumb_props.get(&win_id) {
            apply_thumb_props(new_thumb, dest, 255);
            st.last_thumb_props.insert(win_id, (dest, 255));
        }
    }
}

fn lerp_rect(a: IRect, b: IRect, t: f64) -> IRect {
    let f = |p: i32, q: i32| (p as f64 + (q as f64 - p as f64) * t).round() as i32;
    IRect::new(
        f(a.x, b.x),
        f(a.y, b.y),
        f(a.w, b.w).max(16),
        f(a.h, b.h).max(16),
    )
}

fn animated_card_rect(st: &OverviewState, b: &CellBox) -> IRect {
    let is_closing = st.closing_jump.is_some();
    let t = if is_closing {
        st.open_spring.current.clamp(0.0, 1.0)
    } else {
        st.open_spring.current.clamp(-0.15, 1.25)
    };
    let sx = st.scroll_x.current.round() as i32;
    let sy = st.scroll_y.current.round() as i32;

    if is_closing {
        if let Some(win_id) = b.window_id {
            if st.hero_window_id == Some(win_id) {
                let spring_rect = st
                    .card_springs
                    .get(&win_id)
                    .map(CardSpring::current_rect)
                    .unwrap_or(b.rect);
                let screen_card = IRect::new(
                    spring_rect.x - sx,
                    spring_rect.y - sy,
                    spring_rect.w,
                    spring_rect.h,
                );
                return lerp_rect(st.return_rect, screen_card, t);
            }
        }
    }

    let hover_scale = st
        .hover_springs
        .get(&(b.col, b.row))
        .map(|s| s.current.clamp(0.95, 1.08))
        .unwrap_or(1.0);
    let virt_base = if let Some(win_id) = b.window_id {
        let spring_rect = st
            .card_springs
            .get(&win_id)
            .map(CardSpring::current_rect)
            .unwrap_or(b.rect);
        let start_rect = st
            .card_springs
            .get(&win_id)
            .and_then(|c| c.start_rect)
            .unwrap_or(spring_rect);
        lerp_rect(start_rect, spring_rect, t)
    } else {
        let cx = st.bounds.w / 2 + sx;
        let cy = st.bounds.h / 2 + sy;
        let origin = IRect::new(
            cx + ((b.rect.x - cx) as f64 * 1.14) as i32,
            cy + ((b.rect.y - cy) as f64 * 1.14) as i32,
            b.rect.w,
            b.rect.h,
        );
        lerp_rect(origin, b.rect, t)
    };
    let screen_rect = IRect::new(virt_base.x - sx, virt_base.y - sy, virt_base.w, virt_base.h);
    if is_closing {
        // Push background windows gently backward as the hero window expands to the front.
        scale_around_center(screen_rect, 0.84 + 0.16 * t)
    } else {
        scale_around_center(screen_rect, hover_scale)
    }
}

fn update_animated_thumbnails(st: &mut OverviewState) {
    let t = st.open_spring.current.clamp(0.0, 1.0);
    let active_drag = st.drag.as_ref().filter(|d| d.active).cloned();
    let is_closing = st.closing_jump.is_some();

    let missing_ids: Vec<u64> = st
        .boxes
        .iter()
        .filter_map(|b| b.window_id)
        .filter(|id| !st.thumbs.contains_key(id))
        .collect();
    for id in missing_ids {
        register_one_thumbnail(st, id);
    }

    let unqueried: Vec<(u64, isize)> = st
        .thumbs
        .iter()
        .filter(|(wid, _)| !st.thumb_sizes.contains_key(wid))
        .map(|(&wid, &t)| (wid, t))
        .collect();
    for (win_id, thumb) in unqueried {
        let mut src_size: SIZE = unsafe { zeroed() };
        if unsafe { DwmQueryThumbnailSourceSize(thumb, &mut src_size) } == 0 {
            if src_size.cx >= 120 && src_size.cy >= 80 {
                st.thumb_sizes.insert(win_id, src_size);
            } else if src_size.cx > 0 && src_size.cy > 0 && (src_size.cx < 120 || src_size.cy < 80) {
                if let Some(t) = st.thumbs.remove(&win_id) {
                    unsafe {
                        DwmUnregisterThumbnail(t);
                    }
                }
            }
        }
    }

    for b in &st.boxes {
        let Some(win_id) = b.window_id else {
            continue;
        };
        let Some(&thumb) = st.thumbs.get(&win_id) else {
            continue;
        };
        if active_drag.as_ref().is_some_and(|d| d.window_id == win_id) {
            continue;
        }
        let card = animated_card_rect(st, b);
        let is_hero = Some(win_id) == st.hero_window_id;
        let alpha = if is_closing {
            if is_hero {
                255
            } else {
                (255.0 * t).round() as u8
            }
        } else if is_hero {
            255
        } else {
            (255.0 * t.clamp(0.05, 1.0)).round() as u8
        };
        let src_size = st.thumb_sizes.get(&win_id).copied();
        let dest = fit_thumb_rect(card, src_size);
        let unchanged = st.last_thumb_props.get(&win_id).is_some_and(|&(r, a)| {
            a == alpha
                && r.left == dest.left
                && r.top == dest.top
                && r.right == dest.right
                && r.bottom == dest.bottom
        });
        if !unchanged {
            apply_thumb_props(thumb, dest, alpha);
            st.last_thumb_props.insert(win_id, (dest, alpha));
        }
    }

    if let Some(ref drag) = active_drag {
        update_drag_thumbnail_pos(st, drag);
    }
}

fn fit_thumb_rect(area: IRect, src_size: Option<SIZE>) -> RECT {
    if let Some(sz) = src_size {
        if sz.cx > 0 && sz.cy > 0 {
            let scale_w = area.w as f64 / sz.cx as f64;
            let scale_h = area.h as f64 / sz.cy as f64;
            let scale = scale_w.min(scale_h);
            let dw = ((sz.cx as f64) * scale).round() as i32;
            let dh = ((sz.cy as f64) * scale).round() as i32;
            let dx = area.x + (area.w - dw) / 2;
            let dy = area.y + (area.h - dh) / 2;
            return RECT {
                left: dx,
                top: dy,
                right: dx + dw.max(1),
                bottom: dy + dh.max(1),
            };
        }
    }
    RECT {
        left: area.x,
        top: area.y,
        right: area.right(),
        bottom: area.bottom(),
    }
}

fn apply_thumb_props(thumb: isize, dest: RECT, opacity: u8) {
    let props = DWM_THUMBNAIL_PROPERTIES {
        dwFlags: DWM_TNP_RECTDESTINATION
            | DWM_TNP_VISIBLE
            | DWM_TNP_OPACITY
            | DWM_TNP_SOURCECLIENTAREAONLY,
        rcDestination: dest,
        rcSource: unsafe { zeroed() },
        opacity,
        fVisible: TRUE,
        fSourceClientAreaOnly: 0,
    };
    unsafe {
        DwmUpdateThumbnailProperties(thumb, &props);
    }
}

fn drag_ghost_rect(st: &OverviewState, drag: &DragState) -> IRect {
    let gw = (st.bounds.w / 6).clamp(200, 280);
    let gh = ((gw as f64 * st.bounds.h.max(1) as f64) / st.bounds.w.max(1) as f64)
        .round()
        .clamp(120.0, 190.0) as i32;
    let gx = (drag.cur_x - gw / 2).clamp(4, (st.bounds.w - gw - 4).max(4));
    let gy = (drag.cur_y - gh / 2).clamp(4, (st.bounds.h - gh - 4).max(4));
    IRect::new(gx, gy, gw, gh)
}

fn update_drag_thumbnail_pos(st: &mut OverviewState, drag: &DragState) {
    if let Some(&thumb) = st.thumbs.get(&drag.window_id) {
        let ghost = drag_ghost_rect(st, drag);
        let dest = fit_thumb_rect(ghost, st.thumb_sizes.get(&drag.window_id).copied());
        apply_thumb_props(thumb, dest, 250);
        st.last_thumb_props.insert(drag.window_id, (dest, 250));
    }
}

fn rebuild_boxes_and_animate(st: &mut OverviewState) {
    rebuild_boxes(st);
    for b in &st.boxes {
        st.hover_springs.entry((b.col, b.row)).or_insert_with(|| {
            let mut hs = Spring1D::default();
            hs.snap_to(1.0);
            hs
        });
        if let Some(win_id) = b.window_id {
            let entry = st.card_springs.entry(win_id).or_insert_with(|| {
                let mut cs = CardSpring::default();
                cs.set_target(b.rect, false);
                cs
            });
            entry.set_target(b.rect, true);
        }
    }
    sync_hover_targets(st);
}

/// Computes dynamic `(cell_w, cell_h, gap, start_x, start_y)` so the entire `cols x rows`
/// overview grid fits completely inside `(bounds_w x bounds_h)` with zero scrolling.
pub fn compute_overview_grid_layout(
    bounds_w: i32,
    bounds_h: i32,
    cols: i32,
    rows: i32,
    drag_active: bool,
) -> (i32, i32, i32, i32, i32) {
    let cols = cols.max(1);
    let rows = rows.max(1);
    let pad_x = if drag_active {
        (bounds_w / 34).clamp(24, 48)
    } else {
        (bounds_w / 30).clamp(28, 56)
    };
    let pad_y = if drag_active {
        (bounds_h / 24).clamp(20, 40)
    } else {
        (bounds_h / 22).clamp(24, 48)
    };
    let avail_w = (bounds_w - pad_x * 2).max(120);
    let avail_h = (bounds_h - pad_y * 2).max(120);
    let mon_aspect = (bounds_w.max(1) as f64) / (bounds_h.max(1) as f64);

    let base_gap = if drag_active {
        14
    } else {
        (bounds_w / 68).clamp(16, 26)
    };
    let max_gap_w = if cols > 1 {
        avail_w / (cols * 6).max(1)
    } else {
        base_gap
    };
    let max_gap_h = if rows > 1 {
        avail_h / (rows * 6).max(1)
    } else {
        base_gap
    };
    let gap = base_gap.min(max_gap_w).min(max_gap_h).max(6);

    let slot_w = ((avail_w - gap * (cols - 1)).max(cols) / cols).max(32);
    let slot_h = ((avail_h - gap * (rows - 1)).max(rows) / rows).max(24);
    let (max_w_per_cell, max_h_per_cell) = if drag_active {
        (slot_w.min(340), slot_h.min(220))
    } else {
        (
            slot_w.min((avail_w * 74) / 100),
            slot_h.min((avail_h * 74) / 100),
        )
    };

    let (cell_w, cell_h) = if (max_w_per_cell as f64 / mon_aspect) <= max_h_per_cell as f64 {
        (
            max_w_per_cell.max(24),
            (max_w_per_cell as f64 / mon_aspect)
                .floor()
                .clamp(20.0, max_h_per_cell as f64) as i32,
        )
    } else {
        (
            (max_h_per_cell as f64 * mon_aspect)
                .floor()
                .clamp(24.0, max_w_per_cell as f64) as i32,
            max_h_per_cell.max(20),
        )
    };

    let total_w = cell_w * cols + gap * (cols - 1);
    let total_h = cell_h * rows + gap * (rows - 1);
    let start_x = ((bounds_w - total_w) / 2).max(pad_x);
    let start_y = (pad_y + (avail_h - total_h) / 2).max(pad_y);
    (cell_w, cell_h, gap, start_x, start_y)
}

fn rebuild_boxes(st: &mut OverviewState) {
    st.boxes.clear();
    let Some(ref ws) = st.snapshot else {
        return;
    };
    let tiled: Vec<_> = ws.windows.iter().filter(|w| !w.floating).collect();
    let floating: Vec<_> = ws.windows.iter().filter(|w| w.floating).collect();
    let drag_active = st.drag.as_ref().is_some_and(|d| d.active);

    let (mut min_col, mut max_col, mut min_row, mut max_row) = if let Some(first) = tiled.first() {
        let mut c0 = first.col;
        let mut c1 = first.col;
        let mut r0 = first.row;
        let mut r1 = first.row;
        for w in &tiled {
            c0 = c0.min(w.col);
            c1 = c1.max(w.col);
            r0 = r0.min(w.row);
            r1 = r1.max(w.row);
        }
        (c0, c1, r0, r1)
    } else {
        (ws.camera_col, ws.camera_col, ws.camera_row, ws.camera_row)
    };

    let float_row = if tiled.is_empty() {
        min_row
    } else {
        max_row + 1
    };
    if !floating.is_empty() {
        max_col = max_col.max(min_col + floating.len() as i32 - 1);
        max_row = float_row;
    }

    let float_start_col = min_col;
    if drag_active {
        min_col -= 1;
        max_col += 1;
        min_row -= 1;
        max_row += 1;
    }

    let cols = (max_col - min_col + 1).max(1);
    let rows = (max_row - min_row + 1).max(1);
    let (cell_w, cell_h, gap, start_x, start_y) =
        compute_overview_grid_layout(st.bounds.w, st.bounds.h, cols, rows, drag_active);

    st.max_scroll = (0.0, 0.0);
    st.content_size = (st.bounds.w, st.bounds.h);

    for row in min_row..=max_row {
        for col in min_col..=max_col {
            let gx = start_x + (col - min_col) * (cell_w + gap);
            let gy = start_y + (row - min_row) * (cell_h + gap);
            let mut cell_wins: Vec<_> = tiled
                .iter()
                .filter(|w| w.col == col && w.row == row)
                .copied()
                .collect();
            cell_wins.sort_by_key(|w| w.stack_index);

            if cell_wins.is_empty() {
                let is_float = !floating.is_empty() && row == float_row && col >= float_start_col;
                let float_win = if is_float {
                    floating.get((col - float_start_col) as usize).copied()
                } else {
                    None
                };
                if !drag_active && float_win.is_none() {
                    continue;
                }
                st.boxes.push(CellBox {
                    rect: IRect::new(gx, gy, cell_w, cell_h),
                    col,
                    row,
                    window_id: float_win.map(|w| w.id),
                    stack_len: float_win.map_or(0, |_| 1),
                    focused: float_win.is_some_and(|w| w.focused),
                });
            } else if cell_wins.len() == 1 {
                let w = cell_wins[0];
                st.boxes.push(CellBox {
                    rect: IRect::new(gx, gy, cell_w, cell_h),
                    col,
                    row,
                    window_id: Some(w.id),
                    stack_len: 1,
                    focused: w.focused,
                });
            } else {
                // Multi-window horizontally snapped view: subdivide width across windows so all thumbnails are visible side-by-side
                let n = cell_wins.len();
                let sub_gap = 4;
                let avail = cell_w - sub_gap * (n as i32 - 1);
                let sub_w = (avail / n as i32).max(24);
                for (idx, w) in cell_wins.iter().enumerate() {
                    let sx = gx + idx as i32 * (sub_w + sub_gap);
                    st.boxes.push(CellBox {
                        rect: IRect::new(sx, gy, sub_w, cell_h),
                        col,
                        row,
                        window_id: Some(w.id),
                        stack_len: n,
                        focused: w.focused,
                    });
                }
            }
        }
    }

    st.scroll_x.snap_to(0.0);
    st.scroll_y.snap_to(0.0);
}

fn apply_scroll_delta(_st: &mut OverviewState, _dx: f64, _dy: f64, _animate: bool) {}

fn ensure_coord_visible(_st: &mut OverviewState, _coord: (i32, i32), _animate: bool) {}

pub fn close_button_rect(dest_r: IRect, view_w: i32, view_h: i32) -> Option<IRect> {
    if dest_r.w < 56 || dest_r.h < 48 {
        return None;
    }
    let sz = (dest_r.w.min(dest_r.h) / 6).clamp(22, 28);
    // Position cleanly in the right top corner of the card view.
    let bx = (dest_r.right() - (sz * 2) / 3).clamp(2, (view_w - sz - 2).max(2));
    let by = (dest_r.y - sz / 3).clamp(2, (view_h - sz - 2).max(2));
    Some(IRect::new(bx, by, sz, sz))
}

fn card_dest_rect(st: &OverviewState, b: &CellBox) -> Option<IRect> {
    let win_id = b.window_id?;
    let card = animated_card_rect(st, b);
    let dest = fit_thumb_rect(card, st.thumb_sizes.get(&win_id).copied());
    Some(IRect::new(
        dest.left,
        dest.top,
        (dest.right - dest.left).max(16),
        (dest.bottom - dest.top).max(16),
    ))
}

fn point_in_rect_pad(mx: i32, my: i32, r: IRect, pad: i32) -> bool {
    mx >= r.x - pad && mx <= r.right() + pad && my >= r.y - pad && my <= r.bottom() + pad
}

fn hit_close_button(st: &OverviewState, mx: i32, my: i32) -> Option<(i32, i32, u64)> {
    let hl = highlighted_coord(st)?;
    let b = st
        .boxes
        .iter()
        .find(|b| (b.col, b.row) == hl && b.window_id.is_some())?;
    let win_id = b.window_id?;
    let anim_t = st.close_springs.get(&hl).map(|s| s.current).unwrap_or(0.0);
    if anim_t < 0.25 {
        return None;
    }
    let dest_r = card_dest_rect(st, b)?;
    let btn = close_button_rect(dest_r, st.bounds.w, st.bounds.h)?;
    if point_in_rect_pad(mx, my, btn, 4) {
        Some((b.col, b.row, win_id))
    } else {
        None
    }
}

fn draw_line_aa(
    buf: &mut [u32],
    buf_w: i32,
    buf_h: i32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    half_width: f32,
    rgba: (u8, u8, u8, u8),
) {
    let min_x = (x0.min(x1) - half_width - 1.5).floor().max(0.0) as i32;
    let max_x = (x0.max(x1) + half_width + 1.5)
        .ceil()
        .min((buf_w - 1) as f32) as i32;
    let min_y = (y0.min(y1) - half_width - 1.5).floor().max(0.0) as i32;
    let max_y = (y0.max(y1) + half_width + 1.5)
        .ceil()
        .min((buf_h - 1) as f32) as i32;
    let vx = x1 - x0;
    let vy = y1 - y0;
    let len_sq = (vx * vx + vy * vy).max(0.0001);

    for py in min_y..=max_y {
        let row_off = (py * buf_w) as usize;
        let fy = py as f32 + 0.5;
        for px in min_x..=max_x {
            let fx = px as f32 + 0.5;
            let t = (((fx - x0) * vx + (fy - y0) * vy) / len_sq).clamp(0.0, 1.0);
            let dx = fx - (x0 + t * vx);
            let dy = fy - (y0 + t * vy);
            let dist = (dx * dx + dy * dy).sqrt() - half_width;
            if dist <= 0.8 {
                let cov = (0.8 - dist).clamp(0.0, 1.0);
                let a = ((rgba.3 as f32) * cov).round() as u8;
                if a > 0 {
                    buf[row_off + px as usize] =
                        blend_premul(buf[row_off + px as usize], a, rgba.0, rgba.1, rgba.2);
                }
            }
        }
    }
}

fn draw_close_badge(
    buf: &mut [u32],
    buf_w: i32,
    buf_h: i32,
    rect: IRect,
    hovered: bool,
    alpha: u8,
) {
    if alpha < 8 || rect.w < 6 || rect.h < 6 {
        return;
    }
    draw_soft_shadow_circle(
        buf,
        buf_w,
        buf_h,
        rect.x + rect.w / 2,
        rect.y + rect.h / 2 + 1,
        (rect.w * 64) / 100,
        ((alpha as u16 * 150) / 255) as u8,
    );
    let fill = if hovered {
        (244, 66, 84, alpha)
    } else {
        (220, 44, 62, ((alpha as u16 * 242) / 255) as u8)
    };
    let border = if hovered {
        (255, 255, 255, alpha)
    } else {
        (255, 215, 220, ((alpha as u16 * 210) / 255) as u8)
    };
    draw_rounded_card(buf, buf_w, buf_h, rect, rect.w / 2, fill, border, 1, false);
    let pad = ((rect.w as f32) * 0.30).max(3.5);
    let x0 = rect.x as f32 + pad;
    let y0 = rect.y as f32 + pad;
    let x1 = rect.right() as f32 - pad;
    let y1 = rect.bottom() as f32 - pad;
    let glyph = (255, 255, 255, alpha);
    draw_line_aa(buf, buf_w, buf_h, x0, y0, x1, y1, 1.0, glyph);
    draw_line_aa(buf, buf_w, buf_h, x0, y1, x1, y0, 1.0, glyph);
}

fn hit_box(boxes: &[CellBox], sx: i32, sy: i32, mx: i32, my: i32) -> Option<&CellBox> {
    let vx = mx + sx;
    let vy = my + sy;
    boxes
        .iter()
        .find(|b| vx >= b.rect.x && vx <= b.rect.right() && vy >= b.rect.y && vy <= b.rect.bottom())
}

fn hit_or_nearest_box(boxes: &[CellBox], sx: i32, sy: i32, mx: i32, my: i32) -> Option<&CellBox> {
    if let Some(exact) = hit_box(boxes, sx, sy, mx, my) {
        return Some(exact);
    }
    let vx = mx + sx;
    let vy = my + sy;
    boxes.iter().min_by_key(|b| {
        let (cx, cy) = b.rect.center();
        let dx = cx - vx;
        let dy = cy - vy;
        dx * dx + dy * dy
    })
}

fn is_shift_down() -> bool {
    unsafe { (GetKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0 }
}

fn is_ctrl_down() -> bool {
    unsafe { (GetKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0 }
}

unsafe extern "system" fn overview_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => 1,
        WM_APP_VSYNC_STEP => {
            let finish_cmd = with_overview(|st| {
                let now = Instant::now();
                let dt = (now - st.last_tick).as_secs_f64();
                st.last_tick = now;

                let stiff = st.opts.stiffness;
                let damp = st.opts.damping_ratio;
                let open_damp = if st.closing_jump.is_some() {
                    damp.max(0.94)
                } else {
                    damp
                };
                let open_anim = st.open_spring.step(dt, stiff, open_damp, 0.0015);
                let empty_anim = st
                    .empty_fade_spring
                    .step(dt, stiff * 1.15, damp.max(0.86), 0.002);

                st.scroll_x.snap_to(0.0);
                st.scroll_y.snap_to(0.0);
                let mut cards_anim = false;
                for cs in st.card_springs.values_mut() {
                    cards_anim |= cs.step(dt, stiff, damp);
                }
                let hover_stiff = (stiff * 1.45).max(360.0);
                let hover_damp = (damp * 0.88).clamp(0.52, 0.90);
                for hs in st.hover_springs.values_mut() {
                    cards_anim |= hs.step(dt, hover_stiff, hover_damp, 0.0008);
                }
                let close_stiff = (stiff * 1.65).max(420.0);
                let close_damp = (damp * 0.85).clamp(0.55, 0.90);
                for csp in st.close_springs.values_mut() {
                    cards_anim |= csp.step(dt, close_stiff, close_damp, 0.004);
                }

                update_animated_thumbnails(st);

                if st.closing_jump.is_some() && !open_anim && st.open_spring.current <= 0.015 {
                    let jump = st.closing_jump.take().flatten();
                    st.pacer.set_active(false);
                    return Some((st.controller_hwnd, OverviewCommand::FinishClose { jump }));
                }

                if !open_anim && !empty_anim && !cards_anim {
                    st.pacer.set_active(false);
                }
                None
            })
            .flatten();

            unsafe {
                RedrawWindow(
                    hwnd,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    RDW_INVALIDATE | RDW_UPDATENOW,
                );
            }
            if let Some((ctrl, cmd)) = finish_cmd {
                emit_command(ctrl, cmd);
            }
            0
        }
        WM_PAINT => {
            unsafe { paint_overview(hwnd) };
            0
        }
        WM_MOUSEWHEEL | WM_MOUSEHWHEEL => 0,
        WM_LBUTTONDOWN => {
            let mx = (lparam & 0xFFFF) as i16 as i32;
            let my = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            let need_capture = with_overview(|st| {
                if st.closing_jump.is_some() {
                    return false;
                }
                if let Some((col, row, win_id)) = hit_close_button(st, mx, my) {
                    st.drag = None;
                    st.pending_empty_click = None;
                    st.pending_close = Some((col, row, win_id));
                    st.hover_coord = Some((col, row));
                    st.hover_close_win = Some(win_id);
                    return true;
                }
                st.pending_close = None;
                let sx = st.scroll_x.current.round() as i32;
                let sy = st.scroll_y.current.round() as i32;
                if let Some(b) = hit_box(&st.boxes, sx, sy, mx, my).cloned() {
                    st.hover_coord = Some((b.col, b.row));
                    if let Some(win_id) = b.window_id {
                        st.drag = Some(DragState {
                            window_id: win_id,
                            source_col: b.col,
                            source_row: b.row,
                            start_x: mx,
                            start_y: my,
                            cur_x: mx,
                            cur_y: my,
                            active: false,
                        });
                        st.pending_empty_click = None;
                        return true;
                    }
                    st.drag = None;
                    st.pending_empty_click = Some((b.col, b.row));
                } else {
                    st.drag = None;
                    st.pending_empty_click = None;
                }
                false
            })
            .unwrap_or(false);
            if need_capture {
                unsafe {
                    SetCapture(hwnd);
                }
            }
            0
        }
        WM_MOUSEMOVE => {
            let mx = (lparam & 0xFFFF) as i16 as i32;
            let my = ((lparam >> 16) & 0xFFFF) as i16 as i32;
            let mut active_drag_copy: Option<DragState> = None;
            let needs_redraw = with_overview(|st| {
                let sx = st.scroll_x.current.round() as i32;
                let sy = st.scroll_y.current.round() as i32;
                let mut changed = false;
                let mut just_activated_drag = false;
                if let Some(ref mut drag) = st.drag {
                    drag.cur_x = mx;
                    drag.cur_y = my;
                    if !drag.active
                        && ((mx - drag.start_x).abs() > 5 || (my - drag.start_y).abs() > 5)
                    {
                        drag.active = true;
                        just_activated_drag = true;
                    }
                    if drag.active {
                        active_drag_copy = Some(drag.clone());
                        changed = true;
                    }
                }

                if just_activated_drag {
                    if let Some(ref d) = active_drag_copy {
                        promote_thumbnail_to_front(st, d.window_id);
                    }
                    rebuild_boxes_and_animate(st);
                    let sx = st.scroll_x.current.round() as i32;
                    let sy = st.scroll_y.current.round() as i32;
                    st.hover_coord =
                        hit_or_nearest_box(&st.boxes, sx, sy, mx, my).map(|b| (b.col, b.row));
                    sync_hover_targets(st);
                    st.empty_fade_spring.set_target(1.0, true);
                    st.last_tick = Instant::now();
                    st.pacer.set_active(true);
                    update_animated_thumbnails(st);
                    changed = true;
                } else {
                    let is_dragging = active_drag_copy.is_some();
                    let close_hit = if is_dragging {
                        None
                    } else {
                        hit_close_button(st, mx, my)
                    };
                    let new_hover = if is_dragging {
                        hit_or_nearest_box(&st.boxes, sx, sy, mx, my).map(|b| (b.col, b.row))
                    } else {
                        close_hit
                            .map(|(col, row, _)| (col, row))
                            .or_else(|| hit_box(&st.boxes, sx, sy, mx, my).map(|b| (b.col, b.row)))
                    };
                    let new_hover_close = close_hit.map(|(_, _, wid)| wid);
                    if st.hover_close_win != new_hover_close {
                        st.hover_close_win = new_hover_close;
                        changed = true;
                    }
                    if st.hover_coord != new_hover {
                        st.hover_coord = new_hover;
                        sync_hover_targets(st);
                        st.last_tick = Instant::now();
                        st.pacer.set_active(true);
                        changed = true;
                    }
                }

                if let Some(ref drag) = active_drag_copy {
                    update_drag_thumbnail_pos(st, drag);
                }
                changed
            })
            .unwrap_or(false);
            if needs_redraw {
                unsafe {
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
            }
            0
        }
        WM_LBUTTONUP => {
            let mx = (lparam & 0xFFFF) as i16 as i32;
            let my = ((lparam >> 16) & 0xFFFF) as i16 as i32;

            enum UpAction {
                Place(HWND, OverviewCommand),
                Close(Option<(i32, i32, Option<u64>)>),
                None,
            }

            let up = with_overview(|st| {
                let sx = st.scroll_x.current.round() as i32;
                let sy = st.scroll_y.current.round() as i32;
                let pending_close = st.pending_close.take();
                let drag = st.drag.take();
                let empty_click = st.pending_empty_click.take();

                if let Some((_, _, win_id)) = pending_close {
                    if hit_close_button(st, mx, my).is_some_and(|(_, _, w)| w == win_id) {
                        st.hover_close_win = None;
                        return UpAction::Place(
                            st.controller_hwnd,
                            OverviewCommand::CloseWindow { window_id: win_id },
                        );
                    }
                    return UpAction::None;
                }

                if let Some(drag) = drag {
                    if drag.active {
                        if let Some(target) = hit_or_nearest_box(&st.boxes, sx, sy, mx, my) {
                            if let Some(target_win_id) = target.window_id {
                                if target_win_id != drag.window_id {
                                    if is_ctrl_down() {
                                        return UpAction::Place(
                                            st.controller_hwnd,
                                            OverviewCommand::Place {
                                                window_id: drag.window_id,
                                                col: target.col,
                                                row: target.row,
                                                stack: true,
                                            },
                                        );
                                    }
                                    if is_shift_down() {
                                        return UpAction::Place(
                                            st.controller_hwnd,
                                            OverviewCommand::Place {
                                                window_id: drag.window_id,
                                                col: target.col,
                                                row: target.row,
                                                stack: false,
                                            },
                                        );
                                    }
                                    let on_left = (mx + sx) < target.rect.center().0;
                                    return UpAction::Place(
                                        st.controller_hwnd,
                                        OverviewCommand::SplitHorizontal {
                                            window_id: drag.window_id,
                                            target_window_id: target_win_id,
                                            on_left,
                                        },
                                    );
                                }
                            } else if (target.col, target.row) != (drag.source_col, drag.source_row)
                            {
                                return UpAction::Place(
                                    st.controller_hwnd,
                                    OverviewCommand::Place {
                                        window_id: drag.window_id,
                                        col: target.col,
                                        row: target.row,
                                        stack: false,
                                    },
                                );
                            }
                        }
                        st.empty_fade_spring.set_target(0.0, true);
                        rebuild_boxes_and_animate(st);
                        st.last_tick = Instant::now();
                        st.pacer.set_active(true);
                        update_animated_thumbnails(st);
                        return UpAction::None;
                    }
                    return UpAction::Close(Some((
                        drag.source_col,
                        drag.source_row,
                        Some(drag.window_id),
                    )));
                }

                if let Some((col, row)) = empty_click {
                    if hit_box(&st.boxes, sx, sy, mx, my)
                        .is_some_and(|b| b.col == col && b.row == row)
                    {
                        return UpAction::Close(Some((col, row, None)));
                    }
                }
                UpAction::Close(None)
            })
            .unwrap_or(UpAction::None);

            unsafe {
                ReleaseCapture();
            }

            match up {
                UpAction::Place(ctrl, cmd) => emit_command(ctrl, cmd),
                UpAction::Close(jump) => {
                    with_overview(|st| {
                        start_closing(st, jump);
                    });
                }
                UpAction::None => unsafe {
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                },
            }
            0
        }
        WM_RBUTTONDOWN => {
            with_overview(|st| {
                start_closing(st, None);
            });
            0
        }
        WM_CAPTURECHANGED => {
            with_overview(|st| {
                if st.drag.take().is_some() {
                    st.empty_fade_spring.set_target(0.0, true);
                    rebuild_boxes_and_animate(st);
                    st.last_tick = Instant::now();
                    st.pacer.set_active(true);
                    update_animated_thumbnails(st);
                }
            });
            unsafe {
                InvalidateRect(hwnd, std::ptr::null(), 0);
            }
            0
        }
        WM_KEYDOWN => {
            let vk = wparam as u16;
            if vk == VK_ESCAPE {
                with_overview(|st| {
                    start_closing(st, None);
                });
                return 0;
            }
            let moved = with_overview(|st| {
                match vk {
                    VK_LEFT => st.cursor.0 -= 1,
                    VK_RIGHT => st.cursor.0 += 1,
                    VK_UP => st.cursor.1 -= 1,
                    VK_DOWN => st.cursor.1 += 1,
                    VK_RETURN => {
                        let (col, row) = st.cursor;
                        let win_id = st
                            .boxes
                            .iter()
                            .find(|b| b.col == col && b.row == row)
                            .and_then(|b| b.window_id);
                        start_closing(st, Some((col, row, win_id)));
                        return false;
                    }
                    _ => return false,
                }
                st.hover_coord = None;
                ensure_coord_visible(st, st.cursor, true);
                sync_hover_targets(st);
                st.last_tick = Instant::now();
                st.pacer.set_active(true);
                true
            });
            if moved == Some(true) {
                unsafe {
                    InvalidateRect(hwnd, std::ptr::null(), 0);
                }
            }
            0
        }
        WM_KILLFOCUS => {
            with_overview(|st| {
                if st.drag.as_ref().is_none_or(|d| !d.active) {
                    start_closing(st, None);
                }
            });
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

#[inline]
pub(crate) fn premul(a: u8, r: u8, g: u8, b: u8) -> u32 {
    let af = a as u32;
    let pr = ((r as u32) * af + 127) / 255;
    let pg = ((g as u32) * af + 127) / 255;
    let pb = ((b as u32) * af + 127) / 255;
    (af << 24) | (pr << 16) | (pg << 8) | pb
}

#[inline]
pub(crate) fn blend_premul(dst: u32, src_a: u8, r: u8, g: u8, b: u8) -> u32 {
    if src_a == 0 {
        return dst;
    }
    if src_a == 255 {
        return premul(255, r, g, b);
    }
    let sa = src_a as u32;
    let inv = 255 - sa;
    let da = (dst >> 24) & 0xFF;
    let dr = (dst >> 16) & 0xFF;
    let dg = (dst >> 8) & 0xFF;
    let db = dst & 0xFF;

    let sr = (r as u32 * sa + 127) / 255;
    let sg = (g as u32 * sa + 127) / 255;
    let sb = (b as u32 * sa + 127) / 255;

    let oa = sa + (da * inv + 127) / 255;
    let or = (sr + (dr * inv + 127) / 255).min(255);
    let og = (sg + (dg * inv + 127) / 255).min(255);
    let ob = (sb + (db * inv + 127) / 255).min(255);
    (oa.min(255) << 24) | (or << 16) | (og << 8) | ob
}

/// Fast rounded rectangle renderer: flat spans use direct slice fills without per-pixel `sqrt()`,
/// computing SDF antialiasing only inside the 4 corner quadrants.
pub(crate) fn draw_rounded_card(
    buf: &mut [u32],
    buf_w: i32,
    buf_h: i32,
    rect: IRect,
    radius: i32,
    fill_rgba: (u8, u8, u8, u8),
    border_rgba: (u8, u8, u8, u8),
    border_w: i32,
    dashed: bool,
) {
    let x0 = rect.x.max(0);
    let y0 = rect.y.max(0);
    let x1 = rect.right().min(buf_w);
    let y1 = rect.bottom().min(buf_h);
    if x1 <= x0 || y1 <= y0 {
        return;
    }

    let rad = radius.min(rect.w / 2).min(rect.h / 2).max(2);
    let rf = rad as f32;
    let cx_left = rect.x + rad;
    let cx_right = rect.right() - rad;
    let cy_top = rect.y + rad;
    let cy_bot = rect.bottom() - rad;
    let bw = border_w.max(1);
    let bwf = bw as f32;

    let pre_fill = premul(fill_rgba.3, fill_rgba.0, fill_rgba.1, fill_rgba.2);
    let pre_border = premul(border_rgba.3, border_rgba.0, border_rgba.1, border_rgba.2);

    for py in y0..y1 {
        let row_off = (py * buf_w) as usize;
        let in_top_cap = py < cy_top;
        let in_bot_cap = py >= cy_bot;

        if !in_top_cap && !in_bot_cap {
            // Fast horizontal middle span: left border, optional interior fill, right border.
            let left_b_end = (rect.x + bw).clamp(x0, x1);
            let right_b_start = (rect.right() - bw).clamp(x0, x1);

            for px in x0..left_b_end {
                if !dashed || ((px + py) / 6) % 2 == 0 {
                    buf[row_off + px as usize] = blend_premul(
                        buf[row_off + px as usize],
                        border_rgba.3,
                        border_rgba.0,
                        border_rgba.1,
                        border_rgba.2,
                    );
                }
            }
            if fill_rgba.3 > 0 && right_b_start > left_b_end {
                if fill_rgba.3 == 255 {
                    buf[row_off + left_b_end as usize..row_off + right_b_start as usize]
                        .fill(pre_fill);
                } else {
                    for px in left_b_end..right_b_start {
                        buf[row_off + px as usize] = blend_premul(
                            buf[row_off + px as usize],
                            fill_rgba.3,
                            fill_rgba.0,
                            fill_rgba.1,
                            fill_rgba.2,
                        );
                    }
                }
            }
            for px in right_b_start.max(left_b_end)..x1 {
                if !dashed || ((px + py) / 6) % 2 == 0 {
                    buf[row_off + px as usize] = blend_premul(
                        buf[row_off + px as usize],
                        border_rgba.3,
                        border_rgba.0,
                        border_rgba.1,
                        border_rgba.2,
                    );
                }
            }
            continue;
        }

        // Top/bottom cap rows: fast center straight edge, corner SDF only in left/right corner boxes.
        let fy = py as f32 + 0.5;
        let dy = if in_top_cap {
            cy_top as f32 - fy
        } else {
            fy - cy_bot as f32
        };

        let mid_x0 = cx_left.max(x0).min(x1);
        let mid_x1 = cx_right.max(x0).min(x1);
        let is_outer_band = py < rect.y + bw || py >= rect.bottom() - bw;
        if mid_x1 > mid_x0 {
            if is_outer_band {
                if !dashed && border_rgba.3 == 255 {
                    buf[row_off + mid_x0 as usize..row_off + mid_x1 as usize].fill(pre_border);
                } else {
                    for px in mid_x0..mid_x1 {
                        if !dashed || ((px + py) / 6) % 2 == 0 {
                            buf[row_off + px as usize] = blend_premul(
                                buf[row_off + px as usize],
                                border_rgba.3,
                                border_rgba.0,
                                border_rgba.1,
                                border_rgba.2,
                            );
                        }
                    }
                }
            } else if fill_rgba.3 > 0 {
                for px in mid_x0..mid_x1 {
                    buf[row_off + px as usize] = blend_premul(
                        buf[row_off + px as usize],
                        fill_rgba.3,
                        fill_rgba.0,
                        fill_rgba.1,
                        fill_rgba.2,
                    );
                }
            }
        }

        // Left and right rounded corners.
        for px in (x0..mid_x0).chain(mid_x1..x1) {
            let fx = px as f32 + 0.5;
            let dx = if px < cx_left {
                cx_left as f32 - fx
            } else {
                fx - cx_right as f32
            };
            let dist = (dx * dx + dy * dy).sqrt() - rf;
            if dist > 0.7 {
                continue;
            }
            let edge_cov = (0.7 - dist).clamp(0.0, 1.0);
            let idx = row_off + px as usize;
            if dist >= -bwf {
                if dashed && ((px + py) / 6) % 2 == 1 {
                    continue;
                }
                let a = ((border_rgba.3 as f32) * edge_cov).round() as u8;
                buf[idx] = blend_premul(buf[idx], a, border_rgba.0, border_rgba.1, border_rgba.2);
            } else if fill_rgba.3 > 0 {
                let a = ((fill_rgba.3 as f32) * edge_cov).round() as u8;
                buf[idx] = blend_premul(buf[idx], a, fill_rgba.0, fill_rgba.1, fill_rgba.2);
            }
        }
    }
}

unsafe fn create_argb_dib(hdc: HDC, w: i32, h: i32) -> Option<(HBITMAP, *mut u32)> {
    let mut bmi: BITMAPINFO = unsafe { zeroed() };
    bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = w;
    bmi.bmiHeader.biHeight = -h;
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    let mut bits_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
    let bmp = unsafe {
        CreateDIBSection(
            hdc,
            &bmi,
            DIB_RGB_COLORS,
            &mut bits_ptr,
            std::ptr::null_mut(),
            0,
        )
    };
    if bmp.is_null() || bits_ptr.is_null() {
        return None;
    }
    Some((bmp, bits_ptr as *mut u32))
}

pub(crate) unsafe fn draw_text_argb(
    dst: &mut [u32],
    dst_w: i32,
    dst_h: i32,
    scratch_hdc: HDC,
    scratch_buf: &mut [u32],
    scratch_w: i32,
    scratch_h: i32,
    font: HFONT,
    rect: IRect,
    text: &str,
    rgb: (u8, u8, u8),
    alpha: u8,
    flags: u32,
) {
    if alpha == 0 {
        return;
    }
    let rw = rect.w.min(scratch_w).max(1);
    let rh = rect.h.min(scratch_h).max(1);
    for y in 0..rh {
        let start = (y * scratch_w) as usize;
        scratch_buf[start..start + rw as usize].fill(0);
    }

    unsafe {
        let old_font = SelectObject(scratch_hdc, font as _);
        SetBkMode(scratch_hdc, TRANSPARENT as i32);
        SetTextColor(scratch_hdc, 0x00FF_FFFF);
        let mut rc = RECT {
            left: 0,
            top: 0,
            right: rw,
            bottom: rh,
        };
        let mut wide = to_wide(text);
        DrawTextW(scratch_hdc, wide.as_mut_ptr(), -1, &mut rc, flags);
        SelectObject(scratch_hdc, old_font);
    }

    for ly in 0..rh {
        let dy = rect.y + ly;
        if dy < 0 || dy >= dst_h {
            continue;
        }
        let s_row = (ly * scratch_w) as usize;
        let d_row = (dy * dst_w) as usize;
        for lx in 0..rw {
            let dx = rect.x + lx;
            if dx < 0 || dx >= dst_w {
                continue;
            }
            let px = scratch_buf[s_row + lx as usize];
            let cov = ((px >> 16) & 0xFF).max((px >> 8) & 0xFF).max(px & 0xFF);
            if cov == 0 {
                continue;
            }
            let a = ((cov * alpha as u32) / 255) as u8;
            dst[d_row + dx as usize] =
                blend_premul(dst[d_row + dx as usize], a, rgb.0, rgb.1, rgb.2);
        }
    }
}

unsafe fn draw_shadowed_text_argb(
    dst: &mut [u32],
    dst_w: i32,
    dst_h: i32,
    scratch_hdc: HDC,
    scratch_buf: &mut [u32],
    scratch_w: i32,
    scratch_h: i32,
    font: HFONT,
    rect: IRect,
    text: &str,
    rgb: (u8, u8, u8),
    alpha: u8,
    flags: u32,
) {
    let shadow_alpha = ((alpha as u16 * 215) / 255) as u8;
    let shadow_a = IRect::new(rect.x, rect.y + 1, rect.w, rect.h);
    let shadow_b = IRect::new(rect.x + 1, rect.y + 1, rect.w, rect.h);
    unsafe {
        draw_text_argb(
            dst,
            dst_w,
            dst_h,
            scratch_hdc,
            scratch_buf,
            scratch_w,
            scratch_h,
            font,
            shadow_a,
            text,
            (8, 12, 18),
            shadow_alpha,
            flags,
        );
        draw_text_argb(
            dst,
            dst_w,
            dst_h,
            scratch_hdc,
            scratch_buf,
            scratch_w,
            scratch_h,
            font,
            shadow_b,
            text,
            (8, 12, 18),
            shadow_alpha,
            flags,
        );
        draw_text_argb(
            dst,
            dst_w,
            dst_h,
            scratch_hdc,
            scratch_buf,
            scratch_w,
            scratch_h,
            font,
            rect,
            text,
            rgb,
            alpha,
            flags,
        );
    }
}

unsafe fn paint_overview(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = unsafe { zeroed() };
    let hdc_screen = unsafe { BeginPaint(hwnd, &mut ps) };
    if hdc_screen.is_null() {
        return;
    }

    with_overview(|st| {
        let w = st.bounds.w.max(1);
        let h = st.bounds.h.max(1);
        let needs_alloc = st.surface.as_ref().is_none_or(|s| s.w != w || s.h != h);
        if needs_alloc {
            st.surface = unsafe { DibSection::acquire(hdc_screen, w, h) };
        }
        let Some(surf) = st.surface.take() else {
            return;
        };

        let active_drag = st.drag.as_ref().filter(|d| d.active).cloned();
        let shift_down = is_shift_down();
        let open_t = st.open_spring.current.clamp(0.0, 1.0);
        let empty_t = st.empty_fade_spring.current.clamp(0.0, 1.0);
        let ui_alpha = (255.0 * open_t).round() as u8;

        st.wallpaper.ensure(w, h);

        unsafe {
            let pixels = std::slice::from_raw_parts_mut(surf.bits, (w * h) as usize);
            st.wallpaper
                .blit_to(pixels, open_t, st.opts.blur_mode, st.opts.alpha);

            for b in &st.boxes {
                let is_empty = b.window_id.is_none();
                let hovered = st.hover_coord == Some((b.col, b.row));
                let is_drag_source = active_drag
                    .as_ref()
                    .is_some_and(|d| d.source_col == b.col && d.source_row == b.row);
                let is_drop_target = active_drag.is_some() && hovered && !is_drag_source;

                let card = animated_card_rect(st, b);

                if is_empty {
                    let vis = if active_drag.is_some() {
                        empty_t.max(0.25) * open_t
                    } else {
                        empty_t * open_t
                    };
                    if vis <= 0.02 {
                        continue;
                    }
                    let (fill_rgba, border_rgba, bw, dashed) = if is_drop_target {
                        (
                            (130, 225, 165, (72.0 * vis).round() as u8),
                            (158, 226, 106, (240.0 * vis).round() as u8),
                            2,
                            false,
                        )
                    } else {
                        (
                            (255, 255, 255, (14.0 * vis).round() as u8),
                            (220, 232, 248, (145.0 * vis).round() as u8),
                            1,
                            true,
                        )
                    };
                    draw_rounded_card(pixels, w, h, card, 10, fill_rgba, border_rgba, bw, dashed);
                    continue;
                }

                // Solid card backdrop for every managed window
                let card_fill_a = (190.0 * open_t).round() as u8;
                let card_border_a = (140.0 * open_t).round() as u8;
                draw_rounded_card(
                    pixels,
                    w,
                    h,
                    card,
                    10,
                    (26, 30, 42, card_fill_a),
                    (65, 75, 98, card_border_a),
                    1,
                    false,
                );

                if is_drag_source && active_drag.is_some() {
                    draw_rounded_card(
                        pixels,
                        w,
                        h,
                        card,
                        10,
                        (255, 255, 255, 14),
                        (210, 224, 244, ui_alpha.min(130).max(1)),
                        1,
                        true,
                    );
                } else if is_drop_target {
                    let drop_rgb = if shift_down {
                        (122, 162, 247)
                    } else {
                        (247, 184, 104)
                    };
                    draw_rounded_card(
                        pixels,
                        w,
                        h,
                        card,
                        10,
                        (drop_rgb.0, drop_rgb.1, drop_rgb.2, 42),
                        (drop_rgb.0, drop_rgb.1, drop_rgb.2, 220),
                        2,
                        false,
                    );
                }
            }

            BitBlt(hdc_screen, 0, 0, w, h, surf.hdc, 0, 0, SRCCOPY);
        }
        st.surface = Some(surf);

        unsafe {
            paint_icon_layer(
                st,
                hdc_screen,
                w,
                h,
                active_drag.as_ref(),
                shift_down,
                open_t,
                ui_alpha,
            );
        }
    });

    unsafe {
        EndPaint(hwnd, &mut ps);
    }
}

fn draw_stack_dot(dst: &mut [u32], dst_w: i32, dst_h: i32, cx: i32, cy: i32, alpha: u8) {
    if alpha == 0 {
        return;
    }
    let soft = ((alpha as u16 * 150) / 255) as u8;
    for dy in -2..=2_i32 {
        let y = cy + dy;
        if y < 0 || y >= dst_h {
            continue;
        }
        let row = (y * dst_w) as usize;
        for dx in -2..=2_i32 {
            let x = cx + dx;
            if x < 0 || x >= dst_w {
                continue;
            }
            let d2 = dx * dx + dy * dy;
            if d2 <= 1 {
                dst[row + x as usize] = blend_premul(dst[row + x as usize], alpha, 248, 250, 255);
            } else if d2 <= 4 {
                dst[row + x as usize] = blend_premul(dst[row + x as usize], soft, 18, 22, 32);
            }
        }
    }
}

unsafe fn paint_icon_layer(
    st: &mut OverviewState,
    hdc_screen: HDC,
    w: i32,
    h: i32,
    active_drag: Option<&DragState>,
    shift_down: bool,
    open_t: f64,
    ui_alpha: u8,
) {
    if st.icon_hwnd.is_null() {
        return;
    }
    let needs_alloc = st
        .icon_surface
        .as_ref()
        .is_none_or(|s| s.main.w != w || s.main.h != h);
    if needs_alloc {
        st.icon_surface = unsafe { CachedSurface::acquire(hdc_screen, w, h) };
    }
    let Some(icon_surf) = st.icon_surface.take() else {
        return;
    };

    let is_closing = st.closing_jump.is_some();
    let empty_t = st.empty_fade_spring.current.clamp(0.0, 1.0);

    unsafe {
        let pixels = std::slice::from_raw_parts_mut(icon_surf.main.bits, (w * h) as usize);
        pixels.fill(0);
        let scratch_pixels = std::slice::from_raw_parts_mut(
            icon_surf.scratch.bits,
            (icon_surf.scratch.w * icon_surf.scratch.h) as usize,
        );

        // Draw non-hero cards first, and hero card last so it layers above sibling cards.
        let mut draw_order: Vec<usize> = (0..st.boxes.len()).collect();
        draw_order.sort_by_key(|&idx| {
            let wid = st.boxes[idx].window_id;
            if wid.is_some() && wid == st.hero_window_id {
                1u8
            } else {
                0u8
            }
        });

        for idx in draw_order {
            let b = &st.boxes[idx];
            let is_empty = b.window_id.is_none();
            let hovered = st.hover_coord == Some((b.col, b.row));
            let is_drag_source =
                active_drag.is_some_and(|d| d.source_col == b.col && d.source_row == b.row);
            let is_drop_target = active_drag.is_some() && hovered && !is_drag_source;
            let card = animated_card_rect(st, b);

            if is_empty {
                if is_drop_target && empty_t * open_t > 0.05 {
                    draw_shadowed_text_argb(
                        pixels,
                        w,
                        h,
                        icon_surf.scratch.hdc,
                        scratch_pixels,
                        icon_surf.scratch.w,
                        icon_surf.scratch.h,
                        icon_surf.title_font,
                        card,
                        "Drop to MOVE",
                        (158, 226, 106),
                        ui_alpha,
                        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
                    );
                }
                continue;
            }

            let Some(win_id) = b.window_id else {
                continue;
            };
            if is_drag_source {
                continue;
            }

            let is_hero = Some(win_id) == st.hero_window_id;
            let card_alpha = if is_closing {
                if is_hero { 255 } else { ui_alpha }
            } else {
                ui_alpha
            };
            let icon_alpha = if is_closing && is_hero {
                ui_alpha
            } else {
                card_alpha
            };

            let dest = fit_thumb_rect(card, st.thumb_sizes.get(&win_id).copied());
            let dest_r = IRect::new(
                dest.left,
                dest.top,
                (dest.right - dest.left).max(16),
                (dest.bottom - dest.top).max(16),
            );
            let selected = st.cursor == (b.col, b.row);

            let has_thumb = st.thumbs.contains_key(&win_id);
            if !has_thumb && !is_drop_target && card_alpha > 10 {
                draw_rounded_card(
                    pixels,
                    w,
                    h,
                    dest_r,
                    6,
                    (
                        32,
                        38,
                        52,
                        (230.0 * (card_alpha as f64 / 255.0)).round() as u8,
                    ),
                    (70, 80, 105, card_alpha),
                    1,
                    false,
                );
                let title_str = st
                    .snapshot
                    .as_ref()
                    .and_then(|s| s.windows.iter().find(|w| w.id == win_id))
                    .map(|w| w.title.as_str())
                    .unwrap_or("");
                if !title_str.is_empty() && card_alpha > 20 {
                    let title_r = IRect::new(
                        dest_r.x + 12,
                        dest_r.y + (dest_r.h / 3) - 14,
                        (dest_r.w - 24).max(10),
                        28,
                    );
                    draw_shadowed_text_argb(
                        pixels,
                        w,
                        h,
                        icon_surf.scratch.hdc,
                        scratch_pixels,
                        icon_surf.scratch.w,
                        icon_surf.scratch.h,
                        icon_surf.title_font,
                        title_r,
                        title_str,
                        (230, 238, 252),
                        card_alpha,
                        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
                    );
                }
            }

            if is_drop_target {
                let drop_rgb = if shift_down {
                    (122, 162, 247)
                } else {
                    (247, 184, 104)
                };
                draw_rounded_card(
                    pixels,
                    w,
                    h,
                    dest_r,
                    6,
                    (0, 0, 0, 0),
                    (drop_rgb.0, drop_rgb.1, drop_rgb.2, card_alpha.max(1)),
                    2,
                    false,
                );
                let hint = if shift_down || is_ctrl_down() {
                    "Drop to SWAP"
                } else {
                    "Drop to SNAP"
                };
                draw_shadowed_text_argb(
                    pixels,
                    w,
                    h,
                    icon_surf.scratch.hdc,
                    scratch_pixels,
                    icon_surf.scratch.w,
                    icon_surf.scratch.h,
                    icon_surf.title_font,
                    dest_r,
                    hint,
                    drop_rgb,
                    card_alpha,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
                );
            } else if !is_closing && (selected || b.focused) {
                let ring_rgb = if selected {
                    (122, 247, 158)
                } else {
                    (122, 162, 247)
                };
                draw_rounded_card(
                    pixels,
                    w,
                    h,
                    dest_r,
                    6,
                    (0, 0, 0, 0),
                    (ring_rgb.0, ring_rgb.1, ring_rgb.2, card_alpha.max(1)),
                    2,
                    false,
                );
            }

            if !is_drop_target && icon_alpha > 4 {
                let has_thumb = st.thumbs.contains_key(&win_id);
                let cx = dest_r.x + dest_r.w / 2;
                let (icon_sz, ix, iy) = if has_thumb {
                    let sz = (dest_r.w.min(dest_r.h) / 5).clamp(24, 44);
                    let ix = (cx - sz / 2).clamp(2, (w - sz - 2).max(2));
                    let iy = (dest_r.bottom() - sz * 3 / 4)
                        .min(h - sz - 6)
                        .max(dest_r.y + 4);
                    (sz, ix, iy)
                } else {
                    // Windows without live thumbnail (e.g. PowerShell, Task Manager, or minimized)
                    // receive a centered, prominent app icon.
                    let sz = (dest_r.w.min(dest_r.h) * 36 / 100).clamp(44, 72);
                    let cy = dest_r.y + dest_r.h / 2;
                    let ix = (cx - sz / 2).clamp(2, (w - sz - 2).max(2));
                    let iy = (cy - sz / 2).clamp(2, (h - sz - 2).max(2));
                    (sz, ix, iy)
                };

                draw_soft_shadow_circle(
                    pixels,
                    w,
                    h,
                    ix + icon_sz / 2,
                    iy + icon_sz / 2 + 1,
                    (icon_sz * 68) / 100,
                    ((icon_alpha as u16 * 135) / 255) as u8,
                );
                if let Some(icon) = st.icons.get(&win_id) {
                    draw_icon_scaled(
                        pixels,
                        w,
                        h,
                        IRect::new(ix, iy, icon_sz, icon_sz),
                        icon,
                        icon_alpha,
                    );
                }

                if b.stack_len > 1 {
                    let count = b.stack_len.min(5) as i32;
                    let spacing = 7;
                    let dots_w = (count - 1) * spacing;
                    let dot_y = (iy + icon_sz + 3).min(h - 3);
                    let dot_x0 = cx - dots_w / 2;
                    for di in 0..count {
                        draw_stack_dot(pixels, w, h, dot_x0 + di * spacing, dot_y, icon_alpha);
                    }
                }
            }

            if !is_closing && !is_drop_target && active_drag.is_none() && ui_alpha > 12 {
                let close_t = st
                    .close_springs
                    .get(&(b.col, b.row))
                    .map(|s| s.current.clamp(0.0, 1.2))
                    .unwrap_or(0.0);
                if close_t > 0.02 {
                    if let Some(base_btn) = close_button_rect(dest_r, w, h) {
                        let close_hovered = st.hover_close_win == Some(win_id)
                            || st.pending_close.is_some_and(|(_, _, wid)| wid == win_id);
                        let pop_scale =
                            (0.45 + 0.55 * close_t) * if close_hovered { 1.10 } else { 1.0 };
                        let nw = ((base_btn.w as f64) * pop_scale).round().max(8.0) as i32;
                        let nh = ((base_btn.h as f64) * pop_scale).round().max(8.0) as i32;
                        let y_slide = ((1.0 - close_t.min(1.0)) * 5.0).round() as i32;
                        let anim_btn = IRect::new(
                            base_btn.x + (base_btn.w - nw) / 2,
                            (base_btn.y + (base_btn.h - nh) / 2 + y_slide)
                                .clamp(2, (h - nh - 2).max(2)),
                            nw,
                            nh,
                        );
                        let badge_alpha =
                            ((ui_alpha as f64) * close_t.clamp(0.0, 1.0)).round() as u8;
                        draw_close_badge(pixels, w, h, anim_btn, close_hovered, badge_alpha);
                    }
                }
            }
        }

        if let Some(drag) = active_drag {
            let ghost = drag_ghost_rect(st, drag);
            let dest = fit_thumb_rect(ghost, st.thumb_sizes.get(&drag.window_id).copied());
            let dest_r = IRect::new(
                dest.left,
                dest.top,
                (dest.right - dest.left).max(16),
                (dest.bottom - dest.top).max(16),
            );
            draw_rounded_card(
                pixels,
                w,
                h,
                dest_r,
                6,
                (0, 0, 0, 0),
                (122, 162, 247, 255),
                2,
                false,
            );
            let icon_sz = 32;
            let ix = (dest_r.x + dest_r.w / 2 - icon_sz / 2).clamp(2, (w - icon_sz - 2).max(2));
            let iy = (dest_r.bottom() - icon_sz * 3 / 4)
                .min(h - icon_sz - 4)
                .max(dest_r.y + 4);
            draw_soft_shadow_circle(
                pixels,
                w,
                h,
                ix + icon_sz / 2,
                iy + icon_sz / 2 + 1,
                22,
                140,
            );
            if let Some(icon) = st.icons.get(&drag.window_id) {
                draw_icon_scaled(
                    pixels,
                    w,
                    h,
                    IRect::new(ix, iy, icon_sz, icon_sz),
                    icon,
                    255,
                );
            }
        }

        let mut pt_dst = POINT {
            x: st.bounds.x,
            y: st.bounds.y,
        };
        let mut sz = SIZE { cx: w, cy: h };
        let mut pt_src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        UpdateLayeredWindow(
            st.icon_hwnd,
            std::ptr::null_mut(),
            &mut pt_dst,
            &mut sz,
            icon_surf.main.hdc,
            &mut pt_src,
            0,
            &blend,
            ULW_ALPHA,
        );
    }

    st.icon_surface = Some(icon_surf);
}

pub(crate) unsafe fn make_font(height: i32, bold: bool) -> HFONT {
    let face = to_wide("Segoe UI");
    unsafe {
        CreateFontW(
            height,
            0,
            0,
            0,
            if bold {
                FW_BOLD as i32
            } else {
                FW_NORMAL as i32
            },
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            ANTIALIASED_QUALITY as u32,
            DEFAULT_PITCH as u32,
            face.as_ptr(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_layout_fits_all_windows_without_scrolling() {
        for (bw, bh) in [(1920, 1080), (1366, 768), (2560, 1440)] {
            for (cols, rows, drag) in [
                (1, 1, false),
                (5, 1, false),
                (4, 3, false),
                (6, 4, false),
                (6, 3, true),
            ] {
                let (cw, ch, gap, sx, sy) = compute_overview_grid_layout(bw, bh, cols, rows, drag);
                let total_w = cw * cols + gap * (cols - 1);
                let total_h = ch * rows + gap * (rows - 1);
                assert!(cw > 20 && ch > 16);
                assert!(
                    sx >= 0 && sx + total_w <= bw,
                    "horizontal fit failed for {cols}x{rows} in {bw}x{bh}"
                );
                assert!(
                    sy >= 0 && sy + total_h <= bh,
                    "vertical fit failed for {cols}x{rows} in {bw}x{bh}"
                );
            }
        }
    }

    #[test]
    fn close_button_rect_stays_in_top_right_corner_and_screen_bounds() {
        let card = IRect::new(100, 120, 320, 180);
        let btn = close_button_rect(card, 1920, 1080).expect("close button visible");
        assert!(btn.x >= card.x + card.w / 2 && btn.x <= card.right());
        assert!(btn.y >= 2 && btn.bottom() <= 1078);
        assert!(point_in_rect_pad(
            btn.x + btn.w / 2,
            btn.y + btn.h / 2,
            btn,
            4
        ));
        assert!(close_button_rect(IRect::new(0, 0, 40, 30), 1920, 1080).is_none());
    }
}
