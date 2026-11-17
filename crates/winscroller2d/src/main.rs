mod app;
mod ipc_pipe;
mod win32;

use std::path::PathBuf;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Security::{
    GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
};
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetExitCodeProcess, INFINITE, OpenProcessToken, WaitForSingleObject,
};
use windows_sys::Win32::UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOW;
use winscroller2d_core::{Action, Direction, IpcRequest, IpcResponse};

fn main() {
    install_panic_hook();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(err) = run_cli(&args) {
        eprintln!("winscroller2d error: {err}");
        std::process::exit(1);
    }
}

fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let log_path = app::default_config_path()
            .parent()
            .map(|p| p.join("crash.log"))
            .unwrap_or_else(|| PathBuf::from("winscroller2d-crash.log"));
        if let Some(parent) = log_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let bt = std::backtrace::Backtrace::force_capture();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let msg = format!("[unix:{ts}] WinScroller 2D panic: {info}\nBacktrace:\n{bt}\n\n");
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, msg.as_bytes()));
        eprintln!("{msg}");
        default_hook(info);
    }));
}

fn run_cli(args: &[String]) -> Result<(), String> {
    let mut config_override: Option<PathBuf> = None;
    let mut mode_validate = false;
    let mut mode_init = false;
    let mut mode_replace = false;
    let mut no_elevate = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" | "help" => {
                print_help();
                return Ok(());
            }
            "-V" | "--version" => {
                println!(
                    "WinScroller 2D (winscroller2d) {}",
                    env!("CARGO_PKG_VERSION")
                );
                return Ok(());
            }
            "-c" | "--config" => {
                i += 1;
                let path = args.get(i).ok_or("--config requires a path")?;
                config_override = Some(PathBuf::from(path));
            }
            "-r" | "--replace" => {
                mode_replace = true;
            }
            "--no-elevate" | "--non-elevated" => {
                no_elevate = true;
            }
            "--validate" => {
                mode_validate = true;
            }
            "--init-config" => {
                mode_init = true;
            }
            "msg" => {
                return run_msg_command(&args[i + 1..]);
            }
            other => {
                return Err(format!("unknown argument `{other}` (try --help)"));
            }
        }
        i += 1;
    }

    let path = config_override.unwrap_or_else(app::default_config_path);
    if mode_init {
        let _ = app::ensure_config_exists(&path)?;
        println!("Config ready at {}", path.display());
        return Ok(());
    }
    if mode_validate {
        let _ = app::load_config_file(&path)?;
        println!("Config is valid: {}", path.display());
        return Ok(());
    }

    if matches!(
        ipc_pipe::send_ipc_request(&IpcRequest::Ping),
        Ok(IpcResponse::Pong)
    ) {
        if mode_replace {
            println!("[winscroller2d] existing instance detected, requesting graceful shutdown...");
            let _ = ipc_pipe::send_ipc_request(&IpcRequest::Run {
                action: Action::Quit,
            });
            let start = std::time::Instant::now();
            while start.elapsed() < std::time::Duration::from_secs(3) {
                std::thread::sleep(std::time::Duration::from_millis(150));
                if ipc_pipe::send_ipc_request(&IpcRequest::Ping).is_err() {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        } else {
            return Err("WinScroller 2D is already running (use `winscroller2d msg ...` to control it, or `winscroller2d --replace` to restart)".into());
        }
    }

    if !no_elevate && !is_elevated() {
        println!("[winscroller2d] elevating to administrator to manage all windows and hotkeys...");
        return relaunch_elevated(args);
    }

    app::run_daemon(path)
}

pub fn is_elevated() -> bool {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        let current_proc = GetCurrentProcess();
        if OpenProcessToken(current_proc, TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let mut elevation: TOKEN_ELEVATION = std::mem::zeroed();
        let mut ret_len: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_len,
        );
        CloseHandle(token);
        ok != 0 && elevation.TokenIsElevated != 0
    }
}

