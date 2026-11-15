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

Customize touchpad responsiveness in `%APPDATA%\WinScroller2D\winscroller2d.toml`:

```toml
[touchpad]
# Enable three-finger smooth panning
enabled = true

# Enable four-finger swipe to toggle Overview
swipe_overview = true

# Pan sensitivity multiplier (default: 1.0)
sensitivity = 1.0

# Kinetic friction coefficient (default: 0.92, lower = more friction / stops faster)
friction = 0.92

# Velocity threshold required to trigger inertia flick (in pixels/sec)
flick_threshold = 300.0

# Invert scroll direction (natural scrolling)
invert_x = false
invert_y = false
```

---

## 🔍 Verifying Touchpad Compatibility

To check if your device supports Windows Precision Touchpad:
1. Open Windows **Settings** (`Win + I`).
2. Navigate to **Bluetooth & devices** > **Touchpad**.
3. If your PC supports precision touch, you will see the banner:
   *"Your PC has a precision touchpad."*
4. Ensure three-finger and four-finger gestures are set to default in Windows Settings so they don't conflict with WinScroller 2D.
