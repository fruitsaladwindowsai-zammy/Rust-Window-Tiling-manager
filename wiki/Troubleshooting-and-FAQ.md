# Troubleshooting & FAQ

Frequently asked questions and solutions for common issues when running WinScroller 2D.

---

## ❓ Frequently Asked Questions

### Q: Why do Administrator windows (e.g. PowerShell or Task Manager) snap into corners when I press `Win + Arrows`?
**A**: This is caused by Windows **User Interface Privilege Isolation (UIPI)**. If WinScroller 2D is run without Administrator rights, Windows blocks its keyboard hook from intercepting keystrokes on elevated windows, allowing Windows Explorer to catch `Win + Arrows` and Aero-Snap the window.

**Solution**:
Launch WinScroller 2D with the built-in auto-elevator:
```powershell
target\release\winscroller2d.exe --replace
```
When prompted by Windows UAC, click **Yes** to allow Administrator elevation.

---

### Q: WinScroller 2D reports: `WinScroller 2D is already running`. How do I restart it?
**A**: Use the `--replace` (`-r`) flag:
```powershell
target\release\winscroller2d.exe --replace
```
Or gracefully stop the running instance via IPC:
```powershell
target\release\winscroller2d.exe msg quit
```

---

### Q: A window was resized by an installer or game and looks squished. How do I reset it?
**A**: Press **`Win + Shift + R`** or run:
```powershell
winscroller2d.exe msg retile
```
This immediately clears any custom size overrides and returns all windows across your workspaces to standard full-height presets.

---

### Q: How do I completely clear saved window layout memory?
**A**: WinScroller 2D stores its persistent lattice layout in:
```text
%APPDATA%\WinScroller2D\state.json
```
To start completely fresh:
1. Quit WinScroller 2D (`Win + Shift + Escape`).
2. Delete the file:
   ```powershell
   Remove-Item "$env:APPDATA\WinScroller2D\state.json" -Force
   ```
3. Restart `winscroller2d.exe`.

---

### Q: Where are crash and panic logs located?
**A**: If WinScroller 2D encounters an unexpected panic, full backtraces and diagnostics are written to:
```text
%APPDATA%\WinScroller2D\crash.log
```

---

### Q: How do I temporarily disable tiling for gaming or screen sharing?
**A**: Press **`Win + Shift + P`** to pause window tiling. All windows will remain in place and can be moved freely with the mouse. Press **`Win + Shift + P`** again to resume tiling.
