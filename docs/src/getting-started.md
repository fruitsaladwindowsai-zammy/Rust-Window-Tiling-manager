# Getting Started

## Prerequisites

- **Windows 10 or Windows 11** (64-bit)
- **Rust toolchain 1.85+** (workspace uses Rust Edition 2024)
- **Visual Studio Build Tools** (Desktop development with C++ / Windows SDK)

## Build & Install

```powershell
# Build the release binary
cargo build --release

# Create default configuration at %APPDATA%\WinScroller2D\winscroller2d.toml
.\target\release\winscroller2d.exe --init-config

# Validate config syntax & ranges
.\target\release\winscroller2d.exe --validate

# Run WinScroller 2D
.\target\release\winscroller2d.exe
```

> **Elevated windows (UAC):** If you need to tile or trigger hotkeys while an Administrator window is focused, run `winscroller2d.exe` elevated as well.

## First Steps

1. Press **`Win+Left` / `Win+Right` / `Win+Up` / `Win+Down`** to move focus across the 2D grid.
2. Press **`Win+Shift+Left/Right/Up/Down`** to move or swap windows between coordinates.
3. Press **`Win+O`** to open the dynamic overview:
   - Hover a card to highlight it and reveal the red **`✕`** close button in the top-right corner.
   - Click a card to animate it to the front and exit overview.
   - Drag and drop cards to rearrange cells (hold **`Shift`** while dropping to stack).
4. Press **`Win+Alt+S`** (or double/right-click the system tray icon) to tweak layout, animation, hover, and overview settings live.
5. Press **`Win+Shift+Escape`** to cleanly exit and restore all managed windows.

## CLI & IPC Quick Reference

```powershell
winscroller2d --help              # Show CLI flags
winscroller2d --config custom.toml
winscroller2d msg state           # Print JSON state snapshot of running daemon
winscroller2d msg overview        # Toggle Win+O overview via IPC
winscroller2d msg quit            # Exit running daemon cleanly
```
