# Getting Started

## Prerequisites

- **Windows 10 or Windows 11** (64-bit)
- **Git for Windows** (Required for `git clone`)
- **Rust toolchain 1.85+** (Rust Edition 2024)
- **Visual Studio Build Tools** (Desktop development with C++ / Windows SDK)

On a clean machine, install prerequisites in an Administrator PowerShell prompt:

```powershell
# 1. Install Git for Windows
winget install --id Git.Git --accept-package-agreements --accept-source-agreements

# 2. Install Rustup
winget install --id Rustlang.Rustup --accept-package-agreements --accept-source-agreements

# 3. Install MSVC Build Tools
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```
*Restart your PowerShell terminal after installation so `git`, `cargo`, and `rustc` are on your PATH.*

---

## Downloading the Source Code

### Option A: Clone with Git (Recommended)
```powershell
git clone https://github.com/fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager.git
cd "Rust-Window-Tiling-manager"
```

*If the repository requires GitHub authentication:*
```powershell
# With GitHub CLI:
winget install --id GitHub.cli
gh auth login
gh repo clone fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager
cd "Rust-Window-Tiling-manager"

# Or with SSH:
git clone git@github.com:fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager.git
cd "Rust-Window-Tiling-manager"
```

### Option B: Download ZIP (No Git Required)
```powershell
Invoke-WebRequest -Uri "https://github.com/fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager/archive/refs/heads/main.zip" -OutFile "$HOME\Downloads\winscroller.zip"
Expand-Archive -Path "$HOME\Downloads\winscroller.zip" -DestinationPath "$HOME\Projects"
cd "$HOME\Projects\Rust-Window-Tiling-manager-main"
```

---

## Build & First-Time Setup

```powershell
# 1. Build release binary
cargo build --release

# 2. Create default configuration at %APPDATA%\WinScroller2D\winscroller2d.toml
.\target\release\winscroller2d.exe --init-config

# 3. Validate configuration syntax & ranges
.\target\release\winscroller2d.exe --validate

# 4. Run WinScroller 2D
.\target\release\winscroller2d.exe
```

> [!NOTE]
> **Automatic Administrator Elevation**:
> WinScroller 2D automatically requests Administrator elevation via Windows UAC if started non-elevated. Click **Yes** on the UAC prompt. This gives WinScroller 2D High Integrity Level so its low-level keyboard hooks (`WH_KEYBOARD_LL`) can intercept shortcuts across **all** windows (including Administrator PowerShell and Task Manager) without being blocked by Windows UIPI.

### Running on Windows Startup
To have WinScroller 2D launch automatically when you sign in:
```powershell
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\WinScroller2D.lnk")
$Shortcut.TargetPath = "$PWD\target\release\winscroller2d.exe"
$Shortcut.WorkingDirectory = "$PWD"
$Shortcut.Save()
```

---

## Updating WinScroller 2D

When updating, the active `.exe` file is locked by Windows. You must exit the running instance before compiling:

### Step 1: Gracefully Exit the Active Instance
* Press **`Win + Shift + Escape`**
* OR execute via terminal:
  ```powershell
  .\target\release\winscroller2d.exe msg quit
  ```

### Step 2: Pull Latest Changes
```powershell
git pull origin main
```

### Step 3: Recompile Release Binary
```powershell
cargo build --release
```

### Step 4: Relaunch with Replacement
```powershell
.\target\release\winscroller2d.exe --replace
```

### Step 5: (Optional) Retile Workspaces
If updating between releases or if windows appear misaligned:
```powershell
.\target\release\winscroller2d.exe msg retile
```

---

## First Steps

1. Press **`Win+Left` / `Win+Right` / `Win+Up` / `Win+Down`** to move focus across the 2D grid.
2. Press **`Win+Shift+Left/Right/Up/Down`** to move or swap windows between coordinates.
3. Press **`Win+O`** to open the dynamic overview:
   - Hover a card to highlight it and reveal the red **`✕`** close button in the top-right corner.
   - Click a card to animate it to the front and exit overview.
   - Drag and drop cards to rearrange cells (drag onto another card to snap horizontally side-by-side).
4. Press **`Win+Alt+S`** (or double/right-click the system tray icon) to tweak layout, animation, hover, and overview settings live.
5. Press **`Win+Shift+Escape`** to cleanly exit and restore all managed windows.

---

## CLI & IPC Quick Reference

```powershell
winscroller2d --help              # Show CLI flags
winscroller2d --replace           # Replace running instance if active
winscroller2d --config custom.toml
winscroller2d msg state           # Print JSON state snapshot of running daemon
winscroller2d msg overview        # Toggle Win+O overview via IPC
winscroller2d msg retile          # Reset all workspaces to standard clean geometry
winscroller2d msg quit            # Exit running daemon cleanly
```
