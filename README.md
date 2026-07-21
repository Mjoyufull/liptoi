# Liptoi

> Tilt through the fever. Dodge the wall. Become sand.

[![CI](https://github.com/Mjoyufull/liptoi/actions/workflows/ci.yml/badge.svg)](https://github.com/Mjoyufull/liptoi/actions/workflows/ci.yml)
[![Deploy](https://github.com/Mjoyufull/liptoi/actions/workflows/pages.yml/badge.svg)](https://github.com/Mjoyufull/liptoi/actions/workflows/pages.yml)

Liptoi is a slow-burn tunnel dodger rendered entirely as a terminal interface in the browser.
Obstacles grow out of the vanishing point, leaving one safe opening. Tilt a phone to roll and flip
the sphere into that opening before the wall reaches the player plane. A collision turns the sphere
into 220 simulated grains that keep responding to gravity and tilt around the game-over screen.

**Play:** <https://mjoyufull.github.io/liptoi/>

The visual language combines a crisp Ratatui frame, the bounded playfield of rhythm games, and the
cool tunnel / hot obstacle contrast of *Sensory Overload*. The mechanics, layouts, simulation, and
terminal artwork are original to Liptoi.

## Controls

| Input | Action |
| --- | --- |
| Phone tilt | Steer the sphere; pour the sand after impact |
| Touch or mouse drag | Direct fallback steering |
| WASD or arrow keys | Keyboard steering |
| Space / Enter / tap | Start or reform after impact |
| R / **Recenter Tilt** | Use the current phone pose as neutral |

Press **Arm Motion + Start** from the phone itself. iOS requires that direct gesture before a site
can read orientation. Motion sensors require HTTPS in production; `localhost` is accepted for local
development. If permission is denied or the device has no sensor, touch and keyboard controls remain
fully playable.

Orientation samples never leave the tab. Liptoi has no analytics or network telemetry. Only the best
score is retained, in browser-local storage.

## Gameplay model

- Gates arrive slowly enough to read their shape and opening before committing.
- Apertures, vertical slots, horizontal slots, and offset cores vary the movement pattern.
- A deterministic generator limits the distance between consecutive openings so sequences stay fair.
- Speed rises from `0.105` to at most `0.160` tunnel-depth units per second.
- Pointer steering uses a spring target; tilt and keys use acceleration, damping, and wall rebound.
- Impact inherits the sphere velocity, then every sand grain responds to tilt, gravity, drag, and the
  playfield walls.

## Technology

- [Ratatui 0.30.2](https://crates.io/crates/ratatui/0.30.2) composes every visible game screen.
- [Ratzilla 0.3.1](https://crates.io/crates/ratzilla/0.3.1) renders Ratatui through a responsive WebGL2
  canvas and runs the browser animation loop.
- Rust owns gameplay, input shaping, collision, sand physics, persistence, and rendering.
- A small HTML boundary performs the user-gesture permission request required by mobile browsers.
- The release bundle is roughly 1.6 MiB before transfer compression and has no runtime CDN assets.

The WebGL2 backend is deliberate: unlike the DOM backend, it safely resizes its terminal grid during
phone rotation and keeps the particle field smooth. A dynamic font atlas preserves the Unicode
terminal glyphs at desktop and phone sizes.

## Run locally

Prerequisites are Rust 1.90 or newer, the `wasm32-unknown-unknown` target, and
[Trunk](https://trunkrs.dev/).

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
trunk serve --open
```

Open the LAN HTTPS URL on a phone when testing real sensors. Plain HTTP is sufficient only on
`localhost`; browsers intentionally block motion APIs in an insecure remote context.

## Deploy

Pushes to `main` publish the Trunk release bundle through GitHub Actions. A newly created repository
must first select **Settings → Pages → Build and deployment → Source: GitHub Actions**; after that,
the checked-in Pages workflow handles each deployment.

## Verify

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --locked
cargo clippy --target wasm32-unknown-unknown --all-features --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
trunk build --release
```

The suite covers orientation calibration and rotation mapping, fair gate generation, radius-aware
collisions, bounded player motion, sand response and containment, state transitions, and responsive
Ratatui layouts from `28×10` through `180×56` cells.

For a dependency-free browser pass, serve `dist/`, launch Firefox with a BiDi port, then run:

```bash
node tools/visual-check.mjs \
  ws://127.0.0.1:9222/session \
  http://127.0.0.1:4173/ \
  /tmp/liptoi-visual-check
```

The script captures ready, gyro-controlled running, sand/game-over, and mobile states. It also fails
if Firefox reports a browser error.

## Project shape

```text
src/
├── browser.rs   Web APIs, permissions, persistence, and the fixed-step loop
├── game.rs      Game state machine and player physics
├── hazard.rs    Fair gate sequence and collision contracts
├── input.rs     Tilt calibration and unified control shaping
├── math.rs      Normalized 2D geometry
├── sand.rs      Post-impact grain simulation
└── ui/          Responsive Ratatui composition and tunnel renderer
```

The pure simulation modules compile and test natively. Browser types are confined to `browser.rs`,
which keeps sensor availability from infecting the game model.

## License

Liptoi is available under the [MIT License](LICENSE).