fn relaunch_elevated(args: &[String]) -> Result<(), String> {
    let exe =
        std::env::current_exe().map_err(|e| format!("cannot determine executable path: {e}"))?;
    let exe_w = win32::to_wide(exe.to_string_lossy().as_ref());
    let verb_w = win32::to_wide("runas");

    let mut param_str = String::new();
    for arg in args {
        if !param_str.is_empty() {
            param_str.push(' ');
        }
        if arg.contains(' ') || arg.contains('\t') || arg.is_empty() {
            param_str.push('"');
            param_str.push_str(&arg.replace('"', "\\\""));
            param_str.push('"');
        } else {
            param_str.push_str(arg);
        }
    }
    let params_w = win32::to_wide(&param_str);

    let mut sei: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    sei.fMask = SEE_MASK_NOCLOSEPROCESS;
    sei.lpVerb = verb_w.as_ptr();
    sei.lpFile = exe_w.as_ptr();
    sei.lpParameters = if param_str.is_empty() {
        std::ptr::null()
    } else {
        params_w.as_ptr()
    };
    sei.nShow = SW_SHOW;

    let ok = unsafe { ShellExecuteExW(&mut sei) };
    if ok == 0 {
        let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
        if err == 1223 {
            return Err("elevation request was cancelled by user (UAC prompt dismissed)".into());
        }
        return Err(format!(
            "failed to relaunch as administrator (Win32 error {err})"
        ));
    }

    if !sei.hProcess.is_null() {
        unsafe {
            WaitForSingleObject(sei.hProcess, INFINITE);
            let mut code: u32 = 0;
            GetExitCodeProcess(sei.hProcess, &mut code);
            CloseHandle(sei.hProcess);
            if code != 0 {
                std::process::exit(code as i32);
            }
        }
    }
    Ok(())
}

fn run_msg_command(args: &[String]) -> Result<(), String> {
    let Some(cmd) = args.first().map(|s| s.as_str()) else {
        print_msg_help();
        return Ok(());
    };

    let req = match cmd {
        "ping" => IpcRequest::Ping,
        "state" => IpcRequest::GetState,
        "focus" => {
            let dir = parse_dir(args.get(1))?;
            IpcRequest::Run {
                action: Action::Focus { direction: dir },
            }
        }
        "move" => {
            let dir = parse_dir(args.get(1))?;
            IpcRequest::Run {
                action: Action::MoveWindow { direction: dir },
            }
        }
        "pan" => {
            let dir = parse_dir(args.get(1))?;
            IpcRequest::Run {
                action: Action::PanCamera { direction: dir },
            }
        }
        "pan-pixels" => {
            let (dx, dy) = parse_f64_pair(&args[1..], "pan-pixels requires <dx> <dy>")?;
            IpcRequest::Run {
                action: Action::PanPixels { dx, dy },
            }
        }
        "pan-commit" => {
            let (vx, vy) = parse_f64_pair(&args[1..], "pan-commit requires <vx> <vy>")?;
            IpcRequest::Run {
                action: Action::PanCommit { vx, vy },
            }
        }
        "center" => IpcRequest::Run {
            action: Action::CenterCamera,
        },
        "width" => {
            let delta = parse_delta(args.get(1))?;
            IpcRequest::Run {
                action: Action::ResizeWidth { delta },
            }
        }
        "height" => {
            let delta = parse_delta(args.get(1))?;
            IpcRequest::Run {
                action: Action::ResizeHeight { delta },
            }
        }
        "consume" => {
            let dir = args.get(1).and_then(|s| Direction::parse(s));
            IpcRequest::Run {
                action: Action::ConsumeIntoCell { direction: dir },
            }
        }
        "expel" => {
            let dir = args.get(1).and_then(|s| Direction::parse(s));
            IpcRequest::Run {
                action: Action::ExpelFromCell { direction: dir },
            }
        }
        "tabbed" | "toggle-stack" => IpcRequest::Run {
            action: Action::ToggleCellStackMode,
        },
        "maximize" => IpcRequest::Run {
            action: Action::ToggleMaximizeCell,
        },
        "fullscreen" => IpcRequest::Run {
            action: Action::ToggleFullscreen,
        },
        "float" => IpcRequest::Run {
            action: Action::ToggleFloating,
        },
        "overview" => IpcRequest::Run {
            action: Action::ToggleOverview,
        },
        "close" => IpcRequest::Run {
            action: Action::CloseFocused,
        },
        "workspace" => {
            let one_based: usize = args
                .get(1)
                .ok_or("workspace requires a 1-based number")?
                .parse()
                .map_err(|_| "invalid workspace number")?;
            IpcRequest::Run {
                action: Action::SwitchWorkspace {
                    index: one_based.saturating_sub(1),
                },
            }
        }
        "move-to-workspace" => {
            let one_based: usize = args
                .get(1)
                .ok_or("move-to-workspace requires a 1-based number")?
                .parse()
                .map_err(|_| "invalid workspace number")?;
            IpcRequest::Run {
                action: Action::MoveToWorkspace {
                    index: one_based.saturating_sub(1),
                },
            }
        }
        "focus-monitor" => {
            let dir = parse_dir(args.get(1))?;
            IpcRequest::Run {
                action: Action::FocusMonitor { direction: dir },
            }
        }
        "move-to-monitor" => {
            let dir = parse_dir(args.get(1))?;
            IpcRequest::Run {
                action: Action::MoveToMonitor { direction: dir },
            }
        }
        "retile" => IpcRequest::Run {
            action: Action::Retile,
        },
        "pause" => IpcRequest::Run {
            action: Action::TogglePause,
        },
        "layout-toggle" => IpcRequest::Run {
            action: Action::ToggleWorkspaceLayout,
        },
        "settings" => IpcRequest::Run {
            action: Action::OpenSettings,
        },
        "reload" => IpcRequest::Run {
            action: Action::ReloadConfig,
        },
        "quit" => IpcRequest::Run {
            action: Action::Quit,
        },
        "help" | "-h" | "--help" => {
            print_msg_help();
            return Ok(());
        }
        other => return Err(format!("unknown msg command `{other}`")),
    };

    let resp = ipc_pipe::send_ipc_request(&req)?;
    match resp {
        IpcResponse::Ok { message } => {
            if let Some(m) = message {
                println!("{m}");
            } else {
                println!("ok");
            }
        }
        IpcResponse::Pong => println!("pong"),
        IpcResponse::State { state } => {
            println!("{}", serde_json::to_string_pretty(&state).unwrap());
        }
        IpcResponse::Error { message } => return Err(message),
    }
    Ok(())
}

