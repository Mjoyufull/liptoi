//! Gameplay state machine and fixed-step physics.

use std::collections::VecDeque;

use crate::{
    hazard::{Gate, GateSequence, IMPACT_DEPTH},
    input::ControlFrame,
    math::Vec2,
    sand::SandCloud,
};

const PLAYER_RADIUS: f32 = 0.065;
const PLAYER_LIMIT: f32 = 0.91;
const BASE_SPEED: f32 = 0.105;
const MAX_SPEED_BONUS: f32 = 0.055;
const GATE_SPACING: f32 = 0.31;
const FIRST_GATE_DEPTH: f32 = 0.48;
const TRAIL_INTERVAL: f32 = 0.035;
const TRAIL_LENGTH: usize = 11;

/// Top-level game state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GamePhase {
    /// The tunnel is visible and waiting for a start gesture.
    #[default]
    Ready,
    /// Gates are advancing and collisions are active.
    Running,
    /// The sphere has burst into a tilt-responsive sand cloud.
    GameOver,
}

/// Player sphere state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Player {
    /// Sphere center in normalized arena coordinates.
    pub position: Vec2,
    /// Current velocity in arena units per second.
    pub velocity: Vec2,
    /// Visual rotation in radians.
    pub spin: f32,
}

/// Complete deterministic game simulation.
#[derive(Clone, Debug)]
pub struct Game {
    phase: GamePhase,
    phase_time: f32,
    elapsed: f32,
    player: Player,
    trail: VecDeque<Vec2>,
    trail_timer: f32,
    gates: Vec<Gate>,
    sequence: GateSequence,
    seed: u32,
    run_number: u32,
    cleared: u32,
    score: u32,
    best_score: u32,
    last_cleared_flash: f32,
    sand: Option<SandCloud>,
}

