use crate::app;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use windows_sys::Win32::Foundation::BOOL;
use windows_sys::Win32::Graphics::Dwm::DwmIsCompositionEnabled;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const PREREQ_MARKER_FILE: &str = ".prereqs_v1_ok";
const VCREDIST_WINGET_ID: &str = "Microsoft.VCRedist.2015+.x64";
const VCREDIST_DIRECT_URL: &str = "https://aka.ms/vs/17/release/vc_redist.x64.exe";

/// Automatically verifies and installs required runtime prerequisites before daemon startup.
/// Uses a lightweight marker file in the config directory so subsequent launches start immediately.
pub fn ensure_runtime_prerequisites(config_path: &Path, force_check: bool) -> Result<(), String> {
    let marker = marker_path(config_path);
    let config_ready = config_path.exists();
    if !force_check && config_ready && marker.exists() && is_vcruntime_present() {
        return Ok(());
    }

    println!("[winscroller2d] checking runtime prerequisites...");

    // 1. Desktop Window Manager (DWM) composition check
    if !is_dwm_enabled() {
        eprintln!(
            "[winscroller2d] warning: Windows Desktop Window Manager (DWM) composition appears disabled; live overview thumbnails and cloaking require DWM."
        );
    }

    // 2. Microsoft Visual C++ 2015-2022 x64 Runtime check + automatic silent installation
    if !is_vcruntime_present() {
        println!(
            "[winscroller2d] Microsoft Visual C++ x64 Runtime (vcruntime140.dll) not found; installing automatically..."
        );
        if let Err(err) = install_vcredist() {
            eprintln!(
                "[winscroller2d] warning: automatic VC++ Runtime installation did not complete ({err}); continuing with statically linked runtime."
            );
        } else {
            println!("[winscroller2d] Visual C++ x64 Runtime installed successfully.");
        }
    }

    // 3. Ensure %APPDATA%\WinScroller2D\ and default winscroller2d.toml exist
    let _ = app::ensure_config_exists(config_path)?;

    // Record marker so future startups are instantaneous
    if let Some(parent) = marker.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let stamp = format!("winscroller2d {}\n", env!("CARGO_PKG_VERSION"));
    let _ = std::fs::write(&marker, stamp);

    println!("[winscroller2d] all runtime prerequisites verified.");
    Ok(())
}

/// Runs a full automated prerequisite check and installs missing runtime and optional
/// source-build prerequisites (`Microsoft.VCRedist.2015+.x64`, `Git.Git`, `Rustlang.Rustup`).
pub fn run_full_setup(config_path: &Path) -> Result<(), String> {
    println!("=== WinScroller 2D Automated Prerequisite Setup ===");
    ensure_runtime_prerequisites(config_path, true)?;

    let has_winget = command_exists("winget");
    println!(
        "  [{}] winget package manager",
        if has_winget { "OK" } else { "MISSING" }
    );
    println!(
        "  [{}] Desktop Window Manager (DWM)",
        if is_dwm_enabled() { "OK" } else { "WARN" }
    );
    println!(
        "  [{}] Visual C++ x64 Runtime (vcruntime140.dll)",
        if is_vcruntime_present() {
            "OK"
        } else {
            "MISSING"
        }
    );
    println!(
        "  [OK] Configuration initialized at {}",
        config_path.display()
    );

    // Optional build / update tools (installed via winget when available)
    let dev_tools = [
        ("git", "Git.Git", "Git for Windows"),
        ("cargo", "Rustlang.Rustup", "Rust toolchain (Rustup)"),
    ];
    for (bin, winget_id, label) in dev_tools {
        if command_exists(bin) {
            println!("  [OK] {label} (`{bin}` found on PATH)");
        } else if has_winget {
            println!("  [..] Installing {label} ({winget_id}) via winget...");
            match winget_install(winget_id) {
                Ok(()) => println!("  [OK] {label} installed (reopen terminal to refresh PATH)"),
                Err(err) => eprintln!("  [WARN] Could not install {label}: {err}"),
            }
        } else {
            println!(
                "  [INFO] {label} not found and `winget` is unavailable; skipping optional dev tool."
            );
        }
    }

    println!("=== Setup Complete ===");
    Ok(())
}

fn marker_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(PREREQ_MARKER_FILE)
}

fn is_dwm_enabled() -> bool {
    let mut enabled: BOOL = 0;
    let hr = unsafe { DwmIsCompositionEnabled(&mut enabled) };
    hr == 0 && enabled != 0
}

fn is_vcruntime_present() -> bool {
    let sys_root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".to_string());
    vcruntime_exists_under(Path::new(&sys_root))
}

fn vcruntime_exists_under(system_root: &Path) -> bool {
    system_root
        .join("System32")
        .join("vcruntime140.dll")
        .exists()
}

fn command_exists(cmd: &str) -> bool {
    Command::new("where")
        .arg(cmd)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn winget_install(package_id: &str) -> Result<(), String> {
    let status = Command::new("winget")
        .args([
            "install",
            "--id",
            package_id,
            "--silent",
            "--accept-package-agreements",
            "--accept-source-agreements",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("failed to launch winget: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("winget exited with status {status}"))
    }
}

fn install_vcredist() -> Result<(), String> {
    if command_exists("winget") && winget_install(VCREDIST_WINGET_ID).is_ok() {
        return Ok(());
    }

    // Direct fallback using PowerShell to download Microsoft's official vc_redist.x64.exe
    let installer = std::env::temp_dir().join("winscroller2d_vc_redist.x64.exe");
    let dl_script = format!(
        "Invoke-WebRequest -UseBasicParsing -Uri '{}' -OutFile '{}'",
        VCREDIST_DIRECT_URL,
        installer.display()
    );
    let dl_status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &dl_script,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("failed to invoke PowerShell download: {e}"))?;
    if !dl_status.success() {
        return Err(format!("PowerShell download failed ({dl_status})"));
    }

    let inst_status = Command::new(&installer)
        .args(["/install", "/quiet", "/norestart"])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("failed to execute vc_redist.x64.exe: {e}"))?;
    let _ = std::fs::remove_file(&installer);

    // 0 = success, 3010 = success (reboot recommended), 1638 = newer version already installed
    match inst_status.code() {
        Some(0 | 3010 | 1638) => Ok(()),
        other => Err(format!("vc_redist.x64.exe exited with code {other:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prereq_marker_path_resolves_inside_config_dir() {
        let cfg = Path::new(r"C:\Users\Test\AppData\Roaming\WinScroller2D\winscroller2d.toml");
        let marker = marker_path(cfg);
        assert_eq!(
            marker,
            Path::new(r"C:\Users\Test\AppData\Roaming\WinScroller2D\.prereqs_v1_ok")
        );
    }

    #[test]
    fn vcruntime_detection_handles_missing_root() {
        assert!(!vcruntime_exists_under(Path::new(
            r"C:\NonExistent_WinScroller_Dir_987654"
        )));
    }

    #[test]
    fn command_exists_finds_cmd_and_rejects_nonexistent() {
        assert!(command_exists("cmd"));
        assert!(!command_exists("winscroller2d_nonexistent_tool_xyz"));
    }
}
