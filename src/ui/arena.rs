//! Perspective tunnel and particle rendering inside the bounded playfield.

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Widget},
};

use crate::{
    game::{Game, GamePhase},
    hazard::Gate,
    math::Vec2,
    ui::{BLUE, CORAL, CYAN, ICE, INK, MUTED, ORANGE, VOID},
};

const NAVY: Color = Color::Rgb(5, 17, 27);
const ORANGE_DIM: Color = Color::Rgb(123, 60, 27);
const SAND_LIGHT: Color = Color::Rgb(255, 214, 136);
const SAND_DARK: Color = Color::Rgb(176, 103, 55);

pub(super) struct Arena<'a> {
    game: &'a Game,
}

impl<'a> Arena<'a> {
    pub(super) const fn new(game: &'a Game) -> Self {
        Self { game }
    }
}

impl Widget for Arena<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        let border_color = match self.game.phase() {
            GamePhase::GameOver => ORANGE,
            GamePhase::Running if self.game.cleared_flash() => ICE,
            _ => CYAN,
        };
        let title = match self.game.phase() {
            GamePhase::Ready => "[ FEVER CHANNEL // IDLE ]",
            GamePhase::Running => "[ FEVER CHANNEL // LIVE ]",
            GamePhase::GameOver => "[ FEVER CHANNEL // LOST ]",
        };
        let frame = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(Style::new().fg(border_color))
            .title(Line::styled(title, Style::new().fg(border_color)));
        let inner = frame.inner(area);
        frame.render(area, buffer);
        fill(inner, buffer, " ", Style::new().bg(NAVY).fg(MUTED));

        let space = ArenaSpace::new(inner);
        draw_static(buffer, space, self.game.elapsed());
        draw_tunnel(buffer, space, self.game);
        if self.game.phase() != GamePhase::GameOver {
            for gate in self.game.gates().iter().rev() {
                draw_gate(buffer, space, *gate);
            }
        }
        draw_trail(buffer, space, self.game);
        if self.game.phase() != GamePhase::GameOver {
            draw_player(buffer, space, self.game);
        }
        draw_sand(buffer, space, self.game);
        draw_overlay(buffer, inner, self.game);
    }
}

#[derive(Clone, Copy)]
struct ArenaSpace {
    area: Rect,
}

impl ArenaSpace {
    const fn new(area: Rect) -> Self {
        Self { area }
    }

    fn to_cell(self, point: Vec2) -> Option<(u16, u16)> {
        if self.area.is_empty() {
            return None;
        }
        let x_span = self.area.width.saturating_sub(1) as f32;
        let y_span = self.area.height.saturating_sub(1) as f32;
        let x = self.area.x + (((point.x + 1.0) * 0.5 * x_span).round() as u16);
        let y = self.area.y + (((point.y + 1.0) * 0.5 * y_span).round() as u16);
        self.area.contains((x, y).into()).then_some((x, y))
    }

    fn cell_point(self, x: u16, y: u16) -> Vec2 {
        let x_span = self.area.width.saturating_sub(1).max(1) as f32;
        let y_span = self.area.height.saturating_sub(1).max(1) as f32;
        Vec2::new(
            ((x - self.area.x) as f32 / x_span).mul_add(2.0, -1.0),
            ((y - self.area.y) as f32 / y_span).mul_add(2.0, -1.0),
        )
    }
}

fn fill(area: Rect, buffer: &mut Buffer, symbol: &str, style: Style) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buffer[(x, y)].set_symbol(symbol).set_style(style);
        }
    }
}

fn draw_static(buffer: &mut Buffer, space: ArenaSpace, elapsed: f32) {
    let time_slice = (elapsed * 2.0) as u32;
    for y in space.area.top()..space.area.bottom() {
        for x in space.area.left()..space.area.right() {
            let hash = u32::from(x)
                .wrapping_mul(73_856_093)
                .wrapping_add(u32::from(y).wrapping_mul(19_349_663))
                .wrapping_add(time_slice.wrapping_mul(83_492_791));
            if hash % 173 == 0 {
                buffer[(x, y)]
                    .set_symbol("·")
                    .set_fg(Color::Rgb(36, 75, 94));
            }
        }
    }
}

fn draw_tunnel(buffer: &mut Buffer, space: ArenaSpace, game: &Game) {
    let drift = game.player().position * -0.055;
    for index in 0..6 {
        let travel = (game.elapsed() * game.speed() * 1.7 + index as f32 / 6.0).fract();
        let scale = 0.14 + travel.powf(1.32) * 0.84;
        let center = drift * (1.0 - scale);
        let color = mix(Color::Rgb(16, 51, 75), BLUE, travel);
        draw_outline(buffer, space, center, Vec2::new(scale, scale), color, "·");
    }

    if let Some((center_x, center_y)) = space.to_cell(drift) {
        buffer[(center_x, center_y)]
            .set_symbol("+")
            .set_fg(Color::Rgb(43, 91, 110));
    }
}

