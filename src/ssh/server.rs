use super::client::AppClient;
use crate::game::{Game, GameState};
use crate::tui::Tui;
use crate::types::{AppResult, GameSide, TerminalEvent};
use crate::utils::img_to_lines;
use itertools::Either;
use rand::RngExt;
use russh::keys::ssh_key::private::{Ed25519Keypair, Ed25519PrivateKey, KeypairData};
use russh::server::{Config, Server};
use std::fs::File;
use std::io::Write;
use std::pin::pin;
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
    println!("Created new keypair for SSH server.");
    Ok(())
}

fn load_keys() -> AppResult<russh::keys::PrivateKey> {
    let bytes = std::fs::read(KEY_PATH)?;
    let key = russh::keys::PrivateKey::from_bytes(&bytes)?;
    println!("Loaded keypair for SSH server.");
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
        println!(
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
        task::spawn(Self::matchmaker(tui_receiver, lobby_sender));

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
                println!("Shutting down");
                time::sleep(Duration::from_secs(1)).await;
                Ok(())
            }
        }
    }

    async fn matchmaker(mut tui_receiver: mpsc::Receiver<Tui>, lobby_sender: Sender<Tui>) {
        while let Some(mut pending) = tui_receiver.recv().await {
            println!("Pending player: {}", pending.username());
            refresh_lobby(&mut pending).await;

            let mate = loop {
                select! {
                    next = tui_receiver.recv() => match next {
                        Some(tui) => break Some(tui),
                        None => return,
                    },
                    event = pending.next() => match event {
                        TerminalEvent::Quit => {
                            println!("Pending player {} disconnected", pending.username());
                            break None;
                        }
                        TerminalEvent::Resize(_, _) => refresh_lobby(&mut pending).await,
                        _ => {}
                    }
                }
            };

            if let Some(other) = mate {
                Self::spawn_game(pending, other, lobby_sender.clone());
            }
        }
    }

    fn spawn_game(red_tui: Tui, blue_tui: Tui, lobby_sender: Sender<Tui>) {
        task::spawn(async move {
            let mut red = Some(red_tui);
            let mut blue = Some(blue_tui);
            let mut game = Game::new();
            println!("Game {} spawned", game.id);

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
        let image_lines = img_to_lines(&game.image()?);
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
            TerminalEvent::Key(key) => game.handle_key_events(side, key.code),
            TerminalEvent::Quit => {
                // Drop the quitter's Tui (Drop closes their channel); the survivor wins.
                own_tui.take();
                if !matches!(game.state, GameState::Ending { .. }) {
                    game.end_with_winner(Some(side.opposite()), true);
                }
            }
            _ => {}
        }
    }
}

async fn refresh_lobby(tui: &mut Tui) {
    let _ = tui.draw_lobby();
    let _ = tui.push_data().await;
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
