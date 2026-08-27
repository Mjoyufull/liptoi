//! Platform-neutral control shaping for motion, keyboard, and pointer input.

use crate::math::Vec2;

const TILT_RANGE_DEGREES: f32 = 24.0;
const FILTER_RESPONSE: f32 = 11.0;
const KEYBOARD_HOLD_SECONDS: f32 = 0.16;

/// Browser motion permission and sensor availability.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MotionStatus {
    /// The permission gesture has not happened yet.
    #[default]
    Waiting,
    /// Permission was granted, but the first sensor sample has not arrived.
    Listening,
    /// Orientation samples are actively controlling the game.
    Active,
    /// The browser or user denied access.
    Denied,
    /// No orientation sensor API is available.
    Unavailable,
}

impl MotionStatus {
    /// Short label used by the terminal HUD.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Waiting => "ARM TO ENABLE",
            Self::Listening => "CALIBRATING",
            Self::Active => "TILT ONLINE",
            Self::Denied => "TOUCH / KEYS",
            Self::Unavailable => "TOUCH / KEYS",
        }
    }
}

/// The input source most recently used to steer the sphere.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ControlSource {
    /// No meaningful movement has been received yet.
    #[default]
    Idle,
    /// Device orientation controls.
    Tilt,
    /// Keyboard controls.
    Keyboard,
    /// Mouse, pen, or touch dragging.
    Pointer,
}

impl ControlSource {
    /// Short label used by the terminal HUD.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "STANDBY",
            Self::Tilt => "GYRO",
            Self::Keyboard => "KEYS",
            Self::Pointer => "POINTER",
        }
    }
}

/// Concrete pointer hardware reported by the browser.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PointerKind {
    /// No pointer event has been received yet.
    #[default]
    Unknown,
    /// A mouse or trackpad pointer.
    Mouse,
    /// A touchscreen pointer.
    Touch,
    /// A stylus pointer.
    Pen,
}

impl PointerKind {
    /// Short label used by the terminal HUD.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Unknown => "POINTER",
            Self::Mouse => "MOUSE",
            Self::Touch => "TOUCH",
            Self::Pen => "PEN",
        }
    }
}

/// Controls consumed by one simulation step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ControlFrame {
    /// Direction and strength for acceleration-based steering.
    pub vector: Vec2,
    /// Direct spring target used while pointer dragging.
    pub pointer_target: Option<Vec2>,
    /// Most recently active source.
    pub source: ControlSource,
}

/// Mutable input state shared by browser callbacks and the render loop.
#[derive(Debug, Default)]
pub struct InputHub {
    motion_status: MotionStatus,
    baseline: Option<Vec2>,
    tilt_target: Vec2,
    tilt_filtered: Vec2,
    keyboard: Vec2,
    keyboard_remaining: f32,
    pointer_target: Option<Vec2>,
    pointer_kind: PointerKind,
    touch_capable: bool,
    motion_capable: bool,
    source: ControlSource,
    start_requested: bool,
}

impl InputHub {
    /// Creates input state awaiting a user permission gesture.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the known motion permission state.
    pub fn set_motion_status(&mut self, status: MotionStatus) {
        self.motion_status = status;
        if matches!(status, MotionStatus::Denied | MotionStatus::Unavailable) {
            self.baseline = None;
            self.tilt_target = Vec2::ZERO;
            self.tilt_filtered = Vec2::ZERO;
        }
    }

    /// Records the input capabilities detected by the browser.
    pub fn set_capabilities(&mut self, touch_capable: bool, motion_capable: bool) {
        self.touch_capable = touch_capable;
        self.motion_capable = motion_capable;
        if !motion_capable {
            self.set_motion_status(MotionStatus::Unavailable);
        }
    }

    /// Records a raw orientation sample and maps it for the current screen rotation.
    pub fn record_orientation(&mut self, beta: f32, gamma: f32, screen_angle: i16) {
        if !matches!(
            self.motion_status,
            MotionStatus::Listening | MotionStatus::Active
        ) {
            return;
        }
        let oriented = orient_sample(beta, gamma, screen_angle);
        let baseline = match self.baseline {
            Some(baseline) => baseline,
            None => {
                self.baseline = Some(oriented);
                self.motion_status = MotionStatus::Active;
                return;
            }
        };

        self.motion_status = MotionStatus::Active;
        self.tilt_target = ((oriented - baseline) * TILT_RANGE_DEGREES.recip()).clamp(-1.0, 1.0);
        if self.tilt_target.length_squared() > 0.002_5 {
            self.source = ControlSource::Tilt;
        }
    }

    /// Uses the next valid sensor sample as the neutral pose.
    pub fn recenter(&mut self) {
        self.baseline = None;
        self.tilt_target = Vec2::ZERO;
        self.tilt_filtered = Vec2::ZERO;
    }

    /// Applies a short keyboard steering pulse. Key repeat extends the pulse.
    pub fn pulse_keyboard(&mut self, direction: Vec2) {
        self.keyboard = direction.limit(1.0);
        self.keyboard_remaining = KEYBOARD_HOLD_SECONDS;
        self.source = ControlSource::Keyboard;
    }

