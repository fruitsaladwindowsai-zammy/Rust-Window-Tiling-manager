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

If you are setting up a fresh machine, install the Rust toolchain and Visual Studio C++ Build Tools:

```powershell
# Install Rustup
winget install --id Rustlang.Rustup --accept-package-agreements --accept-source-agreements

# Install MSVC Build Tools (Required for linking native Windows APIs)
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

Restart your terminal after installation so that `cargo` and `rustc` are on your system `PATH`.

---

## 📥 Cloning and Building

Clone the repository and build the release binary:

```powershell
git clone https://github.com/fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager.git
cd "Rust-Window-Tiling-manager"

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

---

## 🖥️ User Interface Overview

* **System Tray Icon**: A scroll/grid icon appears in the Windows notification area (taskbar tray).
  * **Right Click**: Provides a context menu to open Settings, pause/resume tiling, reload configuration, or exit cleanly.
* **Live Graphical Settings**: Press **`Win + Alt + S`** at any time to open the live settings GUI to configure gaps, spring speeds, borders, and blur modes in real time.
* **Graceful Exit**: Press **`Win + Shift + Escape`** or select **Exit** from the tray menu. WinScroller 2D guarantees that all managed windows are immediately uncloaked, restored, and centered on their respective displays.
