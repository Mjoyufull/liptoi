//! Responsive Ratatui composition for the browser terminal.

mod arena;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::{
    game::{Game, GamePhase},
    input::{ControlSource, InputHub, MotionStatus},
    math::Vec2,
};

use arena::Arena;

pub(crate) const VOID: Color = Color::Rgb(3, 7, 12);
pub(crate) const INK: Color = Color::Rgb(206, 231, 241);
pub(crate) const MUTED: Color = Color::Rgb(72, 103, 124);
pub(crate) const ICE: Color = Color::Rgb(125, 211, 252);
pub(crate) const CYAN: Color = Color::Rgb(34, 211, 238);
pub(crate) const BLUE: Color = Color::Rgb(30, 89, 126);
pub(crate) const ORANGE: Color = Color::Rgb(251, 146, 60);
pub(crate) const CORAL: Color = Color::Rgb(251, 113, 133);

/// Browser input telemetry displayed alongside the playfield.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UiTelemetry {
    /// Motion permission and availability.
    pub motion_status: MotionStatus,
    /// Last active steering source.
    pub source: ControlSource,
    /// Filtered tilt vector.
    pub tilt: Vec2,
}

impl From<&InputHub> for UiTelemetry {
    fn from(input: &InputHub) -> Self {
        Self {
            motion_status: input.motion_status(),
            source: input.source(),
            tilt: input.tilt(),
        }
    }
}

/// Draws the complete game at any browser terminal size.
pub fn render(frame: &mut Frame<'_>, game: &Game, telemetry: UiTelemetry) {
    let area = frame.area();
    frame.render_widget(Block::new().style(Style::new().bg(VOID)), area);
    if area.width < 30 || area.height < 14 {
        render_too_small(frame, area);
        return;
    }

    let shell = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::new().fg(BLUE))
        .title(Line::from(vec![
            Span::styled(" LIPTOI ", Style::new().fg(CYAN).bold()),
            Span::styled("// TILT THROUGH THE FEVER ", Style::new().fg(MUTED)),
        ]))
        .title_bottom(Line::styled(
            " RATATUI × RATZILLA // LOCAL SENSOR SIGNAL ONLY ",
            Style::new().fg(MUTED),
        ));
    let inner = shell.inner(area).inner(Margin::new(1, 0));
    frame.render_widget(shell, area);

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(2),
    ])
    .areas(inner);
    render_header(frame, header, game, telemetry);
    render_body(frame, body, game, telemetry);
    render_footer(frame, footer, game, telemetry);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, game: &Game, telemetry: UiTelemetry) {
    let phase = match game.phase() {
        GamePhase::Ready => "AWAITING SIGNAL",
        GamePhase::Running => "RUN ACTIVE",
        GamePhase::GameOver => "PARTICULATE STATE",
    };
    let phase_color = match game.phase() {
        GamePhase::Ready => ICE,
        GamePhase::Running if game.cleared_flash() => CYAN,
        GamePhase::Running => INK,
        GamePhase::GameOver => ORANGE,
    };

    let score = format!("SCORE {:05}", game.score());
    let best = format!("BEST {:05}", game.best_score());
    let status = Line::from(vec![
        Span::styled(format!(" {phase} "), Style::new().fg(phase_color).bold()),
        Span::styled("│ ", Style::new().fg(BLUE)),
        Span::styled(telemetry.motion_status.label(), Style::new().fg(CYAN)),
        Span::styled("  ", Style::default()),
        Span::styled(score, Style::new().fg(INK).bold()),
        Span::styled("  ", Style::default()),
        Span::styled(best, Style::new().fg(MUTED)),
    ]);

    frame.render_widget(
        Paragraph::new(status).block(
            Block::new()
                .borders(Borders::BOTTOM)
                .border_style(Style::new().fg(BLUE)),
        ),
        area,
    );
}

fn render_body(frame: &mut Frame<'_>, area: Rect, game: &Game, telemetry: UiTelemetry) {
    let arena_area = arena_rect(area);
    frame.render_widget(Arena::new(game), arena_area);

    let left_width = arena_area.x.saturating_sub(area.x + 1);
    if left_width >= 18 {
        let left = Rect::new(area.x, arena_area.y, left_width, arena_area.height);
        render_telemetry(frame, left, game, telemetry);
    }

    let arena_right = arena_area.x + arena_area.width;
    let body_right = area.x + area.width;
    let right_width = body_right.saturating_sub(arena_right + 1);
    if right_width >= 18 {
        let right = Rect::new(
            arena_right + 1,
            arena_area.y,
            right_width,
            arena_area.height,
        );
        render_next_gate(frame, right, game);
    }
}

fn arena_rect(area: Rect) -> Rect {
    let maximum_height = area.height.min(34);
    let maximum_width = area.width.min(82);
    let width_from_height = maximum_height.saturating_sub(2).saturating_mul(2) + 2;
    let width = maximum_width.min(width_from_height).max(24);
    let height_from_width = width.saturating_sub(2) / 2 + 2;
    let height = maximum_height.min(height_from_width).max(10);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width.min(area.width),
        height.min(area.height),
    )
}

