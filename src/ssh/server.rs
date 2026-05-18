use super::client::AppClient;
use crate::game::{Game, GameState};
use crate::lobby::{
    generate_invite_code, FriendCodeState, LobbyStats, PendingPlayer, PlayerMode, FRIEND_CODE_LEN,
};
use crate::tui::Tui;
use crate::types::{AppResult, GameSide, TerminalEvent};
use crossterm::event::KeyCode;
use itertools::Either;
use rand::RngExt;
use russh::keys::ssh_key::private::{Ed25519Keypair, Ed25519PrivateKey, KeypairData};
use russh::server::{Config, Server};
use std::fs::File;
use std::io::Write;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, Sender};
use tokio::task;
use tokio::time::MissedTickBehavior;
use tokio::{select, time};
use tokio_util::sync::CancellationToken;

const UPDATE_TIME_STEP: Duration = Duration::from_millis(1000 / 100);
const DRAW_TIME_STEP: Duration = Duration::from_millis(1000 / 30);
const AFTER_GAME_DELAY: Duration = Duration::from_millis(2000);
const KEY_PATH: &str = "./keys";

fn save_keys(signing_key: &russh::keys::PrivateKey) -> AppResult<()> {
    let mut buffer = std::io::BufWriter::new(File::create(KEY_PATH)?);
    buffer.write_all(&signing_key.to_bytes()?)?;
    log::info!("Created new keypair for SSH server.");
    Ok(())
}

fn load_keys() -> AppResult<russh::keys::PrivateKey> {
    let bytes = std::fs::read(KEY_PATH)?;
    let key = russh::keys::PrivateKey::from_bytes(&bytes)?;
    log::info!("Loaded keypair for SSH server.");
    Ok(key)
}

pub struct AppServer {
    port: u16,
    shutdown: CancellationToken,
    tui_sender: Sender<Tui>,
    tui_receiver: Option<mpsc::Receiver<Tui>>,
}

impl AppServer {
    pub fn new(port: u16) -> Self {
        let (tui_sender, tui_receiver) = mpsc::channel(64);
        Self {
            port,
            shutdown: CancellationToken::new(),
            tui_sender,
            tui_receiver: Some(tui_receiver),
        }
    }

