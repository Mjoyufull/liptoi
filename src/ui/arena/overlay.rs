//! Ready and game-over panels rendered over the tunnel.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
};

use crate::{
    game::{Game, GamePhase},
    input::MotionStatus,
    ui::{BLUE, CORAL, CYAN, ICE, INK, MUTED, ORANGE, UiTelemetry, VOID},
};

use super::SAND_LIGHT;

pub(super) fn draw(buffer: &mut Buffer, area: Rect, game: &Game, telemetry: UiTelemetry) {
    match game.phase() {
        GamePhase::Ready => draw_ready(buffer, area, telemetry),
        GamePhase::Running => {}
        GamePhase::GameOver => draw_game_over(buffer, area, game, telemetry),
    }
}

fn draw_ready(buffer: &mut Buffer, area: Rect, telemetry: UiTelemetry) {
    let overlay = centered(
        area,
        area.width.saturating_sub(2).min(58),
        area.height.min(11),
    );
    Clear.render(overlay, buffer);
    let compact = overlay.height < 10 || overlay.width < 42;
    let lines = if compact {
        vec![
            Line::styled("L I P T O I", Style::new().fg(CYAN).bold()),
            Line::styled("tilt · dodge · dissolve", Style::new().fg(INK)),
            Line::default(),
            Line::styled(ready_control(telemetry), Style::new().fg(ORANGE).bold()),
            Line::styled(ready_fallback(telemetry), Style::new().fg(MUTED)),
        ]
    } else {
        vec![
            Line::styled("╻  ╻┏━┓╺┳╸┏━┓╻", Style::new().fg(CYAN).bold()),
            Line::styled("┃  ┃┣━┛ ┃ ┃ ┃┃", Style::new().fg(ICE).bold()),
            Line::styled("┗━╸╹╹   ╹ ┗━┛╹", Style::new().fg(CYAN).bold()),
            Line::default(),
            Line::styled("A SLOW-BURN TILT DODGER", Style::new().fg(INK)),
            Line::styled(
                "find the opening before the wall arrives",
                Style::new().fg(MUTED),
            ),
            Line::default(),
            Line::styled(
                format!(
                    "[ {} // {} ]",
                    ready_control(telemetry),
                    ready_fallback(telemetry)
                ),
                Style::new().fg(ORANGE).bold(),
            ),
        ]
    };
    panel(lines, BLUE).render(overlay, buffer);
}

fn ready_control(telemetry: UiTelemetry) -> &'static str {
    if !telemetry.touch_capable {
        "MOUSE + KEYS READY"
    } else if telemetry.motion_status == MotionStatus::Active {
        "TILT ONLINE"
    } else if telemetry.motion_capable {
        "ENABLE TILT BELOW"
    } else {
        "TOUCH + KEYS READY"
    }
}

fn ready_fallback(telemetry: UiTelemetry) -> &'static str {
    if telemetry.touch_capable {
        "TAP OR PRESS SPACE"
    } else {
        "CLICK OR PRESS SPACE"
    }
}

fn draw_game_over(buffer: &mut Buffer, area: Rect, game: &Game, telemetry: UiTelemetry) {
    let height = if game.phase_time() < 0.48 { 5 } else { 10 };
    let overlay = centered(
        area,
        area.width.saturating_sub(2).min(56),
        height.min(area.height),
    );
    Clear.render(overlay, buffer);
    let lines = if game.phase_time() < 0.48 {
        vec![
            Line::styled("I M P A C T", Style::new().fg(CORAL).bold()),
            Line::styled("sphere → particulate", Style::new().fg(ORANGE)),
        ]
    } else {
        vec![
            Line::styled("S I G N A L   L O S T", Style::new().fg(CORAL).bold()),
            Line::styled("the sphere became sediment", Style::new().fg(ORANGE)),
            Line::default(),
            Line::styled(
                format!("SCORE {:05}  //  GATES {:03}", game.score(), game.cleared()),
                Style::new().fg(INK),
            ),
            Line::styled(
                format!("SURVIVED {:05.1}s", game.elapsed()),
                Style::new().fg(ICE),
            ),
            Line::default(),
            Line::styled(
                game_over_control(telemetry),
                Style::new().fg(SAND_LIGHT).bold(),
            ),
            Line::styled(game_over_restart(telemetry), Style::new().fg(MUTED)),
        ]
    };
    panel(lines, ORANGE).render(overlay, buffer);
}

fn game_over_control(telemetry: UiTelemetry) -> &'static str {
    if telemetry.motion_status == MotionStatus::Active {
        "TILT TO POUR WHAT REMAINS"
    } else if telemetry.touch_capable {
        "DRAG TO POUR WHAT REMAINS"
    } else {
        "MOVE TO POUR WHAT REMAINS"
    }
}

fn game_over_restart(telemetry: UiTelemetry) -> &'static str {
    if telemetry.touch_capable {
        "tap or press SPACE to reform"
    } else {
        "click or press SPACE to reform"
    }
}

fn panel(lines: Vec<Line<'static>>, border_color: ratatui::style::Color) -> Paragraph<'static> {
    Paragraph::new(lines).alignment(Alignment::Center).block(
        Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::new().fg(border_color))
            .style(Style::new().bg(VOID)),
    )
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width.min(area.width),
        height.min(area.height),
    )
}
