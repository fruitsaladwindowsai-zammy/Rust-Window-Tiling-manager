use crate::ipc_pipe::{spawn_ipc_server, PendingIpc};
use crate::win32::dpi::{enable_per_monitor_dpi_v2, enumerate_monitors};
use crate::win32::events::{WinEventKind, WinEventListener};
use crate::win32::gestures::{GestureEvent, TouchpadTracker, WM_INPUT_MSG};
use crate::win32::hotkeys::{self, HotkeyHook};
use crate::win32::hover::HoverFocusTracker;
use crate::win32::overview::{
    drain_overview_commands, OverviewCommand, OverviewOptions, OverviewWindow,
};
use crate::win32::settings_ui::{take_pending_live_config, SettingsWindow};
use crate::win32::tray::{
    popup_tray_menu, TrayIcon, TRAY_CMD_EXIT, TRAY_CMD_OPEN_CONFIG, TRAY_CMD_OVERVIEW,
    TRAY_CMD_PAUSE, TRAY_CMD_RELOAD_CONFIG, TRAY_CMD_RETILE, TRAY_CMD_SETTINGS,
    TRAY_CMD_TOGGLE_WS_LAYOUT,
};
use crate::win32::vdesktop::VirtualDesktopTracker;
use crate::win32::window::{
    center_cursor_in_window, close_os_window, enumerate_manageable_windows,
    enumerate_manageable_windows_opts, focus_os_window, get_foreground_window_id, hwnd_to_id,
    id_to_hwnd, is_manageable_window, is_valid_window, is_window_zoomed,
    read_visible_window_rect, read_window_info, read_window_title, set_cloak, shell_open_file,
    WindowApplier,
};
use crate::win32::{
    to_wide, VsyncPacer, TIMER_DISPLAY_SYNC_ID, TIMER_HOVER_FOCUS_ID, TIMER_PERSIST_SAVE_ID,
    WM_APP_ACTION, WM_APP_APPLY_LIVE_CONFIG, WM_APP_IPC_NOTIFY, WM_APP_OVERVIEW_CMD,
    WM_APP_RELOAD_CONFIG, WM_APP_TRAY, WM_APP_VDESKTOP_CHANGED,
    WM_APP_VSYNC_STEP, WM_APP_WIN_EVENT,
};
use winscroller2d_core::config::parse_hex_colorref;
use winscroller2d_core::{
    Action, Config, IpcRequest, IpcResponse, PersistedState, WorkspaceManager,
};
use std::cell::RefCell;
use std::mem::{size_of, zeroed};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Instant;
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, TRUE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, KillTimer,
    PostMessageW, PostQuitMessage, RegisterClassExW, SetTimer, TranslateMessage, MSG, WM_CLOSE,
    WM_DESTROY, WM_DISPLAYCHANGE, WM_HOTKEY, WM_SETTINGCHANGE, WM_TIMER, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};

const CONTROLLER_CLASS: &str = "WinScroller2dControllerWindow";
static CTRL_C_REQUESTED: AtomicBool = AtomicBool::new(false);
static IGNORE_NEXT_WATCH_RELOAD: AtomicBool = AtomicBool::new(false);

struct AppState {
    own_pid: u32,
    config_path: PathBuf,
    state_path: PathBuf,
    mgr: WorkspaceManager,
    applier: WindowApplier,
    tray: TrayIcon,
    overview: Option<OverviewWindow>,
    settings_win: Option<SettingsWindow>,
    touchpad: TouchpadTracker,
    hover: HoverFocusTracker,
    vdesktop: VirtualDesktopTracker,
    hotkey_hook: HotkeyHook,
    ipc_rx: mpsc::Receiver<PendingIpc>,
    pacer: VsyncPacer,
    last_tick: Instant,
}

thread_local! {
    static APP_STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
}

pub fn default_config_path() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("WinScroller2D").join("winscroller2d.toml")
    } else {
        PathBuf::from("winscroller2d.toml")
    }
}

fn default_state_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.join("state.json"))
        .unwrap_or_else(|| PathBuf::from("state.json"))
}

fn overview_options(cfg: &Config) -> OverviewOptions {
    OverviewOptions {
        blur_mode: cfg.appearance.blur_mode,
        alpha: cfg.appearance.overview_alpha,
        stiffness: cfg.animation.stiffness,
        damping_ratio: cfg.animation.damping_ratio,
        min_w_frac: cfg.layout.overview_min_width,
        min_h_frac: cfg.layout.overview_min_height,
        autoscroll_speed: cfg.layout.overview_autoscroll_speed,
    }
}

fn schedule_state_save(st: &AppState, ctrl_hwnd: HWND) {
    if st.mgr.state_dirty && st.mgr.config.layout.persist_state {
        unsafe {
            SetTimer(ctrl_hwnd, TIMER_PERSIST_SAVE_ID, 600, None);
        }
    }
}

fn flush_persisted_state(st: &mut AppState) {
    if !st.mgr.config.layout.persist_state || !st.mgr.state_dirty {
        return;
    }
    let exported = st.mgr.export_persisted_state();
    match exported.to_json_pretty() {
        Ok(json) => {
            if let Some(parent) = st.state_path.parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }
            let tmp = st.state_path.with_extension("json.tmp");
            if std::fs::write(&tmp, &json).is_ok() {
                if std::fs::rename(&tmp, &st.state_path).is_err() {
                    let _ = std::fs::write(&st.state_path, &json);
                    let _ = std::fs::remove_file(&tmp);
                }
            } else {
                let _ = std::fs::write(&st.state_path, &json);
            }
            st.mgr.state_dirty = false;
        }
        Err(err) => {
            eprintln!("[winscroller2d] failed serializing state.json: {err}");
        }
    }
}