impl Game {
    /// Creates a ready game with a preview tunnel and deterministic seed.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        let stable_seed = seed.max(1);
        let mut game = Self {
            phase: GamePhase::Ready,
            phase_time: 0.0,
            elapsed: 0.0,
            player: Player::default(),
            trail: VecDeque::with_capacity(TRAIL_LENGTH),
            trail_timer: 0.0,
            gates: Vec::with_capacity(5),
            sequence: GateSequence::new(stable_seed),
            seed: stable_seed,
            run_number: 0,
            cleared: 0,
            score: 0,
            best_score: 0,
            last_cleared_flash: 0.0,
            sand: None,
        };
        game.populate_gates();
        game
    }

    /// Starts a ready run or reforms the sphere after the impact animation.
    pub fn trigger_start(&mut self) {
        let may_restart = self.phase != GamePhase::GameOver || self.phase_time >= 0.65;
        if self.phase != GamePhase::Running && may_restart {
            self.begin_run();
        }
    }

    /// Advances the simulation by a small fixed delta.
    pub fn update(&mut self, delta_seconds: f32, controls: ControlFrame) {
        let delta_seconds = delta_seconds.clamp(0.0, 0.05);
        self.phase_time += delta_seconds;
        self.last_cleared_flash = (self.last_cleared_flash - delta_seconds).max(0.0);

        match self.phase {
            GamePhase::Ready => self.update_ready(delta_seconds, controls),
            GamePhase::Running => self.update_running(delta_seconds, controls),
            GamePhase::GameOver => self.update_game_over(delta_seconds, controls),
        }
    }

    fn begin_run(&mut self) {
        self.run_number += 1;
        self.phase = GamePhase::Running;
        self.phase_time = 0.0;
        self.elapsed = 0.0;
        self.player = Player::default();
        self.trail.clear();
        self.trail.push_back(Vec2::ZERO);
        self.trail_timer = 0.0;
        self.cleared = 0;
        self.score = 0;
        self.last_cleared_flash = 0.0;
        self.sand = None;
        self.sequence = GateSequence::new(self.seed.wrapping_add(self.run_number * 977));
        self.gates.clear();
        self.populate_gates();
    }

    fn update_ready(&mut self, delta_seconds: f32, controls: ControlFrame) {
        self.update_player(delta_seconds, controls, 0.42);
        self.player.spin += delta_seconds * 0.8;
        self.record_trail(delta_seconds);
    }

    fn update_running(&mut self, delta_seconds: f32, controls: ControlFrame) {
        self.elapsed += delta_seconds;
        self.update_player(delta_seconds, controls, 1.0);
        self.record_trail(delta_seconds);

        let speed = self.speed();
        let mut impact = None;
        for gate in &mut self.gates {
            let previous_depth = gate.depth;
            gate.depth -= speed * delta_seconds;
            let crossed_player = previous_depth > IMPACT_DEPTH && gate.depth <= IMPACT_DEPTH;
            if crossed_player && !gate.clears(self.player.position, PLAYER_RADIUS) {
                impact = Some(gate.id);
                break;
            }
            if crossed_player {
                self.cleared += 1;
                self.last_cleared_flash = 0.34;
            }
        }

        self.score = self.cleared * 100 + (self.elapsed * 8.0) as u32;
        if let Some(gate_id) = impact {
            self.shatter(gate_id);
            return;
        }

        self.recycle_gates();
    }

    fn update_game_over(&mut self, delta_seconds: f32, controls: ControlFrame) {
        let sand_control = match controls.pointer_target {
            Some(target) => {
                let center = self
                    .sand
                    .as_ref()
                    .map_or(Vec2::ZERO, SandCloud::center_of_mass);
                (target - center).limit(1.0)
            }
            None => controls.vector,
        };
        if let Some(sand) = &mut self.sand {
            sand.update(delta_seconds, sand_control);
        }
    }

    fn update_player(&mut self, delta_seconds: f32, controls: ControlFrame, strength: f32) {
        let acceleration = match controls.pointer_target {
            Some(target) => {
                (target * 0.88 - self.player.position) * 15.0 - self.player.velocity * 5.4
            }
            None => controls.vector * 5.2 - self.player.velocity * 3.7,
        } * strength;

        self.player.velocity += acceleration * delta_seconds;
        self.player.velocity = self.player.velocity.limit(1.55);
        self.player.position += self.player.velocity * delta_seconds;
        self.player.spin += (0.9 + self.player.velocity.length() * 8.0) * delta_seconds;
        contain_player(&mut self.player);
    }

    fn record_trail(&mut self, delta_seconds: f32) {
        self.trail_timer += delta_seconds;
        if self.trail_timer < TRAIL_INTERVAL {
            return;
        }
        self.trail_timer %= TRAIL_INTERVAL;
        self.trail.push_front(self.player.position);
        self.trail.truncate(TRAIL_LENGTH);
    }

    fn shatter(&mut self, gate_id: u32) {
        self.phase = GamePhase::GameOver;
        self.phase_time = 0.0;
        self.best_score = self.best_score.max(self.score);
        let burst_seed = self.seed ^ gate_id.wrapping_mul(2_654_435_761);
        self.sand = Some(SandCloud::burst(
            self.player.position,
            self.player.velocity,
            burst_seed,
        ));
    }

    fn populate_gates(&mut self) {
        while self.gates.len() < 5 {
            let depth = self
                .gates
                .last()
                .map_or(FIRST_GATE_DEPTH, |gate| gate.depth + GATE_SPACING);
            self.gates.push(self.sequence.next_gate(depth));
        }
    }

    fn recycle_gates(&mut self) {
        self.gates.retain(|gate| gate.depth > -0.16);
        self.populate_gates();
    }

    /// Current game phase.
    #[must_use]
    pub const fn phase(&self) -> GamePhase {
        self.phase
    }

    /// Seconds spent in the current phase.
    #[must_use]
    pub const fn phase_time(&self) -> f32 {
        self.phase_time
    }

    /// Seconds survived in the current or most recent run.
    #[must_use]
    pub const fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Current player state.
    #[must_use]
    pub const fn player(&self) -> Player {
        self.player
    }

    /// Player trail ordered from newest to oldest.
    #[must_use]
    pub fn trail(&self) -> &VecDeque<Vec2> {
        &self.trail
    }

    /// Active and upcoming gates, nearest first.
    #[must_use]
    pub fn gates(&self) -> &[Gate] {
        &self.gates
    }

    /// Next gate that has not reached the player plane.
    #[must_use]
    pub fn next_gate(&self) -> Option<&Gate> {
        self.gates.iter().find(|gate| gate.depth > IMPACT_DEPTH)
    }

    /// Number of cleared gates.
    #[must_use]
    pub const fn cleared(&self) -> u32 {
        self.cleared
    }

    /// Current or final score.
    #[must_use]
    pub const fn score(&self) -> u32 {
        self.score
    }

    /// Best score observed in this browser session.
    #[must_use]
    pub const fn best_score(&self) -> u32 {
        self.best_score
    }

    /// Restores a persisted best score.
    pub fn set_best_score(&mut self, best_score: u32) {
        self.best_score = self.best_score.max(best_score);
    }

    /// Current tunnel speed in depth units per second.
    #[must_use]
    pub fn speed(&self) -> f32 {
        BASE_SPEED + (self.cleared as f32 * 0.003).min(MAX_SPEED_BONUS)
    }

    /// Remaining sand after a collision.
    #[must_use]
    pub fn sand(&self) -> Option<&SandCloud> {
        self.sand.as_ref()
    }

    /// Whether a recent successful gate clear should flash in the HUD.
    #[must_use]
    pub fn cleared_flash(&self) -> bool {
        self.last_cleared_flash > 0.0
    }
}