    /// Starts or updates pointer steering in normalized viewport coordinates.
    pub fn set_pointer(&mut self, target: Vec2, kind: PointerKind) {
        self.pointer_target = Some(target.clamp(-1.0, 1.0));
        self.pointer_kind = kind;
        self.source = ControlSource::Pointer;
    }

    /// Ends pointer steering.
    pub fn clear_pointer(&mut self) {
        self.pointer_target = None;
    }

    /// Requests a start or restart on the next frame.
    pub fn request_start(&mut self) {
        self.start_requested = true;
    }

    /// Consumes a pending start or restart request.
    pub fn take_start_request(&mut self) -> bool {
        std::mem::take(&mut self.start_requested)
    }

    /// Advances smoothing and produces controls for the simulation.
    #[must_use]
    pub fn sample(&mut self, delta_seconds: f32) -> ControlFrame {
        let blend = 1.0 - (-FILTER_RESPONSE * delta_seconds).exp();
        self.tilt_filtered = self.tilt_filtered.lerp(self.tilt_target, blend);
        self.keyboard_remaining = (self.keyboard_remaining - delta_seconds).max(0.0);

        let vector = if self.keyboard_remaining > 0.0 {
            self.keyboard
        } else {
            self.tilt_filtered
        };

        ControlFrame {
            vector,
            pointer_target: self.pointer_target,
            source: self.source,
        }
    }

    /// Returns the current permission state.
    #[must_use]
    pub const fn motion_status(&self) -> MotionStatus {
        self.motion_status
    }

    /// Returns the most recently active control source.
    #[must_use]
    pub const fn source(&self) -> ControlSource {
        self.source
    }

    /// Returns the concrete pointer device most recently used.
    #[must_use]
    pub const fn pointer_kind(&self) -> PointerKind {
        self.pointer_kind
    }

    /// Returns whether the browser reports direct touch input.
    #[must_use]
    pub const fn touch_capable(&self) -> bool {
        self.touch_capable
    }

    /// Returns whether motion input is a plausible capability on this device.
    #[must_use]
    pub const fn motion_capable(&self) -> bool {
        self.motion_capable
    }

    /// Returns the filtered tilt vector for telemetry.
    #[must_use]
    pub const fn tilt(&self) -> Vec2 {
        self.tilt_filtered
    }
}

fn orient_sample(beta: f32, gamma: f32, screen_angle: i16) -> Vec2 {
    match screen_angle.rem_euclid(360) {
        90 => Vec2::new(beta, -gamma),
        270 => Vec2::new(-beta, gamma),
        180 => Vec2::new(-gamma, -beta),
        _ => Vec2::new(gamma, beta),
    }
}

#[cfg(test)]
mod tests {
    use super::{ControlSource, InputHub, MotionStatus, PointerKind};
    use crate::math::Vec2;

    #[test]
    fn first_orientation_sample_sets_a_neutral_pose() {
        let mut input = InputHub::new();
        input.set_motion_status(MotionStatus::Listening);
        input.record_orientation(52.0, 8.0, 0);

        assert_eq!(input.motion_status(), MotionStatus::Active);
        assert_eq!(input.sample(1.0 / 60.0).vector, Vec2::ZERO);
    }

    #[test]
    fn portrait_tilt_is_normalized_from_the_baseline() {
        let mut input = InputHub::new();
        input.set_motion_status(MotionStatus::Listening);
        input.record_orientation(50.0, 5.0, 0);
        input.record_orientation(74.0, 29.0, 0);

        let controls = input.sample(1.0);
        assert!((controls.vector.x - 1.0).abs() < 0.001);
        assert!((controls.vector.y - 1.0).abs() < 0.001);
        assert_eq!(controls.source, ControlSource::Tilt);
    }

    #[test]
    fn landscape_rotation_keeps_controls_screen_relative() {
        let mut input = InputHub::new();
        input.set_motion_status(MotionStatus::Listening);
        input.record_orientation(10.0, 20.0, 90);
        input.record_orientation(34.0, -4.0, 90);

        let controls = input.sample(1.0);
        assert!((controls.vector.x - 1.0).abs() < 0.001);
        assert!((controls.vector.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn start_requests_are_consumed_once() {
        let mut input = InputHub::new();
        input.request_start();

        assert!(input.take_start_request());
        assert!(!input.take_start_request());
    }

    #[test]
    fn orientation_is_ignored_until_motion_is_armed() {
        let mut input = InputHub::new();
        input.record_orientation(52.0, 8.0, 0);

        assert_eq!(input.motion_status(), MotionStatus::Waiting);
        assert_eq!(input.sample(1.0).vector, Vec2::ZERO);
    }

    #[test]
    fn pointer_events_preserve_the_actual_pointer_kind() {
        let mut input = InputHub::new();
        input.set_pointer(Vec2::new(0.5, -0.5), PointerKind::Mouse);

        assert_eq!(input.source(), ControlSource::Pointer);
        assert_eq!(input.pointer_kind(), PointerKind::Mouse);
        assert_eq!(input.pointer_kind().label(), "MOUSE");
    }
}
