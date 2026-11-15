# Precision Touchpad Gestures

WinScroller 2D natively integrates with **Windows Precision Touchpad (PTP)** devices using low-level Raw Input (`WM_INPUT`) and Human Interface Device (HID) touch contact tracking.

---

## 🖐️ Gesture Actions

### 1. Three-Finger 2D Smooth Panning
* **Action**: Place three fingers on your touchpad and drag horizontally or vertically.
* **1:1 Pixel Tracking**: The 2D workspace moves underneath your fingers in real time with zero latency.
* **Kinetic Inertia (Flick)**: When you lift your fingers with momentum, WinScroller 2D continues scrolling with natural exponential friction decay until it smoothly settles.
* **Focus Snapping**: When the pan momentum ends, focus automatically settles onto the nearest central cell.

### 2. Four-Finger Swipe for Overview
* **Action**: Swipe up with four fingers on your touchpad to open **`Win + O` Overview**.
* Swiping down closes Overview and zooms back into your active window.