fn render_telemetry(frame: &mut Frame<'_>, area: Rect, game: &Game, telemetry: UiTelemetry) {
    let velocity = game.player().velocity;
    let text = vec![
        Line::styled("", Style::default()),
        Line::styled("CONTROL", Style::new().fg(MUTED)),
        Line::styled(telemetry.source.label(), Style::new().fg(CYAN).bold()),
        Line::styled("", Style::default()),
        Line::styled("TILT VECTOR", Style::new().fg(MUTED)),
        meter_line("X", telemetry.tilt.x),
        meter_line("Y", telemetry.tilt.y),
        Line::styled("", Style::default()),
        Line::styled("SPHERE VEL", Style::new().fg(MUTED)),
        Line::styled(format!("x {:+.2}", velocity.x), Style::new().fg(INK)),
        Line::styled(format!("y {:+.2}", velocity.y), Style::new().fg(INK)),
        Line::styled("", Style::default()),
        Line::styled(
            format!("TIME {:05.1}s", game.elapsed()),
            Style::new().fg(ICE),
        ),
    ];
    frame.render_widget(side_panel(" TELEMETRY ", text), area);
}

fn meter_line(axis: &str, value: f32) -> Line<'static> {
    let level = ((value.abs() * 5.0).round() as usize).min(5);
    let meter = format!("{}{}", "▰".repeat(level), "▱".repeat(5 - level));
    Line::from(vec![
        Span::styled(format!("{axis} "), Style::new().fg(MUTED)),
        Span::styled(
            meter,
            Style::new().fg(if value.abs() > 0.72 { ORANGE } else { ICE }),
        ),
    ])
}

fn render_next_gate(frame: &mut Frame<'_>, area: Rect, game: &Game) {
    let mut text = vec![Line::styled("", Style::default())];
    if let Some(gate) = game.next_gate() {
        let distance =
            ((gate.depth - crate::hazard::IMPACT_DEPTH).max(0.0) / game.speed()).max(0.0);
        let half_size = gate.kind.half_size();
        text.extend([
            Line::styled("NEXT GATE", Style::new().fg(MUTED)),
            Line::styled(gate.kind.label(), Style::new().fg(ORANGE).bold()),
            Line::styled(format!("ETA {distance:04.1}s"), Style::new().fg(INK)),
            Line::styled("", Style::default()),
            Line::styled("SAFE CENTER", Style::new().fg(MUTED)),
            Line::styled(format!("x {:+.2}", gate.opening.x), Style::new().fg(ICE)),
            Line::styled(format!("y {:+.2}", gate.opening.y), Style::new().fg(ICE)),
            Line::styled("", Style::default()),
            Line::styled("CLEARANCE", Style::new().fg(MUTED)),
            Line::styled(
                format!("{:.0} × {:.0}", half_size.x * 200.0, half_size.y * 200.0),
                Style::new().fg(INK),
            ),
            Line::styled("", Style::default()),
            Line::styled(
                format!("CLEARED {:03}", game.cleared()),
                Style::new().fg(CYAN).bold(),
            ),
        ]);
    } else {
        text.push(Line::styled("SCANNING…", Style::new().fg(MUTED)));
    }
    frame.render_widget(side_panel(" APPROACH ", text), area);
}

fn side_panel(title: &'static str, text: Vec<Line<'static>>) -> Paragraph<'static> {
    Paragraph::new(text).block(
        Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::new().fg(BLUE))
            .title(Line::styled(title, Style::new().fg(MUTED))),
    )
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, game: &Game, telemetry: UiTelemetry) {
    let prompt = match game.phase() {
        GamePhase::Ready => "ARM MOTION BELOW · SPACE/ENTER TO START",
        GamePhase::Running => "TILT / WASD / ARROWS / DRAG  ·  R RECENTERS",
        GamePhase::GameOver if game.phase_time() < 0.65 => "IMPACT // SPHERE DECOHERING",
        GamePhase::GameOver => "TILT TO POUR THE SAND  ·  TAP/SPACE TO REFORM",
    };
    let line = Line::from(vec![
        Span::styled(" CONTROL ", Style::new().fg(MUTED)),
        Span::styled(prompt, Style::new().fg(INK)),
        Span::styled(
            format!("  [{}] ", telemetry.source.label()),
            Style::new().fg(CYAN),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(line).alignment(Alignment::Center).block(
            Block::new()
                .borders(Borders::TOP)
                .border_style(Style::new().fg(BLUE)),
        ),
        area,
    );
}

fn render_too_small(frame: &mut Frame<'_>, area: Rect) {
    let message = Paragraph::new(vec![
        Line::styled("LIPTOI", Style::new().fg(CYAN).bold()),
        Line::styled("rotate or enlarge the viewport", Style::new().fg(INK)),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::new()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(BLUE)),
    );
    frame.render_widget(message, area);
}
