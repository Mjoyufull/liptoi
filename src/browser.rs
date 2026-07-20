//! WebAssembly runtime and browser sensor boundary.

use std::{
    cell::{Cell, RefCell},
    io,
    rc::Rc,
};

use ratatui::{Terminal, style::Color};
use ratzilla::{
    FontAtlasConfig, WebGl2Backend, WebRenderer,
    backend::webgl2::WebGl2BackendOptions,
    event::{KeyCode, KeyEvent},
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{CustomEvent, DeviceOrientationEvent, Element, Event, PointerEvent, Window};

use crate::{
    game::Game,
    input::{InputHub, MotionStatus},
    math::Vec2,
    ui::{self, UiTelemetry},
};

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
            .canvas_padding_color(Color::Rgb(3, 7, 12)),
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
        rendered_source: None,
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
    rendered_source: Option<crate::input::ControlSource>,
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
            hide_start_control(&self.window);
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
        self.sync_accessible_status();
        let telemetry = UiTelemetry::from(&*self.input.borrow());
        ui::render(frame, &self.game, telemetry);
    }

    fn sync_accessible_status(&mut self) {
        let phase = self.game.phase();
        let source = self.input.borrow().source();
        if self.rendered_phase == Some(phase) && self.rendered_source == Some(source) {
            return;
        }
        self.rendered_phase = Some(phase);
        self.rendered_source = Some(source);
        let (phase_name, label) = match phase {
            crate::game::GamePhase::Ready => ("ready", "Liptoi ready for motion"),
            crate::game::GamePhase::Running => ("running", "Liptoi run active"),
            crate::game::GamePhase::GameOver => {
                ("game-over", "Liptoi game over; tilt to move the sand")
            }
        };
        let source_name = match source {
            crate::input::ControlSource::Idle => "idle",
            crate::input::ControlSource::Tilt => "tilt",
            crate::input::ControlSource::Keyboard => "keyboard",
            crate::input::ControlSource::Pointer => "pointer",
        };
        let Some(document) = self.window.document() else {
            return;
        };
        if let Some(terminal) = document.get_element_by_id("terminal") {
            let _phase_result = terminal.set_attribute("data-game-phase", phase_name);
            let _source_result = terminal.set_attribute("data-control-source", source_name);
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

/// Owns callbacks for the lifetime of the page render loop.
struct BrowserBindings {
    _orientation: Closure<dyn FnMut(DeviceOrientationEvent)>,
    _motion_status: Closure<dyn FnMut(CustomEvent)>,
    _recenter: Closure<dyn FnMut(Event)>,
    _pointer_down: Closure<dyn FnMut(PointerEvent)>,
    _pointer_move: Closure<dyn FnMut(PointerEvent)>,
    _pointer_up: Closure<dyn FnMut(PointerEvent)>,
}

impl BrowserBindings {
    fn attach(window: &Window, input: Rc<RefCell<InputHub>>) -> Result<Self, JsValue> {
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("browser document missing"))?;
        let terminal = document
            .get_element_by_id("terminal")
            .ok_or_else(|| JsValue::from_str("#terminal element missing"))?;

        let orientation = orientation_listener(window, Rc::clone(&input));
        window.add_event_listener_with_callback(
            "deviceorientation",
            orientation.as_ref().unchecked_ref(),
        )?;

        let motion_status = motion_status_listener(Rc::clone(&input));
        window.add_event_listener_with_callback(
            "liptoi-motion-status",
            motion_status.as_ref().unchecked_ref(),
        )?;

        let recenter = recenter_listener(Rc::clone(&input));
        window.add_event_listener_with_callback(
            "liptoi-recenter",
            recenter.as_ref().unchecked_ref(),
        )?;

        let pointer_active = Rc::new(Cell::new(false));
        let pointer_down = pointer_down_listener(
            terminal.clone(),
            Rc::clone(&input),
            Rc::clone(&pointer_active),
        );
        terminal.add_event_listener_with_callback(
            "pointerdown",
            pointer_down.as_ref().unchecked_ref(),
        )?;

        let pointer_move = pointer_move_listener(
            terminal.clone(),
            Rc::clone(&input),
            Rc::clone(&pointer_active),
        );
        terminal.add_event_listener_with_callback(
            "pointermove",
            pointer_move.as_ref().unchecked_ref(),
        )?;

        let pointer_up = pointer_up_listener(input, pointer_active);
        for event_name in ["pointerup", "pointercancel", "pointerleave"] {
            terminal.add_event_listener_with_callback(
                event_name,
                pointer_up.as_ref().unchecked_ref(),
            )?;
        }

        Ok(Self {
            _orientation: orientation,
            _motion_status: motion_status,
            _recenter: recenter,
            _pointer_down: pointer_down,
            _pointer_move: pointer_move,
            _pointer_up: pointer_up,
        })
    }
}

fn orientation_listener(
    window: &Window,
    input: Rc<RefCell<InputHub>>,
) -> Closure<dyn FnMut(DeviceOrientationEvent)> {
    let window = window.clone();
    Closure::new(move |event: DeviceOrientationEvent| {
        let (Some(beta), Some(gamma)) = (event.beta(), event.gamma()) else {
            return;
        };
        let angle = window
            .screen()
            .and_then(|screen| screen.orientation().angle())
            .map(|angle| angle as i16)
            .unwrap_or(0);
        input
            .borrow_mut()
            .record_orientation(beta as f32, gamma as f32, angle);
    })
}

fn motion_status_listener(input: Rc<RefCell<InputHub>>) -> Closure<dyn FnMut(CustomEvent)> {
    Closure::new(move |event: CustomEvent| {
        let status = match event.detail().as_string().as_deref() {
            Some("granted") => MotionStatus::Listening,
            Some("denied") => MotionStatus::Denied,
            _ => MotionStatus::Unavailable,
        };
        let mut input = input.borrow_mut();
        input.set_motion_status(status);
        input.request_start();
    })
}

fn recenter_listener(input: Rc<RefCell<InputHub>>) -> Closure<dyn FnMut(Event)> {
    Closure::new(move |_event: Event| input.borrow_mut().recenter())
}

fn pointer_down_listener(
    terminal: Element,
    input: Rc<RefCell<InputHub>>,
    pointer_active: Rc<Cell<bool>>,
) -> Closure<dyn FnMut(PointerEvent)> {
    Closure::new(move |event: PointerEvent| {
        event.prevent_default();
        pointer_active.set(true);
        let mut input = input.borrow_mut();
        input.set_pointer(pointer_target(&terminal, &event));
        input.request_start();
    })
}

fn pointer_move_listener(
    terminal: Element,
    input: Rc<RefCell<InputHub>>,
    pointer_active: Rc<Cell<bool>>,
) -> Closure<dyn FnMut(PointerEvent)> {
    Closure::new(move |event: PointerEvent| {
        if !pointer_active.get() {
            return;
        }
        event.prevent_default();
        input
            .borrow_mut()
            .set_pointer(pointer_target(&terminal, &event));
    })
}

fn pointer_up_listener(
    input: Rc<RefCell<InputHub>>,
    pointer_active: Rc<Cell<bool>>,
) -> Closure<dyn FnMut(PointerEvent)> {
    Closure::new(move |_event: PointerEvent| {
        pointer_active.set(false);
        input.borrow_mut().clear_pointer();
    })
}

fn pointer_target(terminal: &Element, event: &PointerEvent) -> Vec2 {
    let bounds = terminal.get_bounding_client_rect();
    let width = bounds.width().max(1.0);
    let height = bounds.height().max(1.0);
    let x = ((f64::from(event.client_x()) - bounds.left()) / width).mul_add(2.0, -1.0);
    let y = ((f64::from(event.client_y()) - bounds.top()) / height).mul_add(2.0, -1.0);
    Vec2::new(x as f32, y as f32).clamp(-1.0, 1.0)
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

fn hide_start_control(window: &Window) {
    let Some(document) = window.document() else {
        return;
    };
    if let Some(button) = document.get_element_by_id("motion-start") {
        let _result = button.set_attribute("hidden", "");
    }
    if let Some(button) = document.get_element_by_id("recenter") {
        let _result = button.remove_attribute("hidden");
    }
}

fn js_error(error: JsValue) -> io::Error {
    io::Error::other(format!("browser integration failed: {error:?}"))
}
