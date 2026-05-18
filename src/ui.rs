use crate::constants::*;
use crate::{
    big_text::{blue_scored, blue_won, dots, draw, red_scored, red_won, BigNumberFont},
    game::{Game, GameData, GameState},
    types::{AppResult, GameSide, Palette},
    utils::img_to_lines,
};
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::Color,
    text::Line,
    widgets::Paragraph,
    Frame,
};

const CONTROLS_LINES: [&str; 3] = ["← ↑ → ↓: move", "space: shoot", "Esc: close game"];

fn render_side_panel(frame: &mut Frame, area: Rect, data: &GameData) {
    let mut lines = vec![Line::from(format!("Saves {}", data.goalie.saves))];
    lines.extend(CONTROLS_LINES.iter().map(|s| Line::from(*s)));
    frame.render_widget(Paragraph::new(lines).centered(), area);
}

fn render_score(frame: &mut Frame, area: Rect, score: u8, fg: Color, bg: Color) {
    let horizontal = if score < 10 { 5 } else { 1 };
    let inner = area.inner(Margin {
        horizontal,
        vertical: 0,
    });
    frame.render_widget(score.big_font_styled(fg, bg), inner);
}

pub fn render(frame: &mut Frame, game: &Game) -> AppResult<()> {
    let split = Layout::vertical([Constraint::Length(7), Constraint::Min(1)]).split(frame.area());
    frame.render_widget(Paragraph::new(img_to_lines(&game.image()?)), split[1]);

    let top_split = Layout::horizontal([
        Constraint::Length(20),
        Constraint::Length(43),
        Constraint::Length(34),
        Constraint::Length(43),
        Constraint::Length(20),
    ])
    .split(split[0]);

    render_score(frame, top_split[0], game.red_data.score, Color::Red, Color::Yellow);
    render_side_panel(frame, top_split[1], &game.red_data);
    render_side_panel(frame, top_split[3], &game.blue_data);
    render_score(frame, top_split[4], game.blue_data.score, Color::Blue, Color::LightMagenta);

    let timer_split = Layout::horizontal([
        Constraint::Length(10),
        Constraint::Length(4),
        Constraint::Length(10),
        Constraint::Length(10),
    ])
    .split(top_split[2]);

    let (color_1, color_2) = palette_colors(game.palette);

    let timer = (Game::DURATION_MILLISECONDS.saturating_sub(game.timer)) / 1000;
    frame.render_widget(
        ((timer / 60) as u8).big_font_styled(color_1, color_2),
        timer_split[0],
    );
    frame.render_widget(dots(color_1, color_2), timer_split[1]);
    frame.render_widget(
        (((timer % 60) / 10) as u8).big_font_styled(color_1, color_2),
        timer_split[2],
    );
    frame.render_widget(
        ((timer % 10) as u8).big_font_styled(color_1, color_2),
        timer_split[3],
    );

    let center_x = (MIN_X + MAX_X) / 2;
    let center_y = (MIN_Y + MAX_Y) / 4 + 5;

    match game.state {
        GameState::Starting { time } => {
            let elapsed = time.elapsed().as_millis() as u64;
            if let Some(remaining) = Game::STARTING_DELAY_MILLISECONDS.checked_sub(elapsed) {
                let rect = Rect::new(center_x - 5, center_y, 10, 10);
                let countdown = ((remaining / 1000) as u8 + 1).big_font_styled(color_1, color_2);
                frame.render_widget(countdown, rect);
            }
        }
        GameState::AfterGoal { scored, .. } => {
            let rect = Rect::new(center_x - 44, center_y, 88, 10);
            let widget = if scored == GameSide::Red {
                red_scored(color_1, color_2)
            } else {
                blue_scored(color_1, color_2)
            };
            frame.render_widget(widget, rect);
        }
        GameState::Ending { .. } => {
            let rect = Rect::new(center_x - 36, center_y, 72, 10);
            let widget = match game.red_data.score.cmp(&game.blue_data.score) {
                std::cmp::Ordering::Greater => red_won(color_1, color_2),
                std::cmp::Ordering::Less => blue_won(color_1, color_2),
                std::cmp::Ordering::Equal => draw(color_1, color_2),
            };
            frame.render_widget(widget, rect);
        }
        GameState::Running => {}
    }

    Ok(())
}

fn palette_colors(palette: Palette) -> (Color, Color) {
    match palette {
        Palette::Dark => (Color::Cyan, Color::White),
        Palette::Light => (Color::DarkGray, Color::Gray),
        Palette::Basket => (Color::Magenta, Color::LightMagenta),
        Palette::Alt => (Color::Green, Color::Red),
    }
}
