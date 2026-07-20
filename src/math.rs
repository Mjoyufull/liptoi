//! Small geometry primitives shared by the simulation modules.

use std::ops::{Add, AddAssign, Mul, MulAssign, Sub};

/// A two-dimensional vector in normalized arena space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    /// Horizontal component. Positive values point right.
    pub x: f32,
    /// Vertical component. Positive values point down.
    pub y: f32,
}

impl Vec2 {
    /// The zero vector.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Creates a vector from its components.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns the squared vector length.
    #[must_use]
    pub fn length_squared(self) -> f32 {
        self.x.mul_add(self.x, self.y * self.y)
    }

    /// Returns the vector length.
    #[must_use]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Returns a unit vector, or zero when the input has no direction.
    #[must_use]
    pub fn normalized(self) -> Self {
        let length = self.length();
        if length <= f32::EPSILON {
            Self::ZERO
        } else {
            self * length.recip()
        }
    }

    /// Limits the vector magnitude while preserving direction.
    #[must_use]
    pub fn limit(self, maximum: f32) -> Self {
        let length_squared = self.length_squared();
        if length_squared > maximum * maximum {
            self.normalized() * maximum
        } else {
            self
        }
    }

    /// Clamps both components to the supplied inclusive range.
    #[must_use]
    pub fn clamp(self, minimum: f32, maximum: f32) -> Self {
        Self::new(
            self.x.clamp(minimum, maximum),
            self.y.clamp(minimum, maximum),
        )
    }

    /// Linearly interpolates between two vectors.
    #[must_use]
    pub fn lerp(self, other: Self, amount: f32) -> Self {
        self + (other - self) * amount
    }
}

impl Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::Vec2;

    #[test]
    fn limit_preserves_direction() {
        let limited = Vec2::new(3.0, 4.0).limit(2.0);

        assert!((limited.length() - 2.0).abs() < 0.000_1);
        assert!((limited.x / limited.y - 0.75).abs() < 0.000_1);
    }

    #[test]
    fn normalize_zero_remains_finite() {
        assert_eq!(Vec2::ZERO.normalized(), Vec2::ZERO);
    }
}
