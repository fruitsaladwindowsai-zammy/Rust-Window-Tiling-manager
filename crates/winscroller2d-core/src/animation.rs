/// Critically-damped / slightly underdamped 1D physical spring solver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring1D {
    pub current: f64,
    pub target: f64,
    pub velocity: f64,
    pub initialized: bool,
}

impl Default for Spring1D {
    fn default() -> Self {
        Self {
            current: 0.0,
            target: 0.0,
            velocity: 0.0,
            initialized: false,
        }
    }
}

impl Spring1D {
    pub fn snap_to(&mut self, value: f64) {
        self.current = value;
        self.target = value;
        self.velocity = 0.0;
        self.initialized = true;
    }

    pub fn set_target(&mut self, value: f64, animate: bool) {
        if !self.initialized || !animate {
            self.snap_to(value);
        } else {
            self.target = value;
        }
    }

    pub fn shift(&mut self, delta: f64) {
        if self.initialized {
            self.current += delta;
            self.target += delta;
        }
    }

    pub fn is_active(&self, epsilon: f64) -> bool {
        self.initialized
            && ((self.target - self.current).abs() > epsilon || self.velocity.abs() > epsilon * 8.0)
    }

    /// Advance the spring by `dt_secs` with sub-stepping for unconditional stability.
    /// Returns `true` if still moving.
    pub fn step(&mut self, dt_secs: f64, stiffness: f64, damping_ratio: f64, epsilon: f64) -> bool {
        if !self.initialized {
            return false;
        }
        let dt_total = dt_secs.clamp(0.0005, 0.05);
        let k = stiffness.clamp(20.0, 2000.0);
        let zeta = damping_ratio.clamp(0.35, 2.0);
        let c = 2.0 * zeta * k.sqrt();

        let substeps = ((dt_total / 0.004).ceil() as usize).clamp(1, 16);
        let h = dt_total / substeps as f64;
        for _ in 0..substeps {
            let disp = self.current - self.target;
            let accel = -k * disp - c * self.velocity;
            self.velocity += accel * h;
            self.current += self.velocity * h;
        }

        if !self.is_active(epsilon) {
            self.current = self.target;
            self.velocity = 0.0;
            false
        } else {
            true
        }
    }
}

/// Smooth physical 2D spring camera state in grid-world pixel space.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CameraAnim {
    pub x: Spring1D,
    pub y: Spring1D,
}

impl CameraAnim {
    pub fn current_x(&self) -> f64 {
        self.x.current
    }

    pub fn current_y(&self) -> f64 {
        self.y.current
    }

    pub fn target_x(&self) -> f64 {
        self.x.target
    }

    pub fn target_y(&self) -> f64 {
        self.y.target
    }

    pub fn initialized(&self) -> bool {
        self.x.initialized && self.y.initialized
    }

    pub fn reset(&mut self) {
        self.x.initialized = false;
        self.y.initialized = false;
        self.x.velocity = 0.0;
        self.y.velocity = 0.0;
    }

    pub fn snap_to(&mut self, x: f64, y: f64) {
        self.x.snap_to(x);
        self.y.snap_to(y);
    }

    pub fn set_target(&mut self, x: f64, y: f64, animate: bool) {
        self.x.set_target(x, animate);
        self.y.set_target(y, animate);
    }

    /// Shift both current and target by `(dx, dy)` when the logical anchor cell
    /// moves in world-space due to an off-screen cell resizing or closing.
    pub fn shift_world(&mut self, dx: f64, dy: f64) {
        self.x.shift(dx);
        self.y.shift(dy);
    }

    pub fn is_active(&self, epsilon: f64) -> bool {
        self.x.is_active(epsilon) || self.y.is_active(epsilon)
    }

    /// Advance 2D spring animation by `dt_secs`. Returns `true` if still animating.
    pub fn step(&mut self, dt_secs: f64, stiffness: f64, damping_ratio: f64, epsilon: f64) -> bool {
        let ax = self.x.step(dt_secs, stiffness, damping_ratio, epsilon);
        let ay = self.y.step(dt_secs, stiffness, damping_ratio, epsilon);
        ax || ay
    }
}
