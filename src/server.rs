use crate::{
    game::Game,
    types::{AppResult, SshTerminal, TerminalHandle},
};
use async_trait::async_trait;
use crossterm::event::KeyCode;
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use russh::{server::*, Channel, ChannelId};
use russh_keys::key::{KeyPair, PublicKey};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    sync::Arc,
};
use tokio::sync::Mutex;

const GAME_NAME: &str = "Rustrick";
const TERMINAL_WIDTH: u16 = 160;
const TERMINAL_HEIGHT: u16 = 50;

pub fn save_keys(signing_key: &ed25519_dalek::SigningKey) -> AppResult<()> {
    let file = File::create::<&str>("./keys".into())?;
    assert!(file.metadata()?.is_file());
    let mut buffer = std::io::BufWriter::new(file);
    buffer.write(&signing_key.to_bytes())?;
    Ok(())
}

pub fn load_keys() -> AppResult<ed25519_dalek::SigningKey> {
    let file = File::open::<&str>("./keys".into())?;
    let mut buffer = std::io::BufReader::new(file);
    let mut buf: [u8; 32] = [0; 32];
    buffer.read(&mut buf)?;
    Ok(ed25519_dalek::SigningKey::from_bytes(&buf))
}

fn convert_data_to_key_code(data: &[u8]) -> crossterm::event::KeyCode {
    match data {
        b"\x1b[A" => crossterm::event::KeyCode::Up,
        b"\x1b[B" => crossterm::event::KeyCode::Down,
        b"\x1b[C" => crossterm::event::KeyCode::Right,
        b"\x1b[D" => crossterm::event::KeyCode::Left,
        // ctrl+c is also converted to esc
        b"\x03" => crossterm::event::KeyCode::Esc,
        b"\x1b" => crossterm::event::KeyCode::Esc,
        b"\x0d" => crossterm::event::KeyCode::Enter,
        b"\x7f" => crossterm::event::KeyCode::Backspace,
        b"\x1b[3~" => crossterm::event::KeyCode::Delete,
        b"\x09" => crossterm::event::KeyCode::Tab,
        _ => crossterm::event::KeyCode::Char(data[0] as char),
    }
}

#[derive(Clone)]
pub struct GameServer {
    clients: Arc<Mutex<HashMap<usize, SshTerminal>>>,
    clients_to_game: Arc<Mutex<HashMap<usize, uuid::Uuid>>>,
    client_id: usize,
    games: Arc<Mutex<HashMap<uuid::Uuid, Game>>>,
    pending_client: Arc<Mutex<Option<usize>>>,
}

impl GameServer {
    pub fn new() -> Self {
        log::info!("Creating new server");
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            clients_to_game: Arc::new(Mutex::new(HashMap::new())),
            client_id: 0,
            games: Arc::new(Mutex::new(HashMap::new())),
            pending_client: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn run(&mut self, port: u16) -> Result<(), anyhow::Error> {
        let games = self.games.clone();
        let clients = self.clients.clone();
        let clients_to_game = self.clients_to_game.clone();
        log::info!("Starting game loop");
        // TODO (maybe): spawn a new loop for each game. Not sure it's a good idea actually
        // To close the loop, check if both are disconnected or the game is over.
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(2)).await;
                let mut to_remove = vec![];
                for (_, game) in games.lock().await.iter_mut() {
                    if game.is_over() {
                        to_remove.push((game.id, game.client_ids()));
                    }
                    game.update().expect(
                        "Failed to update game. This is a bug, please report it to the developers",
                    );

                    game.draw().expect(
                        "Failed to draw game. This is a bug, please report it to the developers",
                    );
                }
                for ids in to_remove {
                    let (game_id, (red_client_id, blue_client_id)) = ids;
                    log::info!("Removing game {game_id}");
                    games.lock().await.remove(&game_id);
                    clients.lock().await.remove(&red_client_id);
                    clients.lock().await.remove(&blue_client_id);
                    clients_to_game.lock().await.remove(&red_client_id);
                    clients_to_game.lock().await.remove(&blue_client_id);
                }
            }
        });

        let signing_key = load_keys().unwrap_or_else(|_| {
            let key_pair = russh_keys::key::KeyPair::generate_ed25519().unwrap();
            let signing_key = match key_pair {
                KeyPair::Ed25519(key) => key,
            };
            let _ = save_keys(&signing_key);
            signing_key
        });

        let key_pair = KeyPair::Ed25519(signing_key);

        let config = Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![key_pair],
            ..Default::default()
        };

        log::info!("Starting server on port {}", port);

        self.run_on_address(Arc::new(config), ("0.0.0.0", port))
            .await?;
        Ok(())
    }
}

