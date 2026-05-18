use super::channel::AppChannel;
use crate::tui::Tui;
use crate::types::AppResult;
use anyhow::{anyhow, Context};
use russh::server::{self, Auth, Msg, Session};
use russh::{Channel, ChannelId, Pty};
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

pub struct AppClient {
    username: String,
    tui_sender: Sender<Tui>,
    channels: HashMap<ChannelId, AppChannel>,
}

impl AppClient {
    pub fn new(tui_sender: Sender<Tui>) -> Self {
        Self {
            username: String::new(),
            tui_sender,
            channels: HashMap::new(),
        }
    }

    fn channel_mut(&mut self, id: ChannelId) -> AppResult<&mut AppChannel> {
        self.channels
            .get_mut(&id)
            .with_context(|| format!("unknown channel: {id}"))
    }

    fn accept(&mut self, user: &str) -> AppResult<Auth> {
        self.username = user.to_string();
        Ok(Auth::Accept)
    }
}

impl server::Handler for AppClient {
    type Error = anyhow::Error;

    async fn auth_password(&mut self, user: &str, _password: &str) -> AppResult<Auth> {
        self.accept(user)
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        _public_key: &russh::keys::PublicKey,
    ) -> AppResult<Auth> {
        self.accept(user)
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> AppResult<bool> {
        let app_channel = AppChannel::new(self.username.clone());
        if self.channels.insert(channel.id(), app_channel).is_some() {
            return Err(anyhow!("channel `{}` has been already opened", channel.id()));
        }

        let ongoing = self.channels.len();
        let games = if ongoing == 1 {
            "is 1 ongoing game".to_string()
        } else {
            format!("are {ongoing} ongoing games")
        };
        let msg = format!("Connection opened, waiting for other player to join.\n\rThere {games}.\n\r");
        let _ = session.handle().data(channel.id(), msg).await;

        Ok(true)
    }

    async fn channel_close(&mut self, id: ChannelId, _: &mut Session) -> AppResult<()> {
        self.channels
            .remove(&id)
            .ok_or_else(|| anyhow!("channel `{id}` has been already closed"))?;
        Ok(())
    }

    async fn data(&mut self, id: ChannelId, data: &[u8], _: &mut Session) -> AppResult<()> {
        self.channel_mut(id)?.data(data).await
    }

    async fn pty_request(
        &mut self,
        id: ChannelId,
        _: &str,
        width: u32,
        height: u32,
        _: u32,
        _: u32,
        _: &[(Pty, u32)],
        session: &mut Session,
    ) -> AppResult<()> {
        let tui_sender = self.tui_sender.clone();
        self.channel_mut(id)?
            .pty_request(id, width, height, session.handle(), tui_sender)
            .await
    }

    async fn window_change_request(
        &mut self,
        id: ChannelId,
        width: u32,
        height: u32,
        _: u32,
        _: u32,
        _: &mut Session,
    ) -> AppResult<()> {
        self.channel_mut(id)?
            .window_change_request(width, height)
            .await
    }
}
