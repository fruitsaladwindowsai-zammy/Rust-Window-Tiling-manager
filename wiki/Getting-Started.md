# Getting Started with WinScroller 2D

This guide walks you through installing, building, and running **WinScroller 2D** on Windows 10 or 11.

---

## 📋 System Requirements

* **Operating System**: Windows 10 (Build 19044+) or Windows 11 (22H2 / 23H2 / 24H2 recommended).
* **Display Configuration**: Single or multi-monitor (extended display mode).
* **Architecture**: x86_64 (AMD64 / Intel 64-bit).
* **Permissions**: Administrator rights (required to manage elevated windows like PowerShell and Task Manager via Windows UIPI).

---

## 🛠️ Prerequisites

On a clean Windows machine, open an **Administrator PowerShell** prompt to install the required toolchains:

```powershell
# 1. Install Git for Windows (Enables 'git clone')
winget install --id Git.Git --accept-package-agreements --accept-source-agreements

# 2. Install Rustup
winget install --id Rustlang.Rustup --accept-package-agreements --accept-source-agreements

# 3. Install MSVC Build Tools (Required for linking native Windows APIs)
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

> [!IMPORTANT]
> **Restart Your Terminal**: After running the commands above, close and reopen your PowerShell terminal so that `git`, `cargo`, and `rustc` are available on your system `PATH`.
> *(Optional: Enable long path support in Git: `git config --global core.longpaths true`)*

---

## 📥 Downloading the Source Code

### Option A: Clone with Git (Recommended)
```powershell
git clone https://github.com/fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager.git
cd "Rust-Window-Tiling-manager"
```

*If the repository requires GitHub authentication, use GitHub CLI or SSH:*
```powershell
# Using GitHub CLI:
winget install --id GitHub.cli
gh auth login
gh repo clone fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager
cd "Rust-Window-Tiling-manager"

# Or using SSH:
git clone git@github.com:fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager.git
cd "Rust-Window-Tiling-manager"
```

### Option B: Download ZIP (No Git Required)
If you do not have Git installed or are having issues with `git clone`, download and extract directly in PowerShell:
```powershell
Invoke-WebRequest -Uri "https://github.com/fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager/archive/refs/heads/main.zip" -OutFile "$HOME\Downloads\winscroller.zip"
Expand-Archive -Path "$HOME\Downloads\winscroller.zip" -DestinationPath "$HOME\Projects"
cd "$HOME\Projects\Rust-Window-Tiling-manager-main"
```

---

## 🔨 Building the Release Binary

```powershell
# Compile with release optimizations
cargo build --release
```

The compiled binary will be placed at `target\release\winscroller2d.exe`.

---

## 🚀 First Launch & Initial Configuration

### 1. Initialize Configuration
Generate the default configuration file:

```powershell
.\target\release\winscroller2d.exe --init-config
```

This creates the default configuration at:
```text
%APPDATA%\WinScroller2D\winscroller2d.toml
```

### 2. Start the Window Manager
Run the executable:

```powershell
.\target\release\winscroller2d.exe
```

> [!NOTE]
> **Automatic Administrator Elevation**:
> When started, WinScroller 2D automatically requests Administrator elevation via Windows UAC if not already running in an elevated prompt. This is **essential** so that `WH_KEYBOARD_LL` low-level keyboard hooks can seamlessly navigate across elevated windows (like Administrator PowerShell and Task Manager) without being blocked by Windows UIPI (User Interface Privilege Isolation).

### 3. Replacing or Restarting an Existing Instance
If WinScroller 2D is already running in the background, you can cleanly replace it using the `--replace` (`-r`) flag:

```powershell
.\target\release\winscroller2d.exe --replace
```

This sends an IPC quit command to the existing daemon, waits for all windows to reconcile, and starts the new daemon cleanly without port or pipe conflicts.

### 4. Running Automatically on Windows Startup
To launch WinScroller 2D automatically when you log into Windows:

```powershell
# Create a shortcut in Windows Startup folder (shell:startup)
$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Startup\WinScroller2D.lnk")
$Shortcut.TargetPath = "$PWD\target\release\winscroller2d.exe"
$Shortcut.WorkingDirectory = "$PWD"
$Shortcut.Save()
```

---

## 🔄 Updating WinScroller 2D

When a new version is released, Windows file locking prevents `cargo build` from replacing an active `.exe` file (`Access is denied (os error 5)`). Follow this safe update workflow:

### Step 1: Gracefully Exit the Running Instance
* Press **`Win + Shift + Escape`**
* OR execute via terminal:
  ```powershell
  .\target\release\winscroller2d.exe msg quit
  ```

### Step 2: Fetch and Pull Latest Changes
```powershell
git pull origin main
```

### Step 3: Recompile the Release Binary
```powershell
cargo build --release
```
*(Optional: Run `cargo test --workspace` to verify all tests pass).*

### Step 4: Relaunch with Automatic Replacement
```powershell
.\target\release\winscroller2d.exe --replace
```

### Step 5: Retile Workspaces (If Migrating)
If the update includes layout changes or if windows appear misaligned:
```powershell
.\target\release\winscroller2d.exe msg retile
```

---

## 🖥️ User Interface Overview

* **System Tray Icon**: A scroll/grid icon appears in the Windows notification area (taskbar tray).
  * **Right Click**: Provides a context menu to open Settings, pause/resume tiling, reload configuration, or exit cleanly.
* **Live Graphical Settings**: Press **`Win + Alt + S`** at any time to open the live settings GUI to configure gaps, spring speeds, borders, and blur modes in real time.
* **Graceful Exit**: Press **`Win + Shift + Escape`** or select **Exit** from the tray menu. WinScroller 2D guarantees that all managed windows are immediately uncloaked, restored, and centered on their respective displays.
