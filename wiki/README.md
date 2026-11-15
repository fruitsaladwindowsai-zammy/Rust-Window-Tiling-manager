# WinScroller 2D Wiki Repository

This directory contains the official wiki for **WinScroller 2D**. It is formatted for direct display in GitHub and is fully compatible with GitHub Wiki (`.wiki.git`).

---

## 📚 Table of Contents

* [Home](Home.md) — Welcome & Overview
* [Getting Started](Getting-Started.md) — Installation, building & running
* [Core Concepts](Core-Concepts.md) — 2D Infinite Sparse Lattice, Virtual Camera & Stacking
* [Keyboard Shortcuts](Keyboard-Shortcuts.md) — Complete keybinding reference
* [Touchpad Gestures](Touchpad-Gestures.md) — Precision Touchpad 3-finger and 4-finger gestures
* [Overview Mode (Win+O)](Overview-Mode.md) — Live 2D overview, acrylic blur, drag-to-snap
* [Multi-Monitor & Virtual Desktops](Multi-Monitor-and-Virtual-Desktops.md) — Extended displays & COM virtual desktops
* [Administrator & Elevated Apps](Administrator-and-Elevated-Apps.md) — UIPI privilege model & auto-elevation
* [Configuration Guide](Configuration-Guide.md) — `winscroller2d.toml` options and schema
* [CLI & IPC Scripting](CLI-and-IPC-Scripting.md) — `winscroller2d msg` commands and pipe IPC
* [Window Rules & Exclusions](Window-Rules-and-Exclusions.md) — Application filtering & floating rules
* [Architecture & Internals](Architecture-and-Internals.md) — Two-tier monorepo architecture
* [Troubleshooting & FAQ](Troubleshooting-and-FAQ.md) — Solutions to common issues

---

## 🚀 How to Publish to GitHub Wiki

GitHub wikis live in a separate git repository linked to your main repository (`<repo>.wiki.git`).

To publish these pages directly to your GitHub repository's Wiki tab:

```powershell
# 1. Initialize a git repository inside the wiki directory
cd wiki
git init
git checkout -b master

# 2. Add all wiki markdown files
git add .
git commit -m "Initial WinScroller 2D Wiki documentation"

# 3. Add your GitHub Wiki remote and push
git remote add origin https://github.com/fruitsaladwindowsai-zammy/Rust-Window-Tiling-manager.wiki.git
git push -u origin master --force
```

*(Note: Enable the **Wiki** feature in your repository settings on GitHub under **Settings** > **Features** > **Wikis** before pushing.)*
