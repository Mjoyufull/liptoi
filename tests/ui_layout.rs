use liptoi::{
    game::{Game, GamePhase},
    input::{ControlFrame, ControlSource, MotionStatus, PointerKind},
    math::Vec2,
    ui::{self, UiTelemetry},
};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

fn render(game: &Game, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test backend dimensions are valid");
    let telemetry = UiTelemetry {
        motion_status: MotionStatus::Active,
        source: ControlSource::Tilt,
        tilt: Vec2::new(0.24, -0.36),
        pointer_kind: PointerKind::Touch,
        touch_capable: true,
        motion_capable: true,
    };
    terminal
        .draw(|frame| ui::render(frame, game, telemetry))
        .expect("test backend render should succeed");
    buffer_text(terminal.backend().buffer())
}

fn buffer_text(buffer: &Buffer) -> String {
    let area = buffer.area;
    let mut output = String::new();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            output.push_str(buffer[(x, y)].symbol());
        }
        output.push('\n');
    }
    output
}

#[test]
fn desktop_layout_shows_playfield_and_both_side_panels() {
    let output = render(&Game::new(42), 132, 44);

    assert!(output.contains("LIPTOI"));
    assert!(output.contains("TELEMETRY"));
    assert!(output.contains("APPROACH"));
    assert!(output.contains("SLOW-BURN TILT DODGER"));
}

#[test]
fn phone_layout_keeps_the_start_prompt_inside_the_viewport() {
    let output = render(&Game::new(42), 46, 30);

    assert!(output.contains("LIPTOI"));
    assert!(output.contains("ENABLE TILT · OR TAP TO START"));
    assert!(output.contains("LOCAL SENSOR ONLY"));
    assert!(!output.contains("TELEMETRY"));
}

#[test]
fn undersized_viewport_provides_actionable_fallback() {
    let output = render(&Game::new(42), 28, 10);

    assert!(output.contains("rotate or enlarge"));
}

#[test]
fn game_over_layout_preserves_the_sand_interaction_prompt() {
    let mut game = Game::new(7);
    game.trigger_start();
    let controls = ControlFrame {
        pointer_target: Some(Vec2::new(1.0, 1.0)),
        source: ControlSource::Pointer,
        ..ControlFrame::default()
    };
    for _ in 0..2_000 {
        game.update(1.0 / 60.0, controls);
        if game.phase() == GamePhase::GameOver && game.phase_time() > 0.6 {
            break;
        }
    }

    assert_eq!(game.phase(), GamePhase::GameOver);
    let output = render(&game, 100, 38);
    assert!(output.contains("S I G N A L   L O S T"));
    assert!(output.contains("TILT TO POUR WHAT REMAINS"));
}

#[test]
fn supported_responsive_sizes_render_without_panics() {
    let game = Game::new(19);
    for (width, height) in [(30, 14), (36, 20), (48, 28), (80, 30), (120, 42), (180, 56)] {
        let output = render(&game, width, height);
        assert!(!output.is_empty());
    }
}
