use crate::big_text::{
    blue_scored, blue_won, disconnection, dots, draw, practice, red_scored, red_won, sshattrick,
    BigNumberFont,
};
use crate::img_lines::PITCH_LINES;
use crate::lobby::{LobbyStats, LobbyView, FRIEND_CODE_LEN};
use sshattrick_core::constants::*;
use sshattrick_core::{Game, GameData, GameSide, GameState, Palette};
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
    Frame,
};

const CONTROLS_LINES: [&str; 3] = [
    "← ↑ → ↓: move",
    "space: shoot",
    "Esc: forfeit / back to lobby",
];
const PRACTICE_CONTROLS_LINES: [&str; 3] = [
    "← ↑ → ↓: move",
    "space: shoot",
    "Esc / Backspace: back to lobby",
];

const DISCONNECT_BANNER_WIDTH: u16 = 102;
const DISCONNECT_BANNER_HEIGHT: u16 = 6;
const DISCONNECT_BANNER_Y_OFFSET: u16 = 8;
const WIN_BANNER_WIDTH: u16 = 72;
const WIN_BANNER_HEIGHT: u16 = 6;
const LOBBY_OVERLAY_WIDTH: u16 = 60;
const LOBBY_OVERLAY_HEIGHT: u16 = 24;

fn render_side_panel(frame: &mut Frame, area: Rect, data: &GameData, controls: Option<&[&str]>) {
    let mut lines = vec![Line::from(format!("Saves {}", data.goalie.saves))];
    if let Some(c) = controls {
        lines.extend(c.iter().map(|s| Line::from(*s)));
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

pub fn render(
    frame: &mut Frame,
    game: &Game,
    image_lines: &[Line],
    viewer: GameSide,
    idle_warning: Option<u32>,
) {
    let split = Layout::vertical([Constraint::Length(6), Constraint::Fill(1)]).split(frame.area());
    frame.render_widget(Paragraph::new(image_lines.to_vec()), split[1]);

    let top_split = Layout::horizontal([
        Constraint::Length(20),
        Constraint::Length(43),
        Constraint::Length(34),
        Constraint::Length(43),
        Constraint::Length(20),
    ])
    .split(split[0]);

    let controls: &[&str] = if game.practice_mode {
        &PRACTICE_CONTROLS_LINES
    } else {
        &CONTROLS_LINES
    };

    render_score(
        frame,
        top_split[0],
        game.red_data.score,
        Color::Red,
        Color::Yellow,
    );
    if !game.practice_mode {
        render_score(
            frame,
            top_split[4],
            game.blue_data.score,
            Color::Blue,
            Color::LightMagenta,
        );
    }
    for (panel_area, data, side) in [
        (top_split[1], &game.red_data, GameSide::Red),
        (top_split[3], &game.blue_data, GameSide::Blue),
    ] {
        // No opponent panel in practice mode.
        if game.practice_mode && side != viewer {
            continue;
        }
        let panel_controls = if viewer == side { Some(controls) } else { None };
        render_side_panel(frame, panel_area, data, panel_controls);
    }

    let (color_1, color_2) = palette_colors(game.palette);

    if game.practice_mode {
        let banner = Rect {
            x: top_split[2].x,
            y: top_split[2].y,
            width: top_split[2].width + top_split[3].width,
            height: top_split[2].height,
        };
        frame.render_widget(practice(color_1, color_2), banner);
    } else {
        let timer_split = Layout::horizontal([
            Constraint::Length(10),
            Constraint::Length(4),
            Constraint::Length(10),
            Constraint::Length(10),
        ])
        .split(top_split[2]);

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
    }

    let center_x = (MIN_X + MAX_X) / 2;
    let center_y = (MIN_Y + MAX_Y) / 4 + 4;

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

    if let Some(secs) = idle_warning {
        let area = frame.area();
        let banner_w: u16 = 50;
        let banner_h: u16 = 3;
        let banner = Rect {
            x: area.x + area.width.saturating_sub(banner_w) / 2,
            y: area.y + area.height.saturating_sub(banner_h).saturating_sub(2),
            width: banner_w.min(area.width),
            height: banner_h.min(area.height),
        };
        frame.render_widget(Clear, banner);
        frame.render_widget(
            Paragraph::new(frittura_ssh_core::idle_warning_text(secs))
                .centered()
                .style(Style::new().red().bold())
                .block(ratatui::widgets::Block::bordered()),
            banner,
        );
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

pub fn render_lobby(
    frame: &mut Frame,
    username: &str,
    games_played: usize,
    games_won: usize,
    stats: &LobbyStats,
    view: LobbyView,
    idle_warning: Option<u32>,
) {
    let split = Layout::vertical([Constraint::Length(6), Constraint::Fill(1)]).split(frame.area());
    let (color_1, color_2) = palette_colors(Palette::Dark);
    frame.render_widget(sshattrick(color_1, color_2), split[0]);

    let pitch_area = split[1];
    let pitch_lines = PITCH_LINES
        .get(&Palette::Dark)
        .expect("Pitch lines should exist")
        .clone();
    frame.render_widget(Paragraph::new(pitch_lines), pitch_area);

    let overlay = Rect {
        x: pitch_area.x + pitch_area.width.saturating_sub(LOBBY_OVERLAY_WIDTH) / 2,
        y: pitch_area.y + pitch_area.height.saturating_sub(LOBBY_OVERLAY_HEIGHT) / 2,
        width: LOBBY_OVERLAY_WIDTH.min(pitch_area.width),
        height: LOBBY_OVERLAY_HEIGHT.min(pitch_area.height),
    };
    frame.render_widget(Clear, overlay);
    frame.render_widget(Block::bordered(), overlay);
    let area = overlay.inner(Margin::new(1, 1));

    let chunks = Layout::vertical([
        Constraint::Length(1), // 0: top pad
        Constraint::Length(1), // 1: username
        Constraint::Length(1), // 2: pad
        Constraint::Length(2), // 3: games played + won
        Constraint::Length(2), // 4: pad
        Constraint::Length(7), // 5: view-specific block (fixed)
        Constraint::Length(1), // 6: pad
        Constraint::Length(1), // 7: kick warning (empty unless within window)
        Constraint::Length(1), // 8: pad
        Constraint::Length(2), // 9: stats (connected + ongoing)
        Constraint::Fill(1),   // 10: bottom spacer
    ])
    .split(area);

    frame.render_widget(Line::from(username.to_string()).centered(), chunks[1]);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("Games played:  {games_played}")),
            Line::from(format!("Games won:     {games_won}")),
        ])
        .centered(),
        chunks[3],
    );

    let view_lines: Vec<Line<'_>> = match view {
        LobbyView::Idle => vec![
            Line::styled("Pick a mode:", Style::new().bold()),
            Line::raw(""),
            Line::styled("a: auto-match", Style::new().dim()),
            Line::styled("p: practice mode", Style::new().dim()),
            Line::styled("g: play with a friend (code)", Style::new().dim()),
            Line::raw(""),
            Line::styled("Esc / Backspace: leave", Style::new().dim()),
        ],
        LobbyView::AutoQueue => vec![
            Line::styled("Looking for an opponent...", Style::new().yellow()),
            Line::raw(""),
            Line::styled("Esc / Backspace: back to lobby", Style::new().dim()),
        ],
        LobbyView::ShowingCode {
            code,
            typed,
            last_attempt_failed,
        } => {
            let padded: String = typed
                .chars()
                .chain(std::iter::repeat('_'))
                .take(FRIEND_CODE_LEN)
                .collect();
            let error_line = if last_attempt_failed {
                Line::styled("no match found, try again", Style::new().red().dim())
            } else {
                Line::raw("")
            };
            // 15-char prefixes on both lines so the codes start at the same column.
            vec![
                Line::from(vec![
                    "Your code:     ".into(),
                    Span::styled(code.to_string(), Style::new().cyan().bold()),
                ]),
                Line::raw(""),
                Line::from(vec![
                    "Friend's code: ".into(),
                    Span::styled(padded, Style::new().yellow()),
                ]),
                Line::raw(""),
                error_line,
                Line::raw(""),
                Line::styled("Esc: back to lobby", Style::new().dim()),
            ]
        }
    };
    frame.render_widget(Paragraph::new(view_lines).centered(), chunks[5]);

    // Borderless inline warning - the parent overlay is already bordered,
    // nesting a second border would look noisy.
    if let Some(secs) = idle_warning {
        frame.render_widget(
            Line::styled(
                frittura_ssh_core::idle_warning_text(secs),
                Style::new().red().bold(),
            )
            .centered(),
            chunks[7],
        );
    }

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("Connected:     {}", stats.connected)),
            Line::from(format!("Ongoing games: {}", stats.ongoing_games)),
        ])
        .centered(),
        chunks[9],
    );
}