pub fn ensure_config_exists(path: &Path) -> Result<Config, String> {
    if !path.exists() {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("failed creating config directory {}: {e}", parent.display()))?;
            }
        }
        std::fs::write(path, Config::default_toml_template())
            .map_err(|e| format!("failed writing default config {}: {e}", path.display()))?;
        println!("[winscroller2d] wrote default config to {}", path.display());
    }
    load_config_file(path)
}

pub fn load_config_file(path: &Path) -> Result<Config, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("failed reading {}: {e}", path.display()))?;
    Config::from_toml_str(&text)
}

pub fn run_daemon(config_path: PathBuf) -> Result<(), String> {
    enable_per_monitor_dpi_v2();

    let config = ensure_config_exists(&config_path)?;
    let state_path = default_state_path(&config_path);
    let own_pid = std::process::id();

    let ctrl_hwnd = create_controller_window()?;
    install_ctrl_c_handler(ctrl_hwnd);

    let vdesktop = VirtualDesktopTracker::new(ctrl_hwnd);
    let ds = vdesktop.current_state();
    let monitors = enumerate_monitors();
    let mut mgr = WorkspaceManager::new(config.clone(), monitors);
    mgr.sync_os_desktops(ds.desktops, ds.current);
    if config.layout.persist_state && state_path.exists() {
        if let Ok(text) = std::fs::read_to_string(&state_path) {
            if let Ok(saved) = PersistedState::from_json_str(&text) {
                mgr.import_persisted_state(saved);
            }
        }
    }
    for mut w in enumerate_manageable_windows_opts(own_pid, true) {
        let whwnd = id_to_hwnd(w.id);
        w.desktop_id = vdesktop.window_desktop_id(whwnd);
        if w.desktop_id.is_some() || is_manageable_window(whwnd, own_pid, false) {
            mgr.add_window(w);
        }
    }
    // Keep saved entries for not-yet-reopened windows intact until user/session changes occur.
    mgr.state_dirty = false;
    if let Some(fg) = get_foreground_window_id() {
        mgr.on_foreground_changed(fg);
    }

    let tray = TrayIcon::create(ctrl_hwnd);
    let overview = OverviewWindow::create(ctrl_hwnd).ok();
    let settings_win = SettingsWindow::create(ctrl_hwnd).ok();
    let touchpad = TouchpadTracker::default();
    touchpad.register_sink(ctrl_hwnd);
    let ipc_rx = spawn_ipc_server(ctrl_hwnd as usize);
    spawn_config_watcher(config_path.clone(), ctrl_hwnd as usize);

    let _event_listener = WinEventListener::install(ctrl_hwnd);
    let hotkey_hook = HotkeyHook::install(ctrl_hwnd, &config.bindings)?;
    let pacer = VsyncPacer::new(ctrl_hwnd, WM_APP_VSYNC_STEP);

    APP_STATE.with(|cell| {
        *cell.borrow_mut() = Some(AppState {
            own_pid,
            config_path: config_path.clone(),
            state_path,
            mgr,
            applier: WindowApplier::new(),
            tray,
            overview,
            settings_win,
            touchpad,
            hover: HoverFocusTracker::default(),
            vdesktop,
            hotkey_hook,
            ipc_rx,
            pacer,
            last_tick: Instant::now(),
        });
    });

    // Initial tile pass + periodic 1s health sweep + 30ms hover-peek tracker.
    with_app(|st| {
        apply_frame_now(st, ctrl_hwnd, false);
    });
    unsafe {
        SetTimer(ctrl_hwnd, TIMER_DISPLAY_SYNC_ID, 1000, None);
        SetTimer(ctrl_hwnd, TIMER_HOVER_FOCUS_ID, 30, None);
    }

    println!(
        "[winscroller2d] running (config: {}). Press Win+Shift+P to pause, Win+O for 2D overview, Win+Alt+S for settings, Win+Shift+Esc to quit.",
        config_path.display()
    );

    unsafe {
        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    // Guaranteed cleanup so all cloaked windows and borders are restored and centered on exit.
    with_app(|st| {
        let extra_ids: Vec<_> = st
            .mgr
            .snapshot()
            .workspaces
            .into_iter()
            .flat_map(|w| w.windows)
            .map(|w| w.id)
            .collect();
        st.applier.restore_all(&extra_ids);
    });
    APP_STATE.with(|cell| {
        *cell.borrow_mut() = None;
    });
    println!("[winscroller2d] exited cleanly; all windows restored.");
    Ok(())
}

fn with_app<R>(f: impl FnOnce(&mut AppState) -> R) -> Option<R> {
    APP_STATE.with(|cell| {
        let Ok(mut guard) = cell.try_borrow_mut() else {
            return None;
        };
        guard.as_mut().map(f)
    })
}

fn create_controller_window() -> Result<HWND, String> {
    let hinst = unsafe { GetModuleHandleW(std::ptr::null()) };
    let class_w = to_wide(CONTROLLER_CLASS);
    let title_w = to_wide("WinScroller 2D Controller");
    let wc = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(controller_wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: hinst,
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_w.as_ptr(),
        hIconSm: std::ptr::null_mut(),
    };
    unsafe {
        RegisterClassExW(&wc);
    }
    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_w.as_ptr(),
            title_w.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        Err("CreateWindowExW for controller failed".into())
    } else {
        Ok(hwnd)
    }
}

