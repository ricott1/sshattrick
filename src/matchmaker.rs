use crate::game::{Game, GameState};
use crate::lobby::{
    generate_invite_code, FriendCodeState, LobbyStats, PendingPlayer, PlayerMode, FRIEND_CODE_LEN,
    LOBBY_IDLE_KICK, LOBBY_IDLE_WARNING_REMAINING,
};
use crate::tui::Tui;
use crate::types::{AppResult, GameSide};
use crossterm::event::KeyCode;
use frittura_ssh_core::TerminalEvent;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, Sender};
use tokio::task;
use tokio::time::MissedTickBehavior;
use tokio::{select, time};

const UPDATE_TIME_STEP: Duration = Duration::from_millis(1000 / 100);
const DRAW_TIME_STEP: Duration = Duration::from_millis(1000 / 30);
const AFTER_GAME_DELAY: Duration = Duration::from_millis(2000);

pub fn spawn(tui_receiver: mpsc::Receiver<Tui>, lobby_sender: Sender<Tui>) {
    let ongoing_games = Arc::new(AtomicUsize::new(0));
    task::spawn(matchmaker(tui_receiver, lobby_sender, ongoing_games));
}

async fn matchmaker(
    mut tui_receiver: mpsc::Receiver<Tui>,
    lobby_sender: Sender<Tui>,
    ongoing_games: Arc<AtomicUsize>,
) {
    let mut players: Vec<PendingPlayer> = vec![];
    let mut update_ticker = time::interval(UPDATE_TIME_STEP);
    let mut draw_ticker = time::interval(DRAW_TIME_STEP);
    draw_ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut last_stats = LobbyStats::default();

    loop {
        select! {
            new_tui = tui_receiver.recv() => match new_tui {
                Some(tui) => {
                    log::info!("Player joined: {}", tui.username());
                    players.push(PendingPlayer::new(tui));
                }
                None => return,
            },
            _ = update_ticker.tick() => {
                if !players.is_empty() {
                    process_lobby_events(&mut players).await;
                    for player in players.iter_mut() {
                        if let PlayerMode::Practicing(g) = &mut player.mode {
                            if let Err(e) = g.update() {
                                log::error!("Practice update error: {e}");
                                player.mode = PlayerMode::Idle;
                                player.dirty = true;
                            }
                        }
                    }
                    pair_friend_codes(&mut players, &lobby_sender, &ongoing_games);
                    pair_auto_queue(&mut players, &lobby_sender, &ongoing_games);
                    kick_idle_players(&mut players).await;
                }
            }
            _ = draw_ticker.tick() => {
                let stats = LobbyStats {
                    connected: players.len(),
                    ongoing_games: ongoing_games.load(Ordering::Relaxed),
                };
                if stats.connected != last_stats.connected
                    || stats.ongoing_games != last_stats.ongoing_games
                {
                    for player in players.iter_mut() {
                        player.dirty = true;
                    }
                    last_stats = stats;
                }
                redraw_players(&mut players, &stats).await;
            }
        }
    }
}

async fn process_lobby_events(players: &mut Vec<PendingPlayer>) {
    let mut to_remove = vec![];
    for (i, player) in players.iter_mut().enumerate() {
        while let Some(event) = player.tui.try_next() {
            match event {
                TerminalEvent::Quit => {
                    log::info!("Player {} disconnected", player.tui.username());
                    to_remove.push(i);
                    break;
                }
                TerminalEvent::Resize(_, _) => {
                    player.dirty = true;
                }
                TerminalEvent::Key(key) => {
                    player.last_input_at = Instant::now();
                    let prev_discriminant = std::mem::discriminant(&player.mode);

                    if matches!(player.mode, PlayerMode::Idle) {
                        match key.code {
                            KeyCode::Esc | KeyCode::Backspace => {
                                log::info!("Player {} left lobby", player.tui.username());
                                to_remove.push(i);
                                break;
                            }
                            KeyCode::Char('a') => player.mode = PlayerMode::AutoQueue,
                            KeyCode::Char('p') => {
                                player.mode = PlayerMode::Practicing(Box::new(
                                    Game::new_practice(),
                                ));
                            }
                            KeyCode::Char('g') => {
                                player.mode = PlayerMode::ShowingCode(Box::new(
                                    FriendCodeState::new(generate_invite_code()),
                                ));
                            }
                            _ => {}
                        }
                    } else if let PlayerMode::ShowingCode(state) = &mut player.mode {
                        match key.code {
                            KeyCode::Esc => player.mode = PlayerMode::Idle,
                            KeyCode::Backspace => {
                                if state.typed.is_empty() {
                                    player.mode = PlayerMode::Idle;
                                } else {
                                    state.typed.pop();
                                    state.last_attempt_failed = false;
                                    player.dirty = true;
                                }
                            }
                            KeyCode::Char(c) if c.is_ascii_alphanumeric() => {
                                if state.last_attempt_failed {
                                    state.typed.clear();
                                    state.last_attempt_failed = false;
                                }
                                if state.typed.len() < FRIEND_CODE_LEN {
                                    state.typed.push(c.to_ascii_uppercase());
                                    player.dirty = true;
                                }
                            }
                            _ => {}
                        }
                    } else if matches!(key.code, KeyCode::Esc | KeyCode::Backspace) {
                        player.mode = PlayerMode::Idle;
                    } else if let PlayerMode::Practicing(g) = &mut player.mode {
                        g.handle_key_events(GameSide::Red, key.code);
                    }

                    if prev_discriminant != std::mem::discriminant(&player.mode) {
                        player.dirty = true;
                    }
                }
                _ => {}
            }
        }
    }
    for i in to_remove.into_iter().rev() {
        let player = players.remove(i);
        player.tui.close().await;
    }
}

