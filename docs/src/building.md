# Building from Source

## Prerequisites

| Requirement | Version |
|-------------|---------|
| Windows | Windows 10 or Windows 11 (64-bit) |
| Rust toolchain | Stable **1.85+** (Rust Edition 2024) |
| Visual Studio Build Tools | 2019+ with **Desktop development with C++** / Windows SDK |

## Build & Test Commands

```powershell
# Debug build
cargo build

# Release build (recommended for daily use)
cargo build --release

# Run all workspace unit & layout tests
cargo test --workspace

# Run release binary
.\target\release\winscroller2d.exe
```

## Troubleshooting

### "Access is denied" (OS error 5) when rebuilding `--release`

If `winscroller2d.exe` is currently running, Windows locks the executable file. Exit cleanly via **`Win+Shift+Escape`** (or `.\target\release\winscroller2d.exe msg quit`), then rebuild:

```powershell
cargo build --release
```

### Target Platform Support

Because `winscroller2d` uses `windows-sys` Win32, DWM, COM, and HID APIs directly, builds must run on Windows (or target `x86_64-pc-windows-msvc` with Windows SDK libraries).
