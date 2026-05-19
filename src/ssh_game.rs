use crate::constants::UI_SCREEN_SIZE;
use crate::matchmaker;
use crate::tui::Tui;
use frittura_ssh_core::{spawn_event_converter, Credential, SshGame, SshSession};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

pub struct SshattrickGame {
    tui_sender: mpsc::Sender<Tui>,
}

impl SshattrickGame {
    pub fn new() -> Arc<Self> {
        let (tui_sender, tui_receiver) = mpsc::channel(64);
        matchmaker::spawn(tui_receiver, tui_sender.clone());
        Arc::new(Self { tui_sender })
    }
}

impl SshGame for SshattrickGame {
    type Auth = String;
    const SCREEN_SIZE: (u16, u16) = UI_SCREEN_SIZE;
    const TITLE: &'static str = "ssHattrick";
    const SERVER_INACTIVITY: Duration = Duration::from_secs(3600);

    async fn authenticate(
        &self,
        username: &str,
        _credential: Credential,
    ) -> anyhow::Result<String> {
        Ok(username.to_string())
    }

    async fn on_session(self: Arc<Self>, session: SshSession<String>) {
        let SshSession {
            auth: username,
            writer,
            data_rx,
            resize_rx,
            ..
        } = session;
        let events = spawn_event_converter(data_rx, resize_rx);
        let tui = match Tui::new(username, writer, events) {
            Ok(t) => t,
            Err(e) => {
                log::error!("Tui init failed: {e}");
                return;
            }
        };
        if self.tui_sender.send(tui).await.is_err() {
            log::warn!("Matchmaker gone; dropping session");
        }
    }
}