async fn kick_idle_players(players: &mut Vec<PendingPlayer>) {
    let now = Instant::now();
    let mut to_remove = vec![];
    for (i, player) in players.iter_mut().enumerate() {
        if matches!(player.mode, PlayerMode::Practicing(_)) {
            continue;
        }
        let elapsed = now.saturating_duration_since(player.last_input_at);
        if elapsed >= LOBBY_IDLE_KICK {
            log::info!(
                "Kicking idle player {} after {}s",
                player.tui.username(),
                elapsed.as_secs()
            );
            to_remove.push(i);
            continue;
        }
        if kick_warning_secs(player.last_input_at, now).is_some() {
            player.dirty = true;
        }
    }
    for i in to_remove.into_iter().rev() {
        let player = players.remove(i);
        player.tui.close().await;
    }
}

fn pair_friend_codes(
    players: &mut Vec<PendingPlayer>,
    lobby_sender: &Sender<Tui>,
    ongoing_games: &Arc<AtomicUsize>,
) {
    if !players
        .iter()
        .any(|p| matches!(&p.mode, PlayerMode::ShowingCode(s) if s.typed.len() == FRIEND_CODE_LEN))
    {
        return;
    }

    loop {
        let host_map: HashMap<&str, usize> = players
            .iter()
            .enumerate()
            .filter_map(|(i, p)| match &p.mode {
                PlayerMode::ShowingCode(s) => Some((s.code.as_str(), i)),
                _ => None,
            })
            .collect();

        let pair = players.iter().enumerate().find_map(|(i, typer)| {
            let s = match &typer.mode {
                PlayerMode::ShowingCode(s) if s.typed.len() == FRIEND_CODE_LEN => s,
                _ => return None,
            };
            let &j = host_map.get(s.typed.as_str())?;
            if i == j {
                return None;
            }
            Some((i.min(j), i.max(j)))
        });

        let Some((lo, hi)) = pair else { break };
        let high_player = players.remove(hi);
        let low_player = players.remove(lo);
        log::info!(
            "Friend-code match: {} and {}",
            low_player.tui.username(),
            high_player.tui.username()
        );
        spawn_game(
            low_player.tui,
            high_player.tui,
            lobby_sender.clone(),
            ongoing_games.clone(),
        );
    }

    for player in players.iter_mut() {
        if let PlayerMode::ShowingCode(state) = &mut player.mode {
            if state.typed.len() == FRIEND_CODE_LEN && !state.last_attempt_failed {
                state.last_attempt_failed = true;
                player.dirty = true;
            }
        }
    }
}

fn pair_auto_queue(
    players: &mut Vec<PendingPlayer>,
    lobby_sender: &Sender<Tui>,
    ongoing_games: &Arc<AtomicUsize>,
) {
    loop {
        let auto_indices: Vec<usize> = players
            .iter()
            .enumerate()
            .filter(|(_, p)| matches!(p.mode, PlayerMode::AutoQueue))
            .map(|(i, _)| i)
            .take(2)
            .collect();
        let [i1, i2] = auto_indices.as_slice() else {
            return;
        };
        let p2 = players.remove(*i2);
        let p1 = players.remove(*i1);
        log::info!(
            "Auto-pairing {} and {}",
            p1.tui.username(),
            p2.tui.username()
        );
        spawn_game(p1.tui, p2.tui, lobby_sender.clone(), ongoing_games.clone());
    }
}

async fn redraw_players(players: &mut [PendingPlayer], stats: &LobbyStats) {
    let now = Instant::now();
    for player in players.iter_mut() {
        let PendingPlayer {
            tui,
            mode,
            dirty,
            last_input_at,
        } = player;
        let mut wrote = false;
        match mode {
            PlayerMode::Practicing(g) => {
                if let Ok(lines) = g.render_lines() {
                    let _ = tui.draw(g, &lines, GameSide::Red);
                    wrote = true;
                }
            }
            lobby_mode if *dirty => {
                let warning = kick_warning_secs(*last_input_at, now);
                let view = match lobby_mode {
                    PlayerMode::Idle => crate::lobby::LobbyView::Idle,
                    PlayerMode::AutoQueue => crate::lobby::LobbyView::AutoQueue,
                    PlayerMode::ShowingCode(state) => crate::lobby::LobbyView::ShowingCode {
                        code: state.code.as_str(),
                        typed: state.typed.as_str(),
                        last_attempt_failed: state.last_attempt_failed,
                    },
                    PlayerMode::Practicing(_) => unreachable!(),
                };
                let _ = tui.draw_lobby(stats, view, warning);
                *dirty = false;
                wrote = true;
            }
            _ => {}
        }
        if wrote {
            let _ = tui.push_data().await;
        }
    }
}

