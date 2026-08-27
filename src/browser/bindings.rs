//! Browser event listeners retained for the lifetime of the render loop.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{CustomEvent, DeviceOrientationEvent, Element, Event, PointerEvent, Window};

use crate::{
    input::{InputHub, MotionStatus, PointerKind},
    math::Vec2,
};

pub(super) struct BrowserBindings {
    _orientation: Closure<dyn FnMut(DeviceOrientationEvent)>,
    _capabilities: Closure<dyn FnMut(CustomEvent)>,
    _motion_status: Closure<dyn FnMut(CustomEvent)>,
    _start: Closure<dyn FnMut(Event)>,
    _recenter: Closure<dyn FnMut(Event)>,
    _pointer_down: Closure<dyn FnMut(PointerEvent)>,
    _pointer_move: Closure<dyn FnMut(PointerEvent)>,
    _pointer_up: Closure<dyn FnMut(PointerEvent)>,
}

impl BrowserBindings {
    pub(super) fn attach(window: &Window, input: Rc<RefCell<InputHub>>) -> Result<Self, JsValue> {
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

        let capabilities = capabilities_listener(Rc::clone(&input));
        window.add_event_listener_with_callback(
            "liptoi-capabilities",
            capabilities.as_ref().unchecked_ref(),
        )?;

        let motion_status = motion_status_listener(Rc::clone(&input));
        window.add_event_listener_with_callback(
            "liptoi-motion-status",
            motion_status.as_ref().unchecked_ref(),
        )?;

        let start = start_listener(Rc::clone(&input));
        window.add_event_listener_with_callback("liptoi-start", start.as_ref().unchecked_ref())?;

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
            _capabilities: capabilities,
            _motion_status: motion_status,
            _start: start,
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
            Some("listening") => MotionStatus::Listening,
            Some("active") => MotionStatus::Active,
            Some("denied") => MotionStatus::Denied,
            _ => MotionStatus::Unavailable,
        };
        input.borrow_mut().set_motion_status(status);
    })
}

fn capabilities_listener(input: Rc<RefCell<InputHub>>) -> Closure<dyn FnMut(CustomEvent)> {
    Closure::new(move |event: CustomEvent| {
        let capabilities = event.detail().as_string().unwrap_or_default();
        input.borrow_mut().set_capabilities(
            capabilities.starts_with("touch"),
            capabilities.ends_with("motion"),
        );
    })
}

fn start_listener(input: Rc<RefCell<InputHub>>) -> Closure<dyn FnMut(Event)> {
    Closure::new(move |_event: Event| input.borrow_mut().request_start())
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
        input.set_pointer(pointer_target(&terminal, &event), pointer_kind(&event));
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
            .set_pointer(pointer_target(&terminal, &event), pointer_kind(&event));
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

fn pointer_kind(event: &PointerEvent) -> PointerKind {
    match event.pointer_type().as_str() {
        "touch" => PointerKind::Touch,
        "pen" => PointerKind::Pen,
        "mouse" => PointerKind::Mouse,
        _ => PointerKind::Unknown,
    }
}
