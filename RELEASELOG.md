# Release log

## [0.1.2] Latest

Fixed

- Made the WebGL canvas fill one CSS-owned responsive rectangle with a matching high-resolution
  backing buffer in desktop, phone portrait, and phone landscape layouts.
- Detect mouse, touch, and pen pointers separately so laptops no longer advertise touch controls.
- Request phone orientation permission only from an explicit gesture, wait for the first valid sensor
  sample before starting, and fall back cleanly when permission or data is unavailable.
- Keep tilt, pointer, and keyboard inputs available with visible last-input auto-switching.
- Shortened narrow terminal prompts so mobile instructions remain inside the viewport.

Changed

- Trylle is now the primary forge and GitHub is the exact mirror used for free Pages hosting.
- Added `trylle.md` to track feature experiments, platform experience, and reproducible Trylle bugs.
- Expanded the Firefox browser pass to assert canvas geometry, backing resolution, overflow, device
  labels, gyro calibration, first-sample start behavior, control switching, and both phone rotations.

Notes

- SemVer: this patch corrects browser input and responsive presentation without changing saved data.

Contributors

- @Mjoyufull
- Co-authored-by: Codex

Compatibility

- Motion remains optional; desktop mouse/keyboard and phone touch/keyboard work without sensor access.
- Existing browser-local best scores remain compatible.

---

## [0.1.1]

Fixed

- Updated every Pages action to its current Node.js 24 release line.
- Documented the one-time GitHub Actions source selection required for a new Pages site.

Notes

- SemVer: this patch repairs deployment configuration without changing gameplay or saved data.

Contributors

- @Mjoyufull
- Co-authored-by: Codex

Compatibility

- Runtime and browser compatibility are unchanged from 0.1.0.

---

## [0.1.0]

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