fn draw_gate(buffer: &mut Buffer, space: ArenaSpace, gate: Gate) {
    if gate.depth < -0.1 || gate.depth > 1.32 {
        return;
    }
    let travel = ((1.08 - gate.depth) / 1.08).clamp(0.0, 1.0);
    let outer = 0.13 + travel.powf(1.38) * 0.93;
    let opening_center = gate.opening * outer;
    let opening_half = gate.kind.half_size() * outer;
    let cell_x = 2.0 / space.area.width.max(1) as f32;
    let cell_y = 2.0 / space.area.height.max(1) as f32;
    let color = mix(ORANGE_DIM, ORANGE, travel);
    let edge_color = mix(ORANGE, SAND_LIGHT, travel);
    let fill_symbol = if travel > 0.82 {
        "▓"
    } else if travel > 0.55 {
        "▒"
    } else {
        "░"
    };

    for y in space.area.top()..space.area.bottom() {
        for x in space.area.left()..space.area.right() {
            let point = space.cell_point(x, y);
            let inside_outer = point.x.abs() <= outer && point.y.abs() <= outer;
            if !inside_outer {
                continue;
            }
            let relative = point - opening_center;
            let inside_opening =
                relative.x.abs() < opening_half.x && relative.y.abs() < opening_half.y;
            if inside_opening {
                continue;
            }
            let at_opening_edge = relative.x.abs() <= opening_half.x + cell_x * 1.2
                && relative.y.abs() <= opening_half.y + cell_y * 1.2;
            let at_outer_edge =
                (point.x.abs() - outer).abs() <= cell_x || (point.y.abs() - outer).abs() <= cell_y;
            let cell = &mut buffer[(x, y)];
            if at_opening_edge || at_outer_edge {
                cell.set_symbol("█").set_fg(edge_color);
            } else {
                cell.set_symbol(fill_symbol).set_fg(color);
            }
            cell.set_bg(NAVY);
        }
    }
}

fn draw_outline(
    buffer: &mut Buffer,
    space: ArenaSpace,
    center: Vec2,
    half_size: Vec2,
    color: Color,
    symbol: &str,
) {
    let Some((left, top)) = space.to_cell(center - half_size) else {
        return;
    };
    let Some((right, bottom)) = space.to_cell(center + half_size) else {
        return;
    };
    for x in left..=right {
        buffer[(x, top)].set_symbol(symbol).set_fg(color);
        buffer[(x, bottom)].set_symbol(symbol).set_fg(color);
    }
    for y in top..=bottom {
        buffer[(left, y)].set_symbol(symbol).set_fg(color);
        buffer[(right, y)].set_symbol(symbol).set_fg(color);
    }
}

fn draw_trail(buffer: &mut Buffer, space: ArenaSpace, game: &Game) {
    let count = game.trail().len().max(1) as f32;
    for (index, point) in game.trail().iter().enumerate().rev() {
        let Some((x, y)) = space.to_cell(*point) else {
            continue;
        };
        let strength = 1.0 - index as f32 / count;
        let symbol = if strength > 0.72 { "•" } else { "·" };
        buffer[(x, y)]
            .set_symbol(symbol)
            .set_fg(mix(BLUE, ICE, strength));
    }
}

fn draw_player(buffer: &mut Buffer, space: ArenaSpace, game: &Game) {
    let Some((x, y)) = space.to_cell(game.player().position) else {
        return;
    };
    let glyphs = ["◐", "◓", "◑", "◒"];
    let index = ((game.player().spin / std::f32::consts::FRAC_PI_2) as usize) % glyphs.len();
    let cell = &mut buffer[(x, y)];
    cell.set_symbol(glyphs[index])
        .set_fg(Color::White)
        .set_bg(Color::Rgb(8, 55, 68))
        .set_style(
            Style::new()
                .fg(Color::White)
                .bg(Color::Rgb(8, 55, 68))
                .bold(),
        );

    for offset in [(-1_i16, 0_i16), (1, 0), (0, -1), (0, 1)] {
        let glow_x = x.checked_add_signed(offset.0);
        let glow_y = y.checked_add_signed(offset.1);
        if let (Some(glow_x), Some(glow_y)) = (glow_x, glow_y)
            && space.area.contains((glow_x, glow_y).into())
            && buffer[(glow_x, glow_y)].symbol() == " "
        {
            buffer[(glow_x, glow_y)].set_symbol("·").set_fg(CYAN);
        }
    }
}