fn contain_player(player: &mut Player) {
    if player.position.x.abs() > PLAYER_LIMIT {
        player.position.x = player.position.x.clamp(-PLAYER_LIMIT, PLAYER_LIMIT);
        player.velocity.x *= -0.22;
    }
    if player.position.y.abs() > PLAYER_LIMIT {
        player.position.y = player.position.y.clamp(-PLAYER_LIMIT, PLAYER_LIMIT);
        player.velocity.y *= -0.22;
    }
}

#[cfg(test)]
mod tests {
    use super::{BASE_SPEED, Game, GamePhase};
    use crate::{
        hazard::{Gate, GateKind, IMPACT_DEPTH},
        input::ControlFrame,
        math::Vec2,
    };

    #[test]
    fn start_resets_every_run_counter() {
        let mut game = Game::new(5);
        game.trigger_start();
        game.cleared = 8;
        game.score = 999;
        game.phase = GamePhase::GameOver;
        game.phase_time = 1.0;

        game.trigger_start();

        assert_eq!(game.phase(), GamePhase::Running);
        assert_eq!(game.cleared(), 0);
        assert_eq!(game.score(), 0);
        assert_eq!(game.player().position, Vec2::ZERO);
    }

    #[test]
    fn missing_an_opening_shatters_the_sphere() {
        let mut game = Game::new(7);
        game.trigger_start();
        game.gates = vec![Gate {
            id: 99,
            depth: IMPACT_DEPTH + BASE_SPEED / 120.0,
            opening: Vec2::new(0.55, 0.55),
            kind: GateKind::OffsetCore,
        }];

        game.update(1.0 / 60.0, ControlFrame::default());

        assert_eq!(game.phase(), GamePhase::GameOver);
        assert_eq!(game.sand().map(|sand| sand.grains().len()), Some(220));
    }

    #[test]
    fn centered_player_clears_a_centered_gate() {
        let mut game = Game::new(7);
        game.trigger_start();
        game.gates = vec![Gate {
            id: 99,
            depth: IMPACT_DEPTH + BASE_SPEED / 120.0,
            opening: Vec2::ZERO,
            kind: GateKind::Aperture,
        }];

        game.update(1.0 / 60.0, ControlFrame::default());

        assert_eq!(game.phase(), GamePhase::Running);
        assert_eq!(game.cleared(), 1);
    }

    #[test]
    fn controls_move_the_player_without_leaving_the_arena() {
        let mut game = Game::new(3);
        game.trigger_start();
        game.gates.clear();
        let controls = ControlFrame {
            vector: Vec2::new(1.0, 1.0),
            ..ControlFrame::default()
        };

        for _ in 0..600 {
            game.update(1.0 / 60.0, controls);
            game.gates.clear();
        }

        assert!(game.player().position.x <= 0.91);
        assert!(game.player().position.y <= 0.91);
    }
}
