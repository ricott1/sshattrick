use crate::constants::*;
use crate::{
    big_text::{blue_scored, blue_won, disconnection, dots, draw, red_scored, red_won, BigNumberFont},
    game::{Game, GameData, GameState},
    types::{GameSide, Palette},
};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Clear, Paragraph},
    Frame,
};

const CONTROLS_LINES: [&str; 3] = ["← ↑ → ↓: move", "space: shoot", "Esc: close game"];

const DISCONNECT_BANNER_WIDTH: u16 = 102;
const DISCONNECT_BANNER_HEIGHT: u16 = 6;
const DISCONNECT_BANNER_Y_OFFSET: u16 = 8;
const WIN_BANNER_WIDTH: u16 = 72;
const WIN_BANNER_HEIGHT: u16 = 6;

fn render_side_panel(frame: &mut Frame, area: Rect, data: &GameData, show_controls: bool) {
    let mut lines = vec![Line::from(format!("Saves {}", data.goalie.saves))];
    if show_controls {
        lines.extend(CONTROLS_LINES.iter().map(|s| Line::from(*s)));
    }
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

pub fn render(frame: &mut Frame, game: &Game, image_lines: &[Line], viewer: GameSide) {
    let split = Layout::vertical([Constraint::Length(7), Constraint::Min(1)]).split(frame.area());
    frame.render_widget(Paragraph::new(image_lines.to_vec()), split[1]);

    let top_split = Layout::horizontal([
        Constraint::Length(20),
        Constraint::Length(43),
        Constraint::Length(34),
        Constraint::Length(43),
        Constraint::Length(20),
    ])
    .split(split[0]);

    render_score(frame, top_split[0], game.red_data.score, Color::Red, Color::Yellow);
    render_score(frame, top_split[4], game.blue_data.score, Color::Blue, Color::LightMagenta);
    for (panel_area, data, side) in [
        (top_split[1], &game.red_data, GameSide::Red),
        (top_split[3], &game.blue_data, GameSide::Blue),
    ] {
        render_side_panel(frame, panel_area, data, viewer == side);
    }

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
        GameState::Ending {
            winner,
            by_disconnect,
            ..
        } => {
            if by_disconnect {
                let rect = Rect::new(
                    center_x - DISCONNECT_BANNER_WIDTH / 2,
                    center_y.saturating_sub(DISCONNECT_BANNER_Y_OFFSET),
                    DISCONNECT_BANNER_WIDTH,
                    DISCONNECT_BANNER_HEIGHT,
                );
                frame.render_widget(disconnection(color_1, color_2), rect);
            }
            let rect = Rect::new(
                center_x - WIN_BANNER_WIDTH / 2,
                center_y,
                WIN_BANNER_WIDTH,
                WIN_BANNER_HEIGHT,
            );
            let widget = match winner {
                Some(GameSide::Red) => red_won(color_1, color_2),
                Some(GameSide::Blue) => blue_won(color_1, color_2),
                None => draw(color_1, color_2),
            };
            frame.render_widget(widget, rect);
        }
        GameState::Running => {}
    }
}

fn palette_colors(palette: Palette) -> (Color, Color) {
    match palette {
        Palette::Dark => (Color::Cyan, Color::White),
        Palette::Light => (Color::DarkGray, Color::Gray),
        Palette::Basket => (Color::Magenta, Color::LightMagenta),
        Palette::Alt => (Color::Green, Color::Red),
    }
}

pub fn render_lobby(frame: &mut Frame, username: &str, games_played: usize, games_won: usize) {
    let area = frame.area();
    frame.render_widget(Clear, area);

    let lines = vec![
        Line::raw(""),
        Line::raw(""),
        Line::raw(""),
        Line::styled("ssHattrick", Style::new().cyan().bold()),
        Line::raw(""),
        Line::raw("Waiting for opponent..."),
        Line::raw(""),
        Line::raw(""),
        Line::from(format!("Player:        {username}")),
        Line::from(format!("Games played:  {games_played}")),
        Line::from(format!("Games won:     {games_won}")),
        Line::raw(""),
        Line::raw(""),
        Line::styled("press Esc / Ctrl+C to leave", Style::new().dim()),
    ];

    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}