fn spawn_game(
    red_tui: Tui,
    blue_tui: Tui,
    lobby_sender: Sender<Tui>,
    ongoing_games: Arc<AtomicUsize>,
) {
    ongoing_games.fetch_add(1, Ordering::Relaxed);
    task::spawn(async move {
        let mut red = Some(red_tui);
        let mut blue = Some(blue_tui);
        let mut game = Game::new();
        log::info!("Game {} spawned", game.id);

        let mut update_ticker = time::interval(UPDATE_TIME_STEP);
        let mut draw_ticker = time::interval(DRAW_TIME_STEP);
        draw_ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            if let GameState::Ending { time, .. } = game.state {
                if Instant::now() - time > AFTER_GAME_DELAY {
                    break;
                }
            }
            if red.is_none() && blue.is_none() {
                break;
            }

            select! {
                _ = update_ticker.tick() => {
                    if let Err(e) = game.update() {
                        log::error!("Error updating game: {e}");
                        break;
                    }
                }
                _ = draw_ticker.tick() => {
                    if let Err(e) = draw_and_push(&game, &mut red, &mut blue).await {
                        log::error!("Error rendering game: {e}");
                        break;
                    }
                }
                event = next_or_pending(&mut red) => {
                    handle_event(&mut game, GameSide::Red, event, &mut red).await;
                }
                event = next_or_pending(&mut blue) => {
                    handle_event(&mut game, GameSide::Blue, event, &mut blue).await;
                }
            }
        }

        let winner = game.winner();

        for (side, slot) in [(GameSide::Red, red), (GameSide::Blue, blue)] {
            if let Some(mut tui) = slot {
                tui.record_game(winner == Some(side));
                if let Err(err) = lobby_sender.send(tui).await {
                    err.0.close().await;
                }
            }
        }
        ongoing_games.fetch_sub(1, Ordering::Relaxed);
    });
}

async fn draw_and_push(
    game: &Game,
    red: &mut Option<Tui>,
    blue: &mut Option<Tui>,
) -> AppResult<()> {
    if red.is_none() && blue.is_none() {
        return Ok(());
    }
    let image_lines = game.render_lines()?;
    for (slot, side) in [
        (red.as_mut(), GameSide::Red),
        (blue.as_mut(), GameSide::Blue),
    ] {
        if let Some(t) = slot {
            t.draw(game, &image_lines, side)?;
        }
    }
    match (red.as_mut(), blue.as_mut()) {
        (Some(r), Some(b)) => {
            let (a, c) = tokio::join!(r.push_data(), b.push_data());
            a?;
            c?;
        }
        (Some(t), None) | (None, Some(t)) => t.push_data().await?,
        (None, None) => {}
    }
    Ok(())
}

async fn handle_event(
    game: &mut Game,
    side: GameSide,
    event: TerminalEvent,
    own_tui: &mut Option<Tui>,
) {
    match event {
        TerminalEvent::Quit => {
            if let Some(tui) = own_tui.take() {
                tui.close().await;
            }
            if !matches!(game.state, GameState::Ending { .. }) {
                game.end_with_winner(Some(side.opposite()), true);
            }
        }
        TerminalEvent::Key(crossterm::event::KeyEvent {
            code: KeyCode::Esc,
            ..
        }) => {
            if !matches!(game.state, GameState::Ending { .. }) {
                game.end_with_winner(Some(side.opposite()), true);
            }
        }
        TerminalEvent::Key(key) => game.handle_key_events(side, key.code),
        _ => {}
    }
}

/// Seconds remaining until inactivity kick, rounded up. `None` when outside
/// the warning window or already past the kick deadline.
fn kick_warning_secs(last_input_at: Instant, now: Instant) -> Option<u32> {
    let elapsed = now.saturating_duration_since(last_input_at);
    if elapsed >= LOBBY_IDLE_KICK {
        return None;
    }
    let remaining = LOBBY_IDLE_KICK - elapsed;
    if remaining >= LOBBY_IDLE_WARNING_REMAINING {
        return None;
    }
    let secs = remaining.as_secs() as u32 + u32::from(remaining.subsec_nanos() > 0);
    Some(secs)
}

async fn next_or_pending(tui: &mut Option<Tui>) -> TerminalEvent {
    match tui {
        Some(t) => t.next().await,
        None => std::future::pending().await,
    }
}
