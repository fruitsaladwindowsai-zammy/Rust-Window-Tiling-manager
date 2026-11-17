# Precision Touchpad Gestures

WinScroller 2D natively integrates with **Windows Precision Touchpad (PTP)** devices using low-level Raw Input (`WM_INPUT`) and Human Interface Device (HID) touch contact tracking.

---

## 🖐️ Gesture Actions

### 1. Three-Finger 2D Smooth Panning
* **Action**: Place three fingers on your touchpad and drag horizontally or vertically.
* **Behavior**:
  * **1:1 Pixel Tracking**: The 2D workspace moves underneath your fingers in real time with zero latency.
  * **Continuous Directional Flow**: Pan diagonally or freely in 2D space across columns and rows.
  * **Kinetic Inertia (Flick)**: When you lift your fingers with momentum, WinScroller 2D continues scrolling with natural exponential friction decay until it smoothly settles.
  * **Focus Snapping**: When the pan momentum ends, focus automatically settles onto the nearest central cell.

### 2. Four-Finger Swipe for Overview
* **Action**: Swipe up with four fingers on your touchpad.
* **Behavior**: Instantly opens the **`Win + O` Overview** mode. Swiping down closes Overview and zooms back into your active window.

---

## ⚙️ Configuration Settings

Customize touchpad responsiveness under `[layout]` in `%APPDATA%\WinScroller2D\winscroller2d.toml`:

```toml
[layout]
# Enable three-finger smooth 2D panning
three_finger_pan = true

# Enable four-finger swipe up/down to open/close Overview
four_finger_overview = true

# Pan sensitivity multiplier (clamped: 0.2..=5.0, default: 1.0)
gesture_pan_sensitivity = 1.0

# Invert pan direction (natural vs. reverse scrolling)
gesture_invert_x = false
gesture_invert_y = false
```

---

## 🔍 Verifying Touchpad Compatibility

To check if your device supports Windows Precision Touchpad:
1. Open Windows **Settings** (`Win + I`).
2. Navigate to **Bluetooth & devices** > **Touchpad**.
3. If your PC supports precision touch, you will see the banner:
   *"Your PC has a precision touchpad."*
4. Ensure three-finger and four-finger gestures are set to default in Windows Settings so they don't conflict with WinScroller 2D.
