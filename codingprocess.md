# Liptoi coding process

This is the repo-local session contract. It adapts the workspace standards to this browser game.

## Read order

1. `README.md` for the player contract and verification commands.
2. `Cargo.toml` for the supported toolchain and exact Ratatui/Ratzilla versions.
3. `src/game.rs`, `src/input.rs`, and `src/hazard.rs` for behavior.
4. `src/ui.rs` and `src/ui/arena.rs` for presentation.
5. `src/browser.rs` only when work crosses the WebAssembly boundary.

## Non-negotiables

- Keep gameplay deterministic and platform-neutral; browser APIs stay in `browser.rs`.
- Keep every game screen inside Ratatui. HTML is only the host, permission gesture, and accessible
  control dock.
- Preserve gyro, pointer, and keyboard parity.
- Do not collect or transmit orientation data.
- Keep the fixed-step simulation at 60 Hz and clamp resumed-frame deltas.
- Do not make gate sequences faster by default; readability is part of the game identity.
- Do not use mutable globals, unsafe Rust, hidden panics, blanket lint allowances, or `unwrap` as
  application error handling.
- Treat phone rotation and narrow portrait layouts as first-class behavior.

## Structure

- Files should remain below 500 lines and normally below 350 lines.
- Functions should normally remain below 40 lines.
- Split by game responsibility, never into `utils`, `helpers`, or other junk drawers.
- Comments explain invariants or browser constraints, not syntax or authoring history.
- Public items require useful rustdoc; `main.rs` remains thin.

## Verification

Run before handing off a behavioral change:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --locked
cargo clippy --target wasm32-unknown-unknown --all-features --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
trunk build --release
```

UI, input-boundary, resizing, or particle changes also require the Firefox visual check documented in
`README.md`. A green native suite is not evidence that the browser bundle starts or looks correct.

## Git

- Treat Trylle as the source of truth and GitHub as an exact mirror used for Pages.
- Record Trylle feature experiments and platform bugs in `trylle.md`.
- Use Conventional Commits.
- Work on `feat/*`, `fix/*`, `refactor/*`, or `chore/*` branches from `dev`.
- Code reaches `dev` through a pull request and `main` through a release branch.
- Documentation-only branches start from and target `main`, then sync back to `dev`.
- Never force-push or rewrite shared history without explicit maintainer direction.
