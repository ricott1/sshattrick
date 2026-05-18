use crate::constants::*;
use crate::{
    big_text::{blue_scored, blue_won, dots, draw, red_scored, red_won, BigNumberFont},
    game::{Game, GameState},
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

pub fn render(frame: &mut Frame, game: &Game) -> AppResult<()> {
    let split = Layout::vertical([Constraint::Length(7), Constraint::Min(1)]).split(frame.area());

    let paragraph = Paragraph::new(img_to_lines(&game.image()?));
    frame.render_widget(paragraph, split[1]);

    let top_split = Layout::horizontal([
        Constraint::Length(20),
        Constraint::Length(43),
        Constraint::Length(34),
        Constraint::Length(43),
        Constraint::Length(20),
    ])
    .split(split[0]);

    let red_score_paragraph = game
        .red_data
        .score
        .big_font_styled(Color::Red, Color::Yellow);

    let horizontal = if game.red_data.score < 10 { 5 } else { 1 };
    frame.render_widget(
        red_score_paragraph,
        top_split[0].inner(Margin {
            horizontal,
            vertical: 0,
        }),
    );

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("Saves {}", game.red_data.goalie.saves)),
            Line::from("← ↑ → ↓: move"),
            Line::from("space: shoot"),
            Line::from("Esc: close game"),
        ])
        .centered(),
        top_split[1],
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("Saves {}", game.blue_data.goalie.saves)),
            Line::from("← ↑ → ↓: move"),
            Line::from("space: shoot"),
            Line::from("Esc: close game"),
        ])
        .centered(),
        top_split[3],
    );

    let blue_score_paragraph = game
        .blue_data
        .score
        .big_font_styled(Color::Blue, Color::LightMagenta);
    let horizontal = if game.blue_data.score < 10 { 5 } else { 1 };
    frame.render_widget(
        blue_score_paragraph,
        top_split[4].inner(Margin {
            horizontal,
            vertical: 0,
        }),
    );

    let timer_split = Layout::horizontal([
        Constraint::Length(10),
        Constraint::Length(4),
        Constraint::Length(10),
        Constraint::Length(10),
    ])
    .split(top_split[2]);

    let (color_1, color_2) = match game.palette {
        Palette::Dark => (Color::Cyan, Color::White),
        Palette::Light => (Color::DarkGray, Color::Gray),
        Palette::Basket => (Color::Magenta, Color::LightMagenta),
        Palette::Alt => (Color::Green, Color::Red),
    };

    let timer = (Game::DURATION_MILLISECONDS.saturating_sub(game.timer)) / 1000;
    let minutes_paragraph = ((timer / 60) as u8).big_font_styled(color_1, color_2);
    let seconds_tens_paragraph = (((timer % 60) / 10) as u8).big_font_styled(color_1, color_2);
    let seconds_units_paragraph = (((timer % 60) % 10) as u8).big_font_styled(color_1, color_2);

    frame.render_widget(minutes_paragraph, timer_split[0]);
    frame.render_widget(dots(color_1, color_2), timer_split[1]);
    frame.render_widget(seconds_tens_paragraph, timer_split[2]);
    frame.render_widget(seconds_units_paragraph, timer_split[3]);

    match game.state {
        GameState::Starting { time } => {
            let rect = Rect::new(
                (MIN_X + MAX_X) as u16 / 2 - 5,
                (MIN_Y + MAX_Y) as u16 / 4 + 5,
                10,
                10,
            );
            let elapsed = time.elapsed().as_millis() as u64;

            let countdown_paragraph = if Game::STARTING_DELAY_MILLISECONDS > elapsed {
                (((Game::STARTING_DELAY_MILLISECONDS - elapsed) / 1000) as u8 + 1)
                    .big_font_styled(color_1, color_2)
            } else {
                Paragraph::new("")
            };

            frame.render_widget(countdown_paragraph, rect);
        }
        GameState::AfterGoal { time: _, scored } => {
            let rect = Rect::new(
                (MIN_X + MAX_X) as u16 / 2 - 44,
                (MIN_Y + MAX_Y) as u16 / 4 + 5,
                88,
                10,
            );
            let scored = if scored == GameSide::Red {
                red_scored(color_1, color_2)
            } else {
                blue_scored(color_1, color_2)
            };
            frame.render_widget(scored, rect);
        }
        GameState::Ending { .. } => {
            let rect = Rect::new(
                (MIN_X + MAX_X) as u16 / 2 - 36,
                (MIN_Y + MAX_Y) as u16 / 4 + 5,
                72,
                10,
            );
            let congrats = if game.red_data.score > game.blue_data.score {
                red_won(color_1, color_2)
            } else if game.blue_data.score > game.red_data.score {
                blue_won(color_1, color_2)
            } else {
                draw(color_1, color_2)
            };
            frame.render_widget(congrats, rect);
        }
        _ => {}
    }

    Ok(())
}