    pub async fn run(&mut self) -> AppResult<()> {
        log::info!(
            "Starting SSH server on port {}. Press Ctrl-C to exit.",
            self.port
        );

        let private_key = load_keys().unwrap_or_else(|_| {
            let seed: [u8; Ed25519PrivateKey::BYTE_SIZE] = rand::rng().random();
            let key_data = KeypairData::from(Ed25519Keypair::from_seed(&seed));
            let key = russh::keys::PrivateKey::new(key_data, "sshattrick ssh server key")
                .expect("Failed to generate SSH keys");
            save_keys(&key).expect("Failed to save SSH keys");
            key
        });

        let config = Config {
            inactivity_timeout: Some(Duration::from_secs(3600)),
            auth_rejection_time: Duration::from_secs(3),
            auth_rejection_time_initial: Some(Duration::from_secs(0)),
            keys: vec![private_key],
            ..Default::default()
        };

        let tui_receiver = self
            .tui_receiver
            .take()
            .expect("AppServer::run called twice");
        let lobby_sender = self.tui_sender.clone();
        let ongoing_games = Arc::new(AtomicUsize::new(0));
        task::spawn(Self::matchmaker(
            tui_receiver,
            lobby_sender,
            ongoing_games,
        ));

        let shutdown = self.shutdown.clone();
        let server = self.run_on_address(Arc::new(config), ("0.0.0.0", self.port));
        let shutdown_cancelled = shutdown.cancelled();

        let result = {
            let mut server = pin!(server);
            let mut shutdown_cancelled = pin!(shutdown_cancelled);
            select! {
                result = &mut server => Either::Left(result),
                _ = &mut shutdown_cancelled => Either::Right(()),
            }
        };

        match result {
            Either::Left(result) => Ok(result?),
            Either::Right(()) => {
                log::info!("Shutting down");
                time::sleep(Duration::from_secs(1)).await;
                Ok(())
            }
        }
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
                        // A new connection changes `connected`; the next stats
                        // check at draw time flags everyone dirty.
                    }
                    None => return,
                },
                _ = update_ticker.tick() => {
                    if !players.is_empty() {
                        Self::process_lobby_events(&mut players);
                        for player in players.iter_mut() {
                            if let PlayerMode::Practicing(g) = &mut player.mode {
                                if let Err(e) = g.update() {
                                    log::error!("Practice update error: {e}");
                                    player.mode = PlayerMode::Idle;
                                    player.dirty = true;
                                }
                            }
                        }
                        Self::pair_friend_codes(&mut players, &lobby_sender, &ongoing_games);
                        Self::pair_auto_queue(&mut players, &lobby_sender, &ongoing_games);
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
                    Self::redraw_players(&mut players, &stats).await;
                }
            }
        }
    }

    fn process_lobby_events(players: &mut Vec<PendingPlayer>) {
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
                        // Other keys in non-Idle modes are intentionally ignored.

                        if prev_discriminant != std::mem::discriminant(&player.mode) {
                            player.dirty = true;
                        }
                    }
                    _ => {}
                }
            }
        }
        for i in to_remove.into_iter().rev() {
            players.remove(i);
        }
    }

    fn pair_friend_codes(
        players: &mut Vec<PendingPlayer>,
        lobby_sender: &Sender<Tui>,
        ongoing_games: &Arc<AtomicUsize>,
    ) {
        // Fast-path: nobody's typed buffer is full, nothing to do.
        if !players
            .iter()
            .any(|p| matches!(&p.mode, PlayerMode::ShowingCode(s) if s.typed.len() == FRIEND_CODE_LEN))
        {
            return;
        }

        loop {
            // host code -> player index. Rebuilt each loop iteration since
            // a successful pair removes two players.
            let host_map: std::collections::HashMap<&str, usize> = players
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
            Self::spawn_game(
                low_player.tui,
                high_player.tui,
                lobby_sender.clone(),
                ongoing_games.clone(),
            );
        }

        // Any remaining full buffer is one that found no host this tick. Flag
        // it so the user sees the error; the next keystroke clears and retries.
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
            Self::spawn_game(p1.tui, p2.tui, lobby_sender.clone(), ongoing_games.clone());
        }
    }

    async fn redraw_players(players: &mut [PendingPlayer], stats: &LobbyStats) {
        for player in players.iter_mut() {
            let PendingPlayer { tui, mode, dirty } = player;
            let mut wrote = false;
            match mode {
                PlayerMode::Practicing(g) => {
                    if let Ok(lines) = g.render_lines() {
                        let _ = tui.draw(g, &lines, GameSide::Red);
                        wrote = true;
                    }
                }
                PlayerMode::Idle if *dirty => {
                    let _ = tui.draw_lobby(stats, crate::lobby::LobbyView::Idle);
                    *dirty = false;
                    wrote = true;
                }
                PlayerMode::AutoQueue if *dirty => {
                    let _ = tui.draw_lobby(stats, crate::lobby::LobbyView::AutoQueue);
                    *dirty = false;
                    wrote = true;
                }
                PlayerMode::ShowingCode(state) if *dirty => {
                    let _ = tui.draw_lobby(
                        stats,
                        crate::lobby::LobbyView::ShowingCode {
                            code: state.code.as_str(),
                            typed: state.typed.as_str(),
                            last_attempt_failed: state.last_attempt_failed,
                        },
                    );
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
                        if let Err(e) = Self::draw_and_push(&game, &mut red, &mut blue).await {
                            log::error!("Error rendering game: {e}");
                            break;
                        }
                    }
                    event = next_or_pending(&mut red) => {
                        Self::handle_event(&mut game, GameSide::Red, event, &mut red);
                    }
                    event = next_or_pending(&mut blue) => {
                        Self::handle_event(&mut game, GameSide::Blue, event, &mut blue);
                    }
                }
            }

            let winner = game.winner();

            for (side, slot) in [(GameSide::Red, red), (GameSide::Blue, blue)] {
                if let Some(mut tui) = slot {
                    tui.record_game(winner == Some(side));
                    let _ = lobby_sender.send(tui).await;
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

    fn handle_event(
        game: &mut Game,
        side: GameSide,
        event: TerminalEvent,
        own_tui: &mut Option<Tui>,
    ) {
        match event {
            TerminalEvent::Quit => {
                // Connection dropped: player is gone, opponent wins.
                own_tui.take();
                if !matches!(game.state, GameState::Ending { .. }) {
                    game.end_with_winner(Some(side.opposite()), true);
                }
            }
            TerminalEvent::Key(crossterm::event::KeyEvent {
                code: KeyCode::Esc,
                ..
            }) => {
                // Esc: opponent wins as if disconnected, but the quitter's Tui
                // stays alive so they return to the lobby with stats updated.
                if !matches!(game.state, GameState::Ending { .. }) {
                    game.end_with_winner(Some(side.opposite()), true);
                }
            }
            TerminalEvent::Key(key) => game.handle_key_events(side, key.code),
            _ => {}
        }
    }
}

/// Yield the next event from an `Option<Tui>`, or park the branch forever when
/// the slot is empty. Used as a `select!` arm so a disconnected side simply
/// stops firing without needing extra control flow.
async fn next_or_pending(tui: &mut Option<Tui>) -> TerminalEvent {
    match tui {
        Some(t) => t.next().await,
        None => std::future::pending().await,
    }
}

impl Server for AppServer {
    type Handler = AppClient;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> AppClient {
        AppClient::new(self.tui_sender.clone())
    }
}
