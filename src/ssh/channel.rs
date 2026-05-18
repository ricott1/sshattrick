use crate::ssh::utils::{convert_data_to_terminal_event, CMD_RESIZE};
use crate::tui::Tui;
use crate::types::{AppResult, TerminalEvent};
use anyhow::anyhow;
use russh::server::Handle;
use russh::ChannelId;
use std::fmt::Debug;
use tokio::sync::mpsc::{self, Sender};

#[derive(Clone)]
pub struct SSHWriterProxy {
    flushing: bool,
    channel_id: ChannelId,
    handle: Handle,
    sink: Vec<u8>,
}

impl Debug for SSHWriterProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SSHWriterProxy")
            .field("flushing", &self.flushing)
            .field("channel_id", &self.channel_id)
            .field("sink", &self.sink)
            .finish()
    }
}

impl SSHWriterProxy {
    pub fn new(channel_id: ChannelId, handle: Handle) -> Self {
        Self {
            flushing: false,
            channel_id,
            handle,
            sink: vec![],
        }
    }

    pub async fn send(&mut self) -> std::io::Result<usize> {
        if !self.flushing {
            return Ok(0);
        }

        let data_length = self.sink.len();
        if let Err(e) = self
            .handle
            .data(self.channel_id, std::mem::take(&mut self.sink))
            .await
        {
            log::error!("Flushing error: {e:?}");
            let _ = self.handle.close(self.channel_id).await;
        }
        self.flushing = false;
        Ok(data_length)
    }
}

impl std::io::Write for SSHWriterProxy {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sink.extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.flushing = true;
        Ok(())
    }
}

#[derive(Debug)]
pub struct AppChannel {
    state: AppChannelState,
    username: String,
}

#[derive(Debug)]
enum AppChannelState {
    AwaitingPty,
    Ready { event_sender: Sender<TerminalEvent> },
}

impl AppChannel {
    pub fn new(username: String) -> Self {
        Self {
            state: AppChannelState::AwaitingPty,
            username,
        }
    }

    pub async fn data(&mut self, data: &[u8]) -> AppResult<()> {
        let AppChannelState::Ready { event_sender } = &mut self.state else {
            return Err(anyhow!("pty hasn't been allocated yet"));
        };

        if let Some(event) = convert_data_to_terminal_event(data) {
            event_sender
                .send(event)
                .await
                .map_err(|_| anyhow!("lost ssh connection"))?;
        }

        Ok(())
    }

    pub async fn pty_request(
        &mut self,
        id: ChannelId,
        width: u32,
        height: u32,
        handle: Handle,
        tui_sender: Sender<Tui>,
    ) -> AppResult<()> {
        let AppChannelState::AwaitingPty = &self.state else {
            return Err(anyhow!("pty has been already allocated"));
        };

        let writer = SSHWriterProxy::new(id, handle);
        let (event_sender, event_receiver) = mpsc::channel(16);
        let tui = Tui::new(self.username.clone(), writer, event_receiver)?;

        tui_sender
            .send(tui)
            .await
            .map_err(|_| anyhow!("game server is gone"))?;

        self.state = AppChannelState::Ready { event_sender };
        self.window_change_request(width, height).await?;

        Ok(())
    }

    pub async fn window_change_request(&mut self, width: u32, height: u32) -> AppResult<()> {
        let AppChannelState::Ready { event_sender } = &mut self.state else {
            return Err(anyhow!("pty hasn't been allocated yet"));
        };

        let width = width.min(255) as u8;
        let height = height.min(255) as u8;
        let data = [CMD_RESIZE, width, height];
        if let Some(event) = convert_data_to_terminal_event(&data) {
            event_sender
                .send(event)
                .await
                .map_err(|_| anyhow!("lost ssh connection"))?;
        }

        Ok(())
    }
}
