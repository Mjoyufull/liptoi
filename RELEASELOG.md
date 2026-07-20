# Release log

## [0.1.0] Latest

Added

- A slow, score-driven tunnel dodger rendered with Ratatui and Ratzilla.
- Calibrated phone-orientation controls with keyboard and pointer fallbacks.
- Deterministic gates, collision detection, trails, and a gyro-reactive sand crash scene.
- Responsive desktop and mobile terminal layouts with motion-permission and privacy guidance.

Technical details

- Pure fixed-step simulation modules remain testable outside the browser.
- WebGL2 rendering resizes safely across phone rotation and viewport changes.
- CI verifies formatting, native and WebAssembly linting, tests, documentation, and release builds.

Documentation

- README: controls, privacy behavior, architecture, verification, and local development.
- `codingprocess.md`: repository-specific development and release checks.

Notes

- SemVer: this is the initial public feature release.
- The application stores only the local best score and sends no motion data to a server.

Contributors

- @Mjoyufull
- Co-authored-by: Codex

Compatibility

- Rust 1.90 or newer for source builds.
- Modern browsers with WebAssembly and WebGL2; device orientation is optional.
- No configuration or saved-data migration is required.
