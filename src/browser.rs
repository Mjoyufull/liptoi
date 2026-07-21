//! WebAssembly runtime and browser sensor boundary.

mod bindings;

use std::{cell::RefCell, io, rc::Rc};

use ratatui::{Terminal, style::Color};
use ratzilla::{
    FontAtlasConfig, WebGl2Backend, WebRenderer,
    backend::webgl2::WebGl2BackendOptions,
    event::{KeyCode, KeyEvent},
};
use wasm_bindgen::JsValue;
use web_sys::Window;

use crate::{
    game::Game,
    input::{ControlSource, InputHub, MotionStatus, PointerKind},
    math::Vec2,
    ui::{self, UiTelemetry},
};

use bindings::BrowserBindings;

const FIXED_STEP: f32 = 1.0 / 60.0;
const MAX_FRAME_DELTA: f32 = 0.1;
const MAX_STEPS_PER_FRAME: usize = 6;
const BEST_SCORE_KEY: &str = "liptoi.best-score.v1";

/// Starts the Ratzilla render loop and browser input listeners.
pub fn run() -> io::Result<()> {
    console_error_panic_hook::set_once();
    let window = web_sys::window().ok_or_else(|| io::Error::other("browser window missing"))?;
    let input = Rc::new(RefCell::new(InputHub::new()));
    let bindings = BrowserBindings::attach(&window, Rc::clone(&input)).map_err(js_error)?;

    let backend = WebGl2Backend::new_with_options(
        WebGl2BackendOptions::new()
            .grid_id("terminal")
            .font_atlas_config(FontAtlasConfig::dynamic(
                &["SFMono-Regular", "Cascadia Code", "DejaVu Sans Mono"],
                16.0,
            ))
            .fallback_glyph("?")
            .canvas_padding_color(Color::Rgb(3, 7, 12))
            .disable_auto_css_resize(),
    )?;
    let mut terminal = Terminal::new(backend)?;
    terminal.on_key_event({
        let input = Rc::clone(&input);
        move |event| handle_key(&mut input.borrow_mut(), event)
    })?;

    let mut game = Game::new(0x0011_F701);
    game.set_best_score(load_best_score(&window));
    let mut runtime = Runtime {
        game,
        input,
        _bindings: bindings,
        window,
        previous_time: None,
        accumulator: 0.0,
        persisted_best: 0,
        rendered_phase: None,
        rendered_input: None,
    };
    runtime.persisted_best = runtime.game.best_score();
    terminal.draw_web(move |frame| runtime.draw(frame));
    Ok(())
}

struct Runtime {
    game: Game,
    input: Rc<RefCell<InputHub>>,
    _bindings: BrowserBindings,
    window: Window,
    previous_time: Option<f64>,
    accumulator: f32,
    persisted_best: u32,
    rendered_phase: Option<crate::game::GamePhase>,
    rendered_input: Option<(ControlSource, PointerKind, MotionStatus)>,
}

impl Runtime {
    fn draw(&mut self, frame: &mut ratatui::Frame<'_>) {
        let now = self
            .window
            .performance()
            .map_or(0.0, |performance| performance.now());
        let delta_seconds = self
            .previous_time
            .map_or(0.0, |previous| ((now - previous) / 1_000.0) as f32)
            .clamp(0.0, MAX_FRAME_DELTA);
        self.previous_time = Some(now);
        self.accumulator = (self.accumulator + delta_seconds).min(MAX_FRAME_DELTA);

        let start_requested = self.input.borrow_mut().take_start_request();
        if start_requested {
            self.game.trigger_start();
        }

        let mut steps = 0;
        while self.accumulator >= FIXED_STEP && steps < MAX_STEPS_PER_FRAME {
            let controls = self.input.borrow_mut().sample(FIXED_STEP);
            self.game.update(FIXED_STEP, controls);
            self.accumulator -= FIXED_STEP;
            steps += 1;
        }

        if self.game.best_score() > self.persisted_best {
            save_best_score(&self.window, self.game.best_score());
            self.persisted_best = self.game.best_score();
        }
        let telemetry = UiTelemetry::from(&*self.input.borrow());
        self.sync_accessible_status(telemetry);
        ui::render(frame, &self.game, telemetry);
    }

    fn sync_accessible_status(&mut self, telemetry: UiTelemetry) {
        let phase = self.game.phase();
        let input_state = (
            telemetry.source,
            telemetry.pointer_kind,
            telemetry.motion_status,
        );
        if self.rendered_phase == Some(phase) && self.rendered_input == Some(input_state) {
            return;
        }
        self.rendered_phase = Some(phase);
        self.rendered_input = Some(input_state);
        let (phase_name, label) = match phase {
            crate::game::GamePhase::Ready => ("ready", "Liptoi ready for input"),
            crate::game::GamePhase::Running => ("running", "Liptoi run active"),
            crate::game::GamePhase::GameOver => {
                ("game-over", "Liptoi game over; move to pour the sand")
            }
        };
        let source_name = match telemetry.source {
            ControlSource::Idle => "idle",
            ControlSource::Tilt => "tilt",
            ControlSource::Keyboard => "keyboard",
            ControlSource::Pointer => match telemetry.pointer_kind {
                PointerKind::Mouse => "mouse",
                PointerKind::Touch => "touch",
                PointerKind::Pen => "pen",
                PointerKind::Unknown => "pointer",
            },
        };
        let motion_name = match telemetry.motion_status {
            MotionStatus::Waiting => "waiting",
            MotionStatus::Listening => "listening",
            MotionStatus::Active => "active",
            MotionStatus::Denied => "denied",
            MotionStatus::Unavailable => "unavailable",
        };
        let Some(document) = self.window.document() else {
            return;
        };
        if let Some(terminal) = document.get_element_by_id("terminal") {
            let _phase_result = terminal.set_attribute("data-game-phase", phase_name);
            let _source_result = terminal.set_attribute("data-control-source", source_name);
            let _motion_result = terminal.set_attribute("data-motion-status", motion_name);
            let _label_result = terminal.set_attribute("aria-label", label);
        }
    }
}

fn handle_key(input: &mut InputHub, event: KeyEvent) {
    match event.code {
        KeyCode::Left | KeyCode::Char('a' | 'A') => {
            input.pulse_keyboard(Vec2::new(-1.0, 0.0));
        }
        KeyCode::Right | KeyCode::Char('d' | 'D') => {
            input.pulse_keyboard(Vec2::new(1.0, 0.0));
        }
        KeyCode::Up | KeyCode::Char('w' | 'W') => {
            input.pulse_keyboard(Vec2::new(0.0, -1.0));
        }
        KeyCode::Down | KeyCode::Char('s' | 'S') => {
            input.pulse_keyboard(Vec2::new(0.0, 1.0));
        }
        KeyCode::Char('r' | 'R') => input.recenter(),
        KeyCode::Char(' ') | KeyCode::Enter => input.request_start(),
        _ => {}
    }
}

fn load_best_score(window: &Window) -> u32 {
    window
        .local_storage()
        .ok()
        .flatten()
        .and_then(|storage| storage.get_item(BEST_SCORE_KEY).ok().flatten())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn save_best_score(window: &Window, score: u32) {
    if let Ok(Some(storage)) = window.local_storage() {
        let _result = storage.set_item(BEST_SCORE_KEY, &score.to_string());
    }
}

fn js_error(error: JsValue) -> io::Error {
    io::Error::other(format!("browser integration failed: {error:?}"))
}