fn install_ctrl_c_handler(ctrl_hwnd: HWND) {
    static CTRL_HWND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
    CTRL_HWND.store(ctrl_hwnd as isize, Ordering::SeqCst);

    unsafe extern "system" fn handler(_ctrl_type: u32) -> BOOL {
        CTRL_C_REQUESTED.store(true, Ordering::SeqCst);
        let hwnd = CTRL_HWND.load(Ordering::SeqCst) as HWND;
        if !hwnd.is_null() {
            unsafe {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
        }
        TRUE
    }
    unsafe {
        SetConsoleCtrlHandler(Some(handler), TRUE);
    }
}

fn spawn_config_watcher(path: PathBuf, target_hwnd: usize) {
    std::thread::Builder::new()
        .name("winscroller2d-config-watch".into())
        .spawn(move || {
            let mut last_mod = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(450));
                if let Ok(meta) = std::fs::metadata(&path) {
                    if let Ok(mtime) = meta.modified() {
                        if last_mod != Some(mtime) {
                            last_mod = Some(mtime);
                            unsafe {
                                PostMessageW(target_hwnd as HWND, WM_APP_RELOAD_CONFIG, 0, 0);
                            }
                        }
                    }
                }
            }
        })
        .expect("spawn config watcher");
}

unsafe extern "system" fn controller_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_HOTKEY => {
            let id = wparam as i32;
            if id >= 1000 {
                let slot = (id - 1000) as usize;
                if let Some(action) = hotkeys::get_action_by_slot(slot) {
                    with_app(|st| execute_action(st, hwnd, action));
                }
            }
            0
        }
        WM_APP_ACTION => {
            let slot = wparam;
            let generation = lparam as u64;
            if let Some(action) = hotkeys::take_action_by_slot(slot, generation) {
                with_app(|st| execute_action(st, hwnd, action));
            }
            0
        }
        WM_APP_WIN_EVENT => {
            let Some(kind) = WinEventKind::from_raw(wparam as u32) else {
                return 0;
            };
            let target_hwnd = lparam as HWND;
            with_app(|st| handle_win_event(st, hwnd, kind, target_hwnd));
            0
        }
        WM_APP_TRAY => {
            // Release APP_STATE borrow before entering TrackPopupMenu's modal message loop
            // so WM_TIMER / WinEvent callbacks cannot panic with RefCell BorrowMutError.
            let Some((paused, ws_floating, config_path_str)) = with_app(|st| {
                (
                    st.mgr.paused,
                    st.mgr.active_workspace_layout_disabled(),
                    st.config_path.to_string_lossy().to_string(),
                )
            }) else {
                return 0;
            };
            if let Some(cmd) = popup_tray_menu(hwnd, lparam, paused, ws_floating) {
                match cmd {
                    TRAY_CMD_OVERVIEW => {
                        with_app(|st| execute_action(st, hwnd, Action::ToggleOverview));
                    }
                    TRAY_CMD_SETTINGS => {
                        with_app(|st| execute_action(st, hwnd, Action::OpenSettings));
                    }
                    TRAY_CMD_TOGGLE_WS_LAYOUT => {
                        with_app(|st| execute_action(st, hwnd, Action::ToggleWorkspaceLayout));
                    }
                    TRAY_CMD_PAUSE => {
                        with_app(|st| execute_action(st, hwnd, Action::TogglePause));
                    }
                    TRAY_CMD_RETILE => {
                        with_app(|st| execute_action(st, hwnd, Action::Retile));
                    }
                    TRAY_CMD_OPEN_CONFIG => shell_open_file(&config_path_str),
                    TRAY_CMD_RELOAD_CONFIG => {
                        with_app(|st| execute_action(st, hwnd, Action::ReloadConfig));
                    }
                    TRAY_CMD_EXIT => {
                        with_app(|st| execute_action(st, hwnd, Action::Quit));
                    }
                    _ => {}
                }
            }
            0
        }
        WM_APP_IPC_NOTIFY => {
            with_app(|st| drain_ipc_requests(st, hwnd));
            0
        }
        WM_APP_RELOAD_CONFIG => {
            if !IGNORE_NEXT_WATCH_RELOAD.swap(false, Ordering::SeqCst) {
                with_app(|st| reload_config(st, hwnd));
            }
            0
        }
        WM_APP_APPLY_LIVE_CONFIG => {
            if let Some(cfg) = take_pending_live_config() {
                with_app(|st| apply_live_config(st, hwnd, cfg));
            }
            0
        }
        WM_APP_VDESKTOP_CHANGED => {
            with_app(|st| {
                sync_os_desktop_state(st, hwnd);
            });
            0
        }
        WM_APP_OVERVIEW_CMD => {
            let cmds = drain_overview_commands();
            with_app(|st| {
                for cmd in cmds {
                    match cmd {
                        OverviewCommand::Place {
                            window_id,
                            col,
                            row,
                            stack,
                        } => {
                            execute_action(
                                st,
                                hwnd,
                                Action::PlaceWindowAt {
                                    window_id,
                                    col,
                                    row,
                                    stack,
                                },
                            );
                        }
                        OverviewCommand::SplitHorizontal {
                            window_id,
                            target_window_id,
                            on_left,
                        } => {
                            execute_action(
                                st,
                                hwnd,
                                Action::SplitWindowHorizontal {
                                    window_id,
                                    target_window_id,
                                    on_left,
                                },
                            );
                        }
                        OverviewCommand::CloseWindow { window_id } => {
                            close_os_window(window_id);
                        }
                        OverviewCommand::FinishClose { jump } => {
                            if let Some((col, row, window_id)) = jump {
                                execute_action(
                                    st,
                                    hwnd,
                                    Action::JumpToCell {
                                        col,
                                        row,
                                        window_id,
                                    },
                                );
                            } else {
                                st.mgr.overview_open = false;
                                st.hover.reset();
                                // Uncloak and position tiles before hiding overview & restoring focus.
                                apply_frame_now(st, hwnd, true);
                                sync_overview(st, false);
                                if let Some(fid) = st.mgr.snapshot().focused_window_id {
                                    focus_os_window(fid);
                                }
                            }
                        }
                    }
                }
            });
            0
        }
        WM_INPUT_MSG => {
            with_app(|st| {
                let can_desktop_pan = !st.mgr.paused
                    && !st.mgr.overview_open
                    && !st.mgr.active_workspace_layout_disabled()
                    && st.mgr.config.layout.three_finger_pan;
                let can_overview_pan = !st.mgr.paused
                    && st.mgr.overview_open
                    && st.mgr.config.layout.overview_thumbnails
                    && st.mgr.config.layout.three_finger_pan;
                let sens = st.mgr.config.layout.gesture_pan_sensitivity;
                if let Some(ev) = st
                    .touchpad
                    .handle_raw_input(lparam, sens, can_desktop_pan || can_overview_pan)
                {
                    match ev {
                        GestureEvent::PanDelta { dx, dy } => {
                            if can_desktop_pan {
                                execute_action(st, hwnd, Action::PanPixels { dx, dy });
                            } else if can_overview_pan {
                                if let Some(ref mut ov) = st.overview {
                                    ov.scroll_by(dx, dy);
                                }
                            }
                        }
                        GestureEvent::PanCommit { vx, vy } => {
                            if can_desktop_pan {
                                execute_action(st, hwnd, Action::PanCommit { vx, vy });
                            }
                        }
                        GestureEvent::OverviewToggle { open } => {
                            if !st.mgr.paused && open != st.mgr.overview_open {
                                execute_action(st, hwnd, Action::ToggleOverview);
                            }
                        }
                    }
                }
            });
            0
        }
        WM_APP_VSYNC_STEP => {
            with_app(|st| {
                let now = Instant::now();
                let dt = (now - st.last_tick).as_secs_f64();
                st.last_tick = now;
                apply_frame_step(st, dt);
            });
            0
        }
        WM_TIMER => {
            if wparam == TIMER_DISPLAY_SYNC_ID {
                with_app(|st| {
                    if CTRL_C_REQUESTED.load(Ordering::Relaxed) {
                        unsafe { PostMessageW(hwnd, WM_CLOSE, 0, 0) };
                        return;
                    }
                    reconcile_windows(st, hwnd);
                });
            } else if wparam == TIMER_HOVER_FOCUS_ID {
                with_app(|st| poll_hover_focus(st, hwnd));
            } else if wparam == TIMER_PERSIST_SAVE_ID {
                unsafe {
                    KillTimer(hwnd, TIMER_PERSIST_SAVE_ID);
                }
                with_app(|st| flush_persisted_state(st));
            }
            0
        }
        WM_DISPLAYCHANGE | WM_SETTINGCHANGE => {
            with_app(|st| {
                st.mgr.sync_monitors(enumerate_monitors());
                apply_frame_now(st, hwnd, true);
            });
            0
        }
        WM_CLOSE => {
            let ui = with_app(|st| {
                st.pacer.set_active(false);
                flush_persisted_state(st);
                (st.settings_win.take(), st.overview.take())
            });
            if let Some((mut sw_opt, mut ov_opt)) = ui {
                if let Some(ref mut sw) = sw_opt {
                    sw.hide();
                }
                if let Some(ref mut ov) = ov_opt {
                    ov.hide();
                }
            }
            with_app(|st| {
                let extra_ids: Vec<_> = st
                    .mgr
                    .snapshot()
                    .workspaces
                    .into_iter()
                    .flat_map(|w| w.windows)
                    .map(|w| w.id)
                    .collect();
                st.applier.restore_all(&extra_ids);
            });
            unsafe {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                KillTimer(hwnd, TIMER_DISPLAY_SYNC_ID);
                KillTimer(hwnd, TIMER_HOVER_FOCUS_ID);
                KillTimer(hwnd, TIMER_PERSIST_SAVE_ID);
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn execute_action(st: &mut AppState, ctrl_hwnd: HWND, action: Action) {
    static LAST_ACTION: std::sync::Mutex<Option<(Action, Instant)>> = std::sync::Mutex::new(None);
    if let Ok(mut guard) = LAST_ACTION.lock() {
        let now = Instant::now();
        if let Some((ref last_act, last_time)) = *guard {
            if *last_act == action && now.duration_since(last_time).as_millis() < 160 {
                return;
            }
        }
        *guard = Some((action.clone(), now));
    }

    if matches!(action, Action::ToggleOverview) && !st.mgr.overview_open {
        if let Some(fg) = get_foreground_window_id() {
            st.mgr.on_foreground_changed(fg);
        }
        reconcile_windows(st, ctrl_hwnd);
        capture_external_snaps(st);
    }
    if matches!(action, Action::ToggleOverview | Action::Retile) && !st.mgr.overview_open {
        sync_os_desktop_state(st, ctrl_hwnd);
    }

    if matches!(action, Action::ToggleOverview)
        && st.mgr.overview_open
        && st.mgr.config.layout.overview_thumbnails
    {
        if let Some(ref mut ov) = st.overview {
            if ov.is_visible() {
                ov.begin_close(None);
                return;
            }
        }
    }

    let center_on_jump = matches!(action, Action::JumpToCell { .. });
    let effects = st.mgr.dispatch(action);

    if let Some(paused) = effects.paused {
        hotkeys::set_paused(paused);
        st.tray.update_status(paused);
        st.hover.reset();
        if paused {
            let extra_ids: Vec<_> = st
                .mgr
                .snapshot()
                .workspaces
                .into_iter()
                .flat_map(|w| w.windows)
                .map(|w| w.id)
                .collect();
            st.applier.restore_all(&extra_ids);
            if let Some(ref mut ov) = st.overview {
                ov.hide();
            }
        }
    }

    if let Some((win_id, ref target_desktop)) = effects.moved_to_desktop {
        let whwnd = id_to_hwnd(win_id);
        st.vdesktop.move_window_to_desktop(whwnd, target_desktop);
        if target_desktop != st.mgr.active_os_desktop() {
            set_cloak(whwnd, true);
        }
    }

    if effects.open_settings {
        let mon_bounds = {
            let snap = st.mgr.snapshot();
            snap.monitors
                .iter()
                .find(|m| m.is_focused)
                .or_else(|| snap.monitors.first())
                .map(|m| winscroller2d_core::IRect::new(m.bounds.0, m.bounds.1, m.bounds.2, m.bounds.3))
                .unwrap_or(winscroller2d_core::IRect::new(0, 0, 1920, 1080))
        };
        if let Some(ref mut sw) = st.settings_win {
            sw.show(&st.mgr.config, mon_bounds);
        }
    }

    let mut pending_reveal: Option<winscroller2d_core::IRect> = None;
    if let Some(open) = effects.overview_open {
        st.hover.reset();
        if open {
            if !st.mgr.config.layout.overview_thumbnails {
                // Fallback mode resizes actual windows smoothly via `compute_frame`.
                apply_frame_now(st, ctrl_hwnd, true);
            } else {
                let snap = st.mgr.snapshot();
                let focused_mon = snap
                    .monitors
                    .iter()
                    .find(|m| m.is_focused)
                    .or_else(|| snap.monitors.first());
                if let (Some(mon), Some(ws)) = (focused_mon, st.mgr.active_workspace_snapshot()) {
                    let (x, y, w, h) = mon.bounds;
                    let (wx, wy, ww, wh) = mon.work_area;
                    let bounds = winscroller2d_core::IRect::new(x, y, w, h);
                    let px = st.mgr.config.layout.peek_x.max(0);
                    let py = st.mgr.config.layout.peek_y.max(0);
                    let return_rect = st
                        .mgr
                        .focused_window_rect_on_monitor(bounds)
                        .unwrap_or_else(|| {
                            winscroller2d_core::IRect::new(
                                (wx - x) + px,
                                (wy - y) + py,
                                (ww - px * 2).max(64),
                                (wh - py * 2).max(64),
                            )
                        });
                    let opts = overview_options(&st.mgr.config);
                    if let Some(ref mut ov) = st.overview {
                        ov.prepare_show(bounds, return_rect, ws, opts);
                        pending_reveal = Some(bounds);
                    }
                } else {
                    st.mgr.overview_open = false;
                }
            }
        } else {
            apply_frame_now(st, ctrl_hwnd, true);
            sync_overview(st, false);
            if effects.focus_window.is_none() {
                if let Some(fid) = st.mgr.snapshot().focused_window_id {
                    focus_os_window(fid);
                }
            }
        }
    }

    if let Some(win_id) = effects.close_window {
        close_os_window(win_id);
    }

    if effects.reload_config {
        reload_config(st, ctrl_hwnd);
    }

    if effects.layout_dirty || pending_reveal.is_some() {
        apply_frame_now(st, ctrl_hwnd, true);
        if let Some(bounds) = pending_reveal {
            if let Some(ref mut ov) = st.overview {
                ov.reveal(bounds);
            }
        } else if effects.layout_dirty
            && st.mgr.overview_open
            && st.mgr.config.layout.overview_thumbnails
        {
            if let Some(ws) = st.mgr.active_workspace_snapshot() {
                let opts = overview_options(&st.mgr.config);
                if let Some(ref mut ov) = st.overview {
                    ov.refresh(ws, opts);
                }
            }
        }
    }

    if let Some(focus_id) = effects.focus_window {
        if !st.mgr.overview_open {
            focus_os_window(focus_id);
            if center_on_jump && st.mgr.config.layout.center_cursor_on_peek {
                center_cursor_in_window(focus_id);
                st.hover.reset();
            }
        }
    }

    if effects.state_dirty || st.mgr.state_dirty {
        schedule_state_save(st, ctrl_hwnd);
    }

    if effects.quit {
        unsafe {
            PostMessageW(ctrl_hwnd, WM_CLOSE, 0, 0);
        }
    }
}

fn sync_overview(st: &mut AppState, open: bool) {
    let Some(ref mut ov) = st.overview else {
        return;
    };
    if !open {
        ov.hide();
    }
}

fn reload_config(st: &mut AppState, ctrl_hwnd: HWND) {
    match load_config_file(&st.config_path) {
        Ok(cfg) => {
            st.hotkey_hook.reload(&cfg.bindings);
            st.mgr.update_config(cfg);
            if let Some(ref mut ov) = st.overview {
                ov.update_options(overview_options(&st.mgr.config));
            }
            if let Some(ref mut sw) = st.settings_win {
                sw.sync_config(&st.mgr.config);
            }
            apply_frame_now(st, ctrl_hwnd, true);
            println!("[winscroller2d] reloaded config from {}", st.config_path.display());
        }
        Err(err) => {
            eprintln!("[winscroller2d] keeping previous config; reload error: {err}");
        }
    }
}

fn apply_live_config(st: &mut AppState, ctrl_hwnd: HWND, cfg: Config) {
    st.hotkey_hook.reload(&cfg.bindings);
    match cfg.to_toml_string() {
        Ok(toml_text) => {
            IGNORE_NEXT_WATCH_RELOAD.store(true, Ordering::SeqCst);
            if let Err(err) = std::fs::write(&st.config_path, toml_text) {
                IGNORE_NEXT_WATCH_RELOAD.store(false, Ordering::SeqCst);
                eprintln!(
                    "[winscroller2d] failed saving live settings to {}: {err}",
                    st.config_path.display()
                );
            }
        }
        Err(err) => {
            eprintln!("[winscroller2d] failed serializing live settings: {err}");
        }
    }
    st.mgr.update_config(cfg);
    if let Some(ref mut ov) = st.overview {
        ov.update_options(overview_options(&st.mgr.config));
    }
    apply_frame_now(st, ctrl_hwnd, true);
}

fn poll_hover_focus(st: &mut AppState, ctrl_hwnd: HWND) {
    if st.mgr.paused
        || st.mgr.overview_open
        || !st.mgr.config.layout.hover_focus_peeks
        || st.mgr.active_workspace_layout_disabled()
    {
        return;
    }
    if st
        .mgr
        .active_workspace_snapshot()
        .is_some_and(|w| w.windows.iter().any(|x| x.fullscreen))
    {
        return;
    }
    // Pause hover-focus transitions briefly while typing so focus never slips out of an input field.
    if hotkeys::ms_since_last_key() < 800 {
        return;
    }

    let focused_win = st.mgr.snapshot().focused_window_id;
    let os_fg = get_foreground_window_id();
    // Never steal keyboard focus away from dialogs, dropdowns, Start/Search,
    // SettingsWindow, floating windows, or any non-tiled foreground window.
    if os_fg != focused_win || focused_win.is_some_and(|fid| st.mgr.is_floating(fid)) {
        st.hover.reset();
        return;
    }

    let delay_ms = st.mgr.config.layout.hover_focus_delay_ms;
    let mgr = &st.mgr;
    let hit = st.hover.poll(delay_ms, focused_win, |id| {
        mgr.is_managed(id) && !mgr.is_cloaked_by_wm(id) && !mgr.is_floating(id)
    });

    if let Some(hit_id) = hit {
        if st.mgr.on_foreground_changed(hit_id) {
            focus_os_window(hit_id);
            apply_frame_now(st, ctrl_hwnd, true);
            if st.mgr.config.layout.center_cursor_on_peek {
                center_cursor_in_window(hit_id);
            }
            st.hover.reset();
        }
    }
}

fn handle_win_event(
    st: &mut AppState,
    ctrl_hwnd: HWND,
    kind: WinEventKind,
    target_hwnd: HWND,
) {
    let id = hwnd_to_id(target_hwnd);
    match kind {
        WinEventKind::Show | WinEventKind::Uncloaked | WinEventKind::MinimizeEnd => {
            if kind == WinEventKind::Show {
                st.hover.reset();
            }
            sync_os_desktop_state(st, ctrl_hwnd);
            if st.mgr.is_managed(id) {
                if let Some(did) = st.vdesktop.window_desktop_id(target_hwnd) {
                    if st.mgr.move_window_to_os_desktop(id, &did) {
                        apply_frame_now(st, ctrl_hwnd, true);
                        refresh_overview_if_open(st);
                        schedule_state_save(st, ctrl_hwnd);
                    }
                }
            } else if is_manageable_window(target_hwnd, st.own_pid, false) {
                capture_external_snaps(st);
                if let Some(mut info) = read_window_info(target_hwnd) {
                    info.desktop_id = st.vdesktop.window_desktop_id(target_hwnd);
                    if st.mgr.add_window(info) {
                        apply_frame_now(st, ctrl_hwnd, true);
                        refresh_overview_if_open(st);
                        schedule_state_save(st, ctrl_hwnd);
                    }
                }
            }
        }
        WinEventKind::Destroy | WinEventKind::Hide | WinEventKind::MinimizeStart => {
            if st.mgr.is_managed(id) {
                let should_remove = kind == WinEventKind::Destroy || !is_valid_window(id);
                if should_remove && st.mgr.remove_window(id) {
                    st.applier.forget(id);
                    apply_frame_now(st, ctrl_hwnd, true);
                    refresh_overview_if_open(st);
                    schedule_state_save(st, ctrl_hwnd);
                }
            }
        }
        WinEventKind::Cloaked => {
            sync_os_desktop_state(st, ctrl_hwnd);
            if st.mgr.is_managed(id) {
                if let Some(did) = st.vdesktop.window_desktop_id(target_hwnd) {
                    if st.mgr.move_window_to_os_desktop(id, &did) {
                        apply_frame_now(st, ctrl_hwnd, true);
                        refresh_overview_if_open(st);
                        schedule_state_save(st, ctrl_hwnd);
                    }
                }
            }
        }
        WinEventKind::Foreground => {
            st.hover.reset();
            capture_external_snaps(st);
            sync_os_desktop_state(st, ctrl_hwnd);
            if let Some(did) = st.vdesktop.window_desktop_id(target_hwnd) {
                if st.mgr.is_managed(id) {
                    let _ = st.mgr.move_window_to_os_desktop(id, &did);
                }
            }
            if st.mgr.on_foreground_changed(id) {
                apply_frame_now(st, ctrl_hwnd, true);
            } else if is_manageable_window(target_hwnd, st.own_pid, false) {
                capture_external_snaps(st);
                if let Some(mut info) = read_window_info(target_hwnd) {
                    info.desktop_id = st.vdesktop.window_desktop_id(target_hwnd);
                    if st.mgr.add_window(info) {
                        apply_frame_now(st, ctrl_hwnd, true);
                        refresh_overview_if_open(st);
                        schedule_state_save(st, ctrl_hwnd);
                    }
                }
            }
        }
        WinEventKind::MoveSizeEnd => {
            if st.mgr.is_managed(id) && !st.mgr.is_floating(id) {
                let hmon = unsafe { MonitorFromWindow(target_hwnd, MONITOR_DEFAULTTONEAREST) } as usize as u64;
                if let Some(owner_mon) = st.mgr.monitor_owning_window(id) {
                    if owner_mon != hmon {
                        if let Some(target_ws) = st.mgr.active_workspace_for_monitor(hmon) {
                            if let Some(target_desktop) = st.mgr.move_window_to_workspace(id, target_ws) {
                                st.vdesktop.move_window_to_desktop(target_hwnd, &target_desktop);
                            }
                            st.mgr.set_focused_monitor(hmon);
                            apply_frame_now(st, ctrl_hwnd, true);
                            refresh_overview_if_open(st);
                            schedule_state_save(st, ctrl_hwnd);
                            return;
                        }
                    }
                }
                if capture_window_snap_from_os(st, id, target_hwnd, 16) {
                    apply_frame_now(st, ctrl_hwnd, true);
                    refresh_overview_if_open(st);
                    schedule_state_save(st, ctrl_hwnd);
                    return;
                }
                let mut moved_dir = None;
                let snap = st.mgr.snapshot();
                if let Some(mon) = snap
                    .monitors
                    .iter()
                    .find(|m| m.is_focused)
                    .or_else(|| snap.monitors.first())
                {
                    let (wx, wy, ww, wh) = mon.work_area;
                    let mut pt: windows_sys::Win32::Foundation::POINT = unsafe { zeroed() };
                    if unsafe {
                        windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut pt) != 0
                    } {
                        if pt.x < wx + (ww * 20) / 100 {
                            moved_dir = Some(winscroller2d_core::Direction::Left);
                        } else if pt.x > wx + (ww * 80) / 100 {
                            moved_dir = Some(winscroller2d_core::Direction::Right);
                        } else if pt.y < wy + (wh * 18) / 100 {
                            moved_dir = Some(winscroller2d_core::Direction::Up);
                        } else if pt.y > wy + (wh * 82) / 100 {
                            moved_dir = Some(winscroller2d_core::Direction::Down);
                        }
                    }
                }
                if let Some(dir) = moved_dir {
                    execute_action(st, ctrl_hwnd, Action::MoveWindow { direction: dir });
                } else {
                    apply_frame_now(st, ctrl_hwnd, true);
                }
            }
        }
        WinEventKind::NameChange => {
            if st.mgr.is_managed(id) {
                st.mgr.update_window_title(id, read_window_title(target_hwnd));
            }
        }
    }
}

fn capture_window_snap_from_os(
    st: &mut AppState,
    id: u64,
    hwnd: HWND,
    min_delta_px: i32,
) -> bool {
    if !st.mgr.is_managed(id) || st.mgr.is_floating(id) || st.mgr.overview_open {
        return false;
    }
    if is_window_zoomed(hwnd) {
        return !st.mgr.is_window_maximized(id) && st.mgr.record_window_maximized(id, true);
    }
    let Some(prev) = st.applier.last_target_rect(id) else {
        return false;
    };
    let Some(vis) = read_visible_window_rect(hwnd) else {
        return false;
    };
    if (vis.w - prev.w).abs() <= min_delta_px && (vis.h - prev.h).abs() <= min_delta_px {
        return false;
    }
    let Some((max_w, max_h)) = st.mgr.window_max_cell_size(id) else {
        return false;
    };
    let wf = vis.w as f64 / max_w.max(1.0);
    let hf = vis.h as f64 / max_h.max(1.0);
    st.mgr.record_window_snap(id, wf, hf)
}

fn capture_external_snaps(st: &mut AppState) {
    if st.mgr.overview_open || st.pacer.is_active() {
        return;
    }
    let ids = st.mgr.all_managed_windows();
    let min_age = std::time::Duration::from_millis(300);
    for id in ids {
        if st.mgr.is_floating(id) || st.mgr.is_cloaked_by_wm(id) {
            continue;
        }
        let Some((_rect, cloaked)) = st.applier.settled_target_rect(id, min_age) else {
            continue;
        };
        if cloaked {
            continue;
        }
        let hwnd = id_to_hwnd(id);
        let _ = capture_window_snap_from_os(st, id, hwnd, 64);
    }
}

fn refresh_overview_if_open(st: &mut AppState) {
    if st.mgr.overview_open && st.mgr.config.layout.overview_thumbnails {
        if let Some(ws) = st.mgr.active_workspace_snapshot() {
            let opts = overview_options(&st.mgr.config);
            if let Some(ref mut ov) = st.overview {
                ov.refresh(ws, opts);
            }
        }
    }
}

fn sync_os_desktop_state(st: &mut AppState, ctrl_hwnd: HWND) -> bool {
    let ds = st.vdesktop.current_state();
    let changed = st.mgr.sync_os_desktops(ds.desktops, ds.current);
    if changed {
        st.hover.reset();
        if st.mgr.overview_open {
            st.mgr.overview_open = false;
            sync_overview(st, false);
        }
        for mut info in enumerate_manageable_windows(st.own_pid) {
            let hwnd = id_to_hwnd(info.id);
            info.desktop_id = st.vdesktop.window_desktop_id(hwnd);
            if st.mgr.is_managed(info.id) {
                if let Some(ref did) = info.desktop_id {
                    st.mgr.move_window_to_os_desktop(info.id, did);
                }
            } else {
                st.mgr.add_window(info);
            }
        }
        if let Some(fg) = get_foreground_window_id() {
            st.mgr.on_foreground_changed(fg);
        }
        apply_frame_now(st, ctrl_hwnd, true);
        schedule_state_save(st, ctrl_hwnd);
    }
    changed
}

fn reconcile_windows(st: &mut AppState, ctrl_hwnd: HWND) {
    capture_external_snaps(st);
    sync_os_desktop_state(st, ctrl_hwnd);

    let mut changed = false;
    let managed = st.mgr.all_managed_windows();
    for id in managed {
        let hwnd = id_to_hwnd(id);
        if !is_valid_window(id) {
            if st.mgr.remove_window(id) {
                st.applier.forget(id);
                changed = true;
            }
            continue;
        }
        if let Some(win_did) = st.vdesktop.window_desktop_id(hwnd) {
            if st.mgr.move_window_to_os_desktop(id, &win_did) {
                changed = true;
            }
        }
    }

    for mut info in enumerate_manageable_windows(st.own_pid) {
        info.desktop_id = st.vdesktop.window_desktop_id(id_to_hwnd(info.id));
        if !st.mgr.is_managed(info.id) && st.mgr.add_window(info) {
            changed = true;
        }
    }

    if changed {
        apply_frame_now(st, ctrl_hwnd, true);
        refresh_overview_if_open(st);
        schedule_state_save(st, ctrl_hwnd);
    }
}

fn apply_frame_now(st: &mut AppState, _ctrl_hwnd: HWND, reset_clock: bool) {
    if reset_clock {
        st.last_tick = Instant::now();
    }
    apply_frame_step(st, 0.016);
}

fn apply_frame_step(st: &mut AppState, dt_secs: f64) {
    let plan = st.mgr.compute_frame(dt_secs);
    let border_colors = if st.mgr.config.appearance.enable_win11_border {
        let active = parse_hex_colorref(&st.mgr.config.appearance.active_border_color)
            .unwrap_or(0x00F7_A27A);
        let inactive = parse_hex_colorref(&st.mgr.config.appearance.inactive_border_color)
            .unwrap_or(0x004D_332F);
        Some((active, inactive))
    } else {
        None
    };

    let cloak_changes = st.applier.apply(&plan.commands, border_colors);
    for (win_id, cloaked) in cloak_changes {
        st.mgr.set_cloaked_state(win_id, cloaked);
    }

    st.pacer.set_active(plan.still_animating);
}

fn drain_ipc_requests(st: &mut AppState, ctrl_hwnd: HWND) {
    while let Ok(pending) = st.ipc_rx.try_recv() {
        let resp = match pending.req {
            IpcRequest::Ping => IpcResponse::Pong,
            IpcRequest::GetState => IpcResponse::State {
                state: st.mgr.snapshot(),
            },
            IpcRequest::Run { action } => {
                execute_action(st, ctrl_hwnd, action);
                IpcResponse::Ok { message: None }
            }
        };
        let _ = pending.reply_tx.send(resp);
    }
}