fn parse_dir(arg: Option<&String>) -> Result<Direction, String> {
    let raw = arg.ok_or("direction required (left|right|up|down)")?;
    Direction::parse(raw).ok_or_else(|| format!("invalid direction `{raw}`"))
}

fn parse_f64_pair(args: &[String], usage: &str) -> Result<(f64, f64), String> {
    let a: f64 = args
        .first()
        .ok_or(usage)?
        .parse()
        .map_err(|_| usage.to_string())?;
    let b: f64 = args
        .get(1)
        .ok_or(usage)?
        .parse()
        .map_err(|_| usage.to_string())?;
    Ok((a, b))
}

fn parse_delta(arg: Option<&String>) -> Result<i32, String> {
    let raw = arg.ok_or("delta required (+1|-1|grow|shrink)")?;
    match raw.to_ascii_lowercase().as_str() {
        "grow" | "+" | "+1" | "1" => Ok(1),
        "shrink" | "-" | "-1" => Ok(-1),
        other => other
            .parse::<i32>()
            .map_err(|_| format!("invalid delta `{other}`")),
    }
}

fn print_help() {
    println!(
        "WinScroller 2D (winscroller2d) — Native Windows 11 2D Scrolling & Tiling Window Manager\n\n\
         USAGE:\n\
         \twinscroller2d [OPTIONS]\n\
         \twinscroller2d msg <COMMAND> [ARGS...]\n\n\
         OPTIONS:\n\
         \t-c, --config <PATH>   Use a custom winscroller2d.toml config path\n\
         \t-r, --replace         Replace running instance if already active\n\
         \t    --no-elevate      Run non-elevated (skip automatic Administrator elevation)\n\
         \t    --init-config     Create the default winscroller2d.toml and exit\n\
         \t    --validate        Validate config syntax and exit\n\
         \t-V, --version         Print version\n\
         \t-h, --help            Show help\n\n\
         DEFAULT CONFIG LOCATION:\n\
         \t%APPDATA%\\WinScroller2D\\winscroller2d.toml"
    );
}

fn print_msg_help() {
    println!(
        "winscroller2d msg commands:\n\
         \tping | state | retile | pause | layout-toggle | settings | reload | quit\n\
         \tfocus <left|right|up|down>\n\
         \tmove <left|right|up|down>\n\
         \tpan <left|right|up|down> | pan-pixels <dx> <dy> | pan-commit <vx> <vy> | center\n\
         \twidth <grow|shrink> | height <grow|shrink>\n\
         \tconsume [left|right|up|down] | expel [left|right|up|down] | tabbed\n\
         \tmaximize | fullscreen | float | overview | close\n\
         \tworkspace <1..N> | move-to-workspace <1..N>\n\
         \tfocus-monitor <left|right|up|down> | move-to-monitor <left|right|up|down>"
    );
}
