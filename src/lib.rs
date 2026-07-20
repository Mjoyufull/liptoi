//! Core simulation and terminal presentation for Liptoi.
//!
//! Browser APIs stay behind the `browser` module. Gameplay, input shaping, sand physics, and UI
//! composition remain deterministic and can be tested on a native target.

pub mod game;
pub mod hazard;
pub mod input;
pub mod math;
pub mod sand;
pub mod ui;

#[cfg(target_arch = "wasm32")]
pub mod browser;
