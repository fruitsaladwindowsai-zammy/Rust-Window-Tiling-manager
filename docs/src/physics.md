# 120Hz Mass-Spring Physics & Camera Dynamics

WinScroller 2D rejects rigid, linear window cuts and jarring teleports. Every camera transition, window swap, and overview zoom is driven by a hardware-paced **underdamped mass-spring-damper harmonic oscillator**.

---

## ⚡ Mathematical Dynamics Architecture

![120Hz Mass-Spring Physics Architecture](../assets/concept-spring-physics.svg)

The camera motion in each coordinate axis ($x$ and $y$) obeys the second-order ordinary differential equation:

$$m \frac{d^2x}{dt^2} + c \frac{dx}{dt} + k(x - x_{\text{target}}) = 0$$

Where:
* $m$ = Virtual inertia / mass of the camera payload (normalized to $1.0$).
* $k$ = Spring stiffness constant (`stiffness`, default $k = 260.0$, clamped $40.0\text{..=}1500.0$).
* $\zeta$ = Damping ratio (`damping_ratio`, default $\zeta = 0.72$, clamped $0.45\text{..=}2.0$).
* $c = 2 \zeta \sqrt{k}$ = Derived damping coefficient ($\approx 23.22$ at defaults).
* $x_{\text{target}}$ = Target coordinate of the newly focused window cell or pan offset.

---

## 📈 Harmonic Response Characteristics

By tuning `stiffness` ($k$) and `damping_ratio` ($\zeta$), WinScroller 2D achieves a crisp **underdamped response**:

* **Natural Frequency**: $\omega_0 = \sqrt{k} \approx 16.12\text{ rad/s}$ ($f_0 \approx 2.57\text{ Hz}$).
* **Damping Ratio**: $\zeta = 0.72$ ($0 < \zeta < 1 \implies$ underdamped).
* **Overshoot**: A deliberate, tactile overshoot ($\approx 3.8\%$) that cushions smoothly into the settled resting position.
* **Settling Deadband**: Snaps cleanly once $|x - x_{\text{target}}| \le \epsilon$ (`snap_epsilon_px`, default `0.5` px) and $|v| \le 8\epsilon$.

```text
Position x(t)
    ▲
    │         ┌───── subtle overshoot (~3.8%)
    │       /   \
Target ────┼─────\─────────────────── Settled resting position
    │     /       \_________
    │    /
    │   /
    │  /
  0 └─┴──────────────────────────────► Time (ms)
      0   60   120  180  240  300
```

---

## ⏱️ Hardware VSync Pacing & Zero CPU Spin

Rather than spinning CPU cycles in a tight polling loop, `VsyncPacer` uses a condition variable (`Condvar`) paired with native DWM compositor VSync (`DwmFlush()`):

1. **Active Motion Phase**:
   - Wakes on each DWM compositor VSync (`DwmFlush()`, with an `8 ms` fallback if DWM is unavailable) and posts `WM_APP_VSYNC_STEP`.
   - Sub-steps semi-implicit Euler integration in `Spring1D::step` ($h \le 4\text{ ms}$, up to 16 substeps per frame) for unconditional numerical stability:
     $$\Delta t = \text{clamp}(t - t_{\text{prev}}, 0.5\text{ms}, 50\text{ms})$$
     $$v_{t+h} = v_t + \big(-k(x_t - x_{\text{target}}) - c \cdot v_t\big) h$$
     $$x_{t+h} = x_t + v_{t+h} h$$
2. **Resting Phase (Zero CPU)**:
   - When `is_active(snap_epsilon_px)` returns `false`, `VsyncPacer::set_active(false)` parks the background thread on its `Condvar`.
   - The main thread sleeps on `GetMessageW` with **$0.0\%$ idle CPU**.

---

## ⚙️ Configuration Reference

Spring parameters can be customized in `%APPDATA%\WinScroller2D\winscroller2d.toml`:

```toml
[animation]
enabled = true
animate_resize = false   # Keep windows static; bounce physics applied purely to camera translation
stiffness = 260.0        # Higher = faster, snappier snap (clamped: 40.0..=1500.0)
damping_ratio = 0.72     # < 1.0 underdamped bounce; 1.0 critically damped (clamped: 0.45..=2.0)
fps = 120                # Target animation refresh rate in Hz (clamped: 15..=240)
snap_epsilon_px = 0.5    # Pixel threshold at which springs settle cleanly onto targets
```

You can fine-tune stiffness and damping values interactively by pressing `Win+Alt+S` to open the Live Settings overlay.