fn draw_sand(buffer: &mut Buffer, space: ArenaSpace, game: &Game) {
    let Some(sand) = game.sand() else {
        return;
    };
    for grain in sand.grains() {
        let Some((x, y)) = space.to_cell(grain.position) else {
            continue;
        };
        let cell = &mut buffer[(x, y)];
        let symbol = match cell.symbol() {
            "·" => "∙",
            "∙" => "•",
            "•" => "░",
            "░" => "▒",
            "▒" => "▓",
            "▓" | "█" => "█",
            _ => "·",
        };
        cell.set_symbol(symbol)
            .set_fg(mix(SAND_DARK, SAND_LIGHT, grain.tone));
    }
}

fn draw_overlay(buffer: &mut Buffer, area: Rect, game: &Game) {
    match game.phase() {
        GamePhase::Ready => draw_ready(buffer, area),
        GamePhase::Running => {}
        GamePhase::GameOver => draw_game_over(buffer, area, game),
    }
}

fn draw_ready(buffer: &mut Buffer, area: Rect) {
    let height = area.height.min(11);
    let width = area.width.saturating_sub(2).min(58);
    let overlay = centered(area, width, height);
    Clear.render(overlay, buffer);
    let compact = overlay.height < 10 || overlay.width < 42;
    let lines = if compact {
        vec![
            Line::styled("L I P T O I", Style::new().fg(CYAN).bold()),
            Line::styled("tilt · dodge · dissolve", Style::new().fg(INK)),
            Line::styled("", Style::default()),
            Line::styled("ARM MOTION BELOW", Style::new().fg(ORANGE).bold()),
            Line::styled("or press SPACE", Style::new().fg(MUTED)),
        ]
    } else {
        vec![
            Line::styled("╻  ╻┏━┓╺┳╸┏━┓╻", Style::new().fg(CYAN).bold()),
            Line::styled("┃  ┃┣━┛ ┃ ┃ ┃┃", Style::new().fg(ICE).bold()),
            Line::styled("┗━╸╹╹   ╹ ┗━┛╹", Style::new().fg(CYAN).bold()),
            Line::styled("", Style::default()),
            Line::styled("A SLOW-BURN TILT DODGER", Style::new().fg(INK)),
            Line::styled(
                "find the opening before the wall arrives",
                Style::new().fg(MUTED),
            ),
            Line::styled("", Style::default()),
            Line::styled(
                "[ ARM MOTION BELOW // OR PRESS SPACE ]",
                Style::new().fg(ORANGE).bold(),
            ),
        ]
    };
    Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(
            Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::new().fg(BLUE))
                .style(Style::new().bg(VOID)),
        )
        .render(overlay, buffer);
}

fn draw_game_over(buffer: &mut Buffer, area: Rect, game: &Game) {
    let width = area.width.saturating_sub(2).min(56);
    let height = if game.phase_time() < 0.48 { 5 } else { 10 }.min(area.height);
    let overlay = centered(area, width, height);
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
            Line::styled("", Style::default()),
            Line::styled(
                format!("SCORE {:05}  //  GATES {:03}", game.score(), game.cleared()),
                Style::new().fg(INK),
            ),
            Line::styled(
                format!("SURVIVED {:05.1}s", game.elapsed()),
                Style::new().fg(ICE),
            ),
            Line::styled("", Style::default()),
            Line::styled(
                "TILT TO POUR WHAT REMAINS",
                Style::new().fg(SAND_LIGHT).bold(),
            ),
            Line::styled("tap or press SPACE to reform", Style::new().fg(MUTED)),
        ]
    };
    Paragraph::new(lines)
        .alignment(Alignment::Center)
        .block(
            Block::new()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::new().fg(ORANGE))
                .style(Style::new().bg(VOID)),
        )
        .render(overlay, buffer);
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width.min(area.width),
        height.min(area.height),
    )
}

fn mix(from: Color, to: Color, amount: f32) -> Color {
    let (Color::Rgb(from_r, from_g, from_b), Color::Rgb(to_r, to_g, to_b)) = (from, to) else {
        return to;
    };
    let amount = amount.clamp(0.0, 1.0);
    let channel = |left: u8, right: u8| {
        (f32::from(left) + (f32::from(right) - f32::from(left)) * amount).round() as u8
    };
    Color::Rgb(
        channel(from_r, to_r),
        channel(from_g, to_g),
        channel(from_b, to_b),
    )
}