impl Server for GameServer {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.client_id += 1;
        s
    }
}

#[async_trait]
impl Handler for GameServer {
    type Error = anyhow::Error;

    async fn channel_open_forwarded_tcpip(
        &mut self,
        channel: Channel<Msg>,
        _: &str,
        _: u32,
        _: &str,
        _: u32,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        self.channel_open_session(channel, session).await
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        {
            let mut terminal_handle = TerminalHandle::new(session.handle(), channel.id());
            let backend = CrosstermBackend::new(terminal_handle.clone());
            let terminal = Terminal::with_options(
                backend,
                ratatui::TerminalOptions {
                    viewport: ratatui::Viewport::Fixed(Rect {
                        x: 0,
                        y: 0,
                        width: TERMINAL_WIDTH,
                        height: TERMINAL_HEIGHT,
                    }),
                },
            )?;

            let mut clients = self.clients.lock().await;
            let mut pending_client_id = self.pending_client.lock().await;

            if pending_client_id.is_some() {
                let client_id = pending_client_id.as_ref().unwrap().clone();
                let pending_terminal = clients
                    .get(&client_id)
                    .ok_or(anyhow::anyhow!("Client not found in clients list"))?;
                let game = Game::new(
                    (client_id.clone(), pending_terminal.clone()),
                    (self.client_id, terminal),
                );

                self.games.lock().await.insert(game.id, game.clone());
                self.clients_to_game.lock().await.insert(client_id, game.id);
                self.clients_to_game
                    .lock()
                    .await
                    .insert(self.client_id, game.id);
                log::info!("Added player to new game");
                *pending_client_id = None;
            } else {
                clients.insert(self.client_id, terminal);
                *pending_client_id = Some(self.client_id);
                log::info!("Added player to pending list");
                terminal_handle.message(
                    format!(
                        "Welcome to the {GAME_NAME}! There are {} games running.\r\nWaiting for another player to join...\r\nIn the meanwhile, remember to set your terminal to a minimum of {TERMINAL_WIDTH}x{TERMINAL_HEIGHT} characters",
                        self.clients_to_game.lock().await.len()
                    )
                    .as_str(),
                )?;
            }
        }

        Ok(true)
    }

    async fn auth_none(&mut self, _: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_password(&mut self, _: &str, _: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_publickey(&mut self, _: &str, _: &PublicKey) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_keyboard_interactive(
        &mut self,
        _: &str,
        _: &str,
        _: Option<Response<'async_trait>>,
    ) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn window_change_request(
        &mut self,
        _: ChannelId,
        _: u32,
        _: u32,
        _: u32,
        _: u32,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        if let Some(game_id) = &mut self.clients_to_game.lock().await.get_mut(&self.client_id) {
            if let Some(game) = self.games.lock().await.get_mut(game_id) {
                game.clear_client(self.client_id);
            }
        }
        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let key_code = convert_data_to_key_code(data);
        if let Some(game_id) = &mut self.clients_to_game.lock().await.get_mut(&self.client_id) {
            if let Some(game) = self.games.lock().await.get_mut(game_id) {
                game.handle_input(self.client_id, key_code);
            } else {
                let pending_client = self.pending_client.lock().await;
                // In case no game is assigned but we are not waiting for a game, disconnect
                if pending_client.is_none()
                    || (pending_client.is_some() && pending_client.unwrap() != self.client_id)
                {
                    session.close(channel);
                    session.disconnect(russh::Disconnect::ByApplication, "Quit", "");
                }
            }
        }

        if key_code == KeyCode::Esc {
            self.clients.lock().await.remove(&self.client_id);
            self.clients_to_game.lock().await.remove(&self.client_id);
            session.close(channel);
            session.disconnect(russh::Disconnect::ByApplication, "Quit", "");
            let mut pending_client = self.pending_client.lock().await;
            if pending_client.is_some() && pending_client.unwrap() == self.client_id {
                *pending_client = None;
            }
        }

        Ok(())
    }

    async fn tcpip_forward(
        &mut self,
        _: &str,
        _: &mut u32,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let handle = session.handle();
        tokio::spawn(async move {
            handle
                .channel_open_forwarded_tcpip("", 0, "0.0.0.0", 0)
                .await
                .unwrap();
        });
        Ok(true)
    }
}