//! Sand-like particles created when the player collides with a gate.

use crate::math::Vec2;

const PARTICLE_COUNT: usize = 220;
const WALL_X: f32 = 0.96;
const WALL_Y: f32 = 0.92;

/// One grain in the game-over particle cloud.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grain {
    /// Grain position in normalized arena coordinates.
    pub position: Vec2,
    /// Grain velocity in arena units per second.
    pub velocity: Vec2,
    /// Stable brightness variation in the range `0..=1`.
    pub tone: f32,
}

/// A bounded particle field that continues responding to tilt after impact.
#[derive(Clone, Debug, Default)]
pub struct SandCloud {
    grains: Vec<Grain>,
}

impl SandCloud {
    /// Bursts a sphere into deterministic grains.
    #[must_use]
    pub fn burst(origin: Vec2, inherited_velocity: Vec2, seed: u32) -> Self {
        let mut random = XorShift32::new(seed);
        let mut grains = Vec::with_capacity(PARTICLE_COUNT);

        for index in 0..PARTICLE_COUNT {
            let angle = random.unit() * std::f32::consts::TAU;
            let distance = random.unit().sqrt() * 0.075;
            let direction = Vec2::new(angle.cos(), angle.sin());
            let speed = 0.35 + random.unit() * 1.75;
            let spray = direction * speed + inherited_velocity * 0.55;

            grains.push(Grain {
                position: origin + direction * distance,
                velocity: spray,
                tone: index as f32 / (PARTICLE_COUNT - 1) as f32,
            });
        }

        Self { grains }
    }

    /// Advances all grains with natural gravity, tilt acceleration, and soft wall collisions.
    pub fn update(&mut self, delta_seconds: f32, tilt: Vec2) {
        let acceleration = tilt.limit(1.0) * 3.1 + Vec2::new(0.0, 0.32);
        let damping = 0.985_f32.powf(delta_seconds * 60.0);

        for grain in &mut self.grains {
            grain.velocity += acceleration * delta_seconds;
            grain.velocity *= damping;
            grain.velocity = grain.velocity.limit(2.8);
            grain.position += grain.velocity * delta_seconds;
            collide_with_walls(grain);
        }
    }

    /// Returns the grains for rendering and telemetry.
    #[must_use]
    pub fn grains(&self) -> &[Grain] {
        &self.grains
    }

    /// Returns the average grain position.
    #[must_use]
    pub fn center_of_mass(&self) -> Vec2 {
        if self.grains.is_empty() {
            return Vec2::ZERO;
        }
        let sum = self.grains.iter().fold(Vec2::ZERO, |accumulator, grain| {
            accumulator + grain.position
        });
        sum * (self.grains.len() as f32).recip()
    }
}

fn collide_with_walls(grain: &mut Grain) {
    if grain.position.x.abs() > WALL_X {
        grain.position.x = grain.position.x.clamp(-WALL_X, WALL_X);
        grain.velocity.x *= -0.24;
        grain.velocity.y *= 0.91;
    }
    if grain.position.y.abs() > WALL_Y {
        grain.position.y = grain.position.y.clamp(-WALL_Y, WALL_Y);
        grain.velocity.y *= -0.20;
        grain.velocity.x *= 0.93;
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

    fn unit(&mut self) -> f32 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.state = value;
        value as f32 / u32::MAX as f32
    }
}

#[cfg(test)]
mod tests {
    use super::SandCloud;
    use crate::math::Vec2;

    #[test]
    fn burst_creates_the_full_particle_field() {
        let cloud = SandCloud::burst(Vec2::ZERO, Vec2::ZERO, 9);

        assert_eq!(cloud.grains().len(), 220);
    }

    #[test]
    fn grains_remain_inside_the_arena() {
        let mut cloud = SandCloud::burst(Vec2::new(0.9, 0.85), Vec2::new(4.0, 3.0), 11);

        for _ in 0..600 {
            cloud.update(1.0 / 60.0, Vec2::new(1.0, 1.0));
        }

        assert!(
            cloud
                .grains()
                .iter()
                .all(|grain| grain.position.x.abs() <= 0.96 && grain.position.y.abs() <= 0.92)
        );
    }

    #[test]
    fn sustained_tilt_moves_the_center_of_mass() {
        let mut cloud = SandCloud::burst(Vec2::ZERO, Vec2::ZERO, 13);
        let before = cloud.center_of_mass();

        for _ in 0..120 {
            cloud.update(1.0 / 60.0, Vec2::new(1.0, 0.0));
        }

        assert!(cloud.center_of_mass().x > before.x + 0.2);
    }
}
