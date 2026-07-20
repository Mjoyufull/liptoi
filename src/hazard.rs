//! Deterministic obstacle generation and collision rules.

use crate::math::Vec2;

/// Perspective depth at which a gate meets the player plane.
pub const IMPACT_DEPTH: f32 = 0.055;

/// The shape of a gate's safe opening.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateKind {
    /// A balanced rectangular aperture.
    Aperture,
    /// A narrow vertical opening with generous vertical movement.
    VerticalSlot,
    /// A narrow horizontal opening with generous horizontal movement.
    HorizontalSlot,
    /// A smaller offset opening used after the player has settled in.
    OffsetCore,
}

impl GateKind {
    /// Compact label used by the HUD.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Aperture => "APERTURE",
            Self::VerticalSlot => "V-SLOT",
            Self::HorizontalSlot => "H-SLOT",
            Self::OffsetCore => "OFFSET",
        }
    }

    /// Half-width and half-height of the safe opening in world space.
    #[must_use]
    pub const fn half_size(self) -> Vec2 {
        match self {
            Self::Aperture => Vec2::new(0.34, 0.32),
            Self::VerticalSlot => Vec2::new(0.24, 0.54),
            Self::HorizontalSlot => Vec2::new(0.51, 0.23),
            Self::OffsetCore => Vec2::new(0.29, 0.29),
        }
    }
}

/// One wall moving down the tunnel toward the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gate {
    /// Stable sequence number.
    pub id: u32,
    /// Perspective depth: `1` is distant and `0` is the player plane.
    pub depth: f32,
    /// Center of the safe opening in normalized arena coordinates.
    pub opening: Vec2,
    /// Opening shape.
    pub kind: GateKind,
}

impl Gate {
    /// Returns whether a circular player fits through the opening.
    #[must_use]
    pub fn clears(self, player: Vec2, radius: f32) -> bool {
        let half_size = self.kind.half_size();
        let relative = player - self.opening;
        relative.x.abs() + radius <= half_size.x && relative.y.abs() + radius <= half_size.y
    }
}

/// Small deterministic generator that creates fair, reproducible gate sequences.
#[derive(Clone, Debug)]
pub struct GateSequence {
    random: XorShift32,
    next_id: u32,
    previous_opening: Vec2,
}

impl GateSequence {
    /// Creates a sequence with a non-zero deterministic seed.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self {
            random: XorShift32::new(seed),
            next_id: 1,
            previous_opening: Vec2::ZERO,
        }
    }

    /// Produces the next gate at the requested depth.
    pub fn next_gate(&mut self, depth: f32) -> Gate {
        let id = self.next_id;
        self.next_id += 1;

        let kind = match id % 7 {
            2 | 6 => GateKind::VerticalSlot,
            3 | 5 => GateKind::HorizontalSlot,
            0 if id > 7 => GateKind::OffsetCore,
            _ => GateKind::Aperture,
        };
        let range = if kind == GateKind::OffsetCore {
            0.55
        } else {
            0.47
        };
        let candidate = Vec2::new(self.random.signed() * range, self.random.signed() * range);
        let maximum_step = if id < 4 { 0.34 } else { 0.52 };
        let step = (candidate - self.previous_opening).limit(maximum_step);
        let opening = (self.previous_opening + step).clamp(-range, range);
        self.previous_opening = opening;

        Gate {
            id,
            depth,
            opening,
            kind,
        }
    }
}

#[derive(Clone, Debug)]
struct XorShift32 {
    state: u32,
}

impl XorShift32 {
    fn new(seed: u32) -> Self {
        Self { state: seed.max(1) }
    }

    fn next(&mut self) -> u32 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.state = value;
        value
    }

    fn signed(&mut self) -> f32 {
        let unit = self.next() as f32 / u32::MAX as f32;
        unit.mul_add(2.0, -1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{Gate, GateKind, GateSequence};
    use crate::math::Vec2;

    #[test]
    fn collision_includes_the_player_radius() {
        let gate = Gate {
            id: 1,
            depth: 0.0,
            opening: Vec2::ZERO,
            kind: GateKind::Aperture,
        };

        assert!(gate.clears(Vec2::new(0.2, 0.1), 0.08));
        assert!(!gate.clears(Vec2::new(0.3, 0.1), 0.08));
    }

    #[test]
    fn generated_openings_never_jump_beyond_the_fairness_budget() {
        let mut sequence = GateSequence::new(42);
        let mut previous = Vec2::ZERO;

        for index in 0..50 {
            let gate = sequence.next_gate(index as f32);
            assert!((gate.opening - previous).length() <= 0.521);
            previous = gate.opening;
        }
    }

    #[test]
    fn identical_seeds_produce_identical_sequences() {
        let mut left = GateSequence::new(7);
        let mut right = GateSequence::new(7);

        for depth in [0.3, 0.6, 0.9, 1.2] {
            assert_eq!(left.next_gate(depth), right.next_gate(depth));
        }
    }
}
