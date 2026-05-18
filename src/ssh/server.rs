use super::client::AppClient;
use crate::game::{Game, GameState};
use crate::tui::Tui;
use crate::types::{AppResult, GameSide, TerminalEvent};
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
        let (tui_sender, tui_receiver) = mpsc::channel(8);
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
        task::spawn(Self::matchmaker(tui_receiver));

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

    async fn matchmaker(mut tui_receiver: mpsc::Receiver<Tui>) {
        let mut pending_tui: Option<Tui> = None;
        while let Some(tui) = tui_receiver.recv().await {
            if let Some(red_tui) = pending_tui.take() {
                println!("Got second TUI for {}", tui.username());
                Self::spawn_game(red_tui, tui);
            } else {
                println!("Got first TUI for {}", tui.username());
                pending_tui = Some(tui);
            }
        }
    }

    fn spawn_game(mut red_tui: Tui, mut blue_tui: Tui) {
        task::spawn(async move {
            let mut game = Game::new();
            println!("Game {} spawned", game.id);

            let mut update_ticker = time::interval(UPDATE_TIME_STEP);
            let mut draw_ticker = time::interval(DRAW_TIME_STEP);
            draw_ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                if let GameState::Ending { time } = game.state {
                    if Instant::now() - time > AFTER_GAME_DELAY {
                        break;
                    }
                }

                select! {
                    _ = update_ticker.tick() => {
                        if let Err(e) = game.update() {
                            println!("Error updating game: {e}");
                            break;
                        }
                    }

                    _ = draw_ticker.tick() => {
                        if let Err(e) = Self::draw_and_push(&game, &mut red_tui, &mut blue_tui).await {
                            println!("Error rendering game: {e}");
                            break;
                        }
                    }

                    event = red_tui.next() => {
                        if Self::handle_event(&mut game, GameSide::Red, event) {
                            break;
                        }
                    }

                    event = blue_tui.next() => {
                        if Self::handle_event(&mut game, GameSide::Blue, event) {
                            break;
                        }
                    }
                }
            }

            let _ = red_tui.exit().await;
            let _ = blue_tui.exit().await;
        });
    }

    async fn draw_and_push(game: &Game, red_tui: &mut Tui, blue_tui: &mut Tui) -> AppResult<()> {
        red_tui.draw(game)?;
        blue_tui.draw(game)?;
        let (red, blue) = tokio::join!(red_tui.push_data(), blue_tui.push_data());
        red?;
        blue?;
        Ok(())
    }

    fn handle_event(game: &mut Game, side: GameSide, event: TerminalEvent) -> bool {
        match event {
            TerminalEvent::Key(key) => {
                game.handle_key_events(side, key.code);
                false
            }
            TerminalEvent::Quit => {
                game.state = GameState::Ending {
                    time: Instant::now(),
                };
                true
            }
            _ => false,
        }
    }
}

impl Server for AppServer {
    type Handler = AppClient;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> AppClient {
        AppClient::new(self.tui_sender.clone())
    }
}
