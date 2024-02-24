use ratatui::{
    backend::CrosstermBackend,
    style::{Style, Stylize},
    Terminal,
};
use russh::{server::Handle, ChannelId, CryptoVec};
use std::{
    fmt::{Debug, Formatter},
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

pub type AppResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub type Tick = u128;
pub type SshTerminal = Terminal<CrosstermBackend<TerminalHandle>>;

pub trait SystemTimeTick {
    fn now() -> Self;
    fn from_system_time(time: SystemTime) -> Self;
    fn as_system_time(&self) -> SystemTime;
}

impl SystemTimeTick for Tick {
    fn now() -> Self {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }

    fn from_system_time(time: SystemTime) -> Tick {
        time.duration_since(UNIX_EPOCH).unwrap().as_millis()
    }

    fn as_system_time(&self) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_millis(*self as u64)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum GameSide {
    Red,
    Blue,
}

impl GameSide {
    pub fn bar_style(&self) -> Style {
        match self {
            GameSide::Red => Style::new().red(),
            GameSide::Blue => Style::new().blue(),
        }
    }
}

#[derive(Clone)]
pub struct TerminalHandle {
    handle: Handle,
    // The sink collects the data which is finally flushed to the handle.
    sink: Vec<u8>,
    channel_id: ChannelId,
}

impl Debug for TerminalHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalHandle")
            .field("channel_id", &self.channel_id)
            .finish()
    }
}

impl TerminalHandle {
    pub fn new(handle: Handle, channel_id: ChannelId) -> Self {
        Self {
            handle,
            sink: Vec::new(),
            channel_id,
        }
    }

    pub fn message(&mut self, text: &str) -> std::io::Result<()> {
        let crypto_vec = CryptoVec::from_slice(text.as_bytes());
        self.write(crypto_vec.as_ref())?;
        self.flush()?;
        Ok(())
    }

    async fn _flush(&self) -> std::io::Result<usize> {
        let handle = self.handle.clone();
        let channel_id = self.channel_id.clone();
        let data: CryptoVec = self.sink.clone().into();
        let data_length = data.len();
        let result = handle.data(channel_id, data).await;
        if result.is_err() {
            log::error!("Failed to send data: {:?}", result);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to send data",
            ));
        }
        log::debug!(
            "Sent {} bytes of data to channel {}",
            data_length,
            channel_id
        );
        Ok(data_length)
    }
}

// The crossterm backend writes to the terminal handle.
impl std::io::Write for TerminalHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sink.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        futures::executor::block_on(self._flush())?;
        self.sink.clear();
        Ok(())
    }
}

pub trait Vector2D {
    fn normalize(&self) -> Self;
    fn dot(&self, other: &Self) -> f32;
    fn magnitude(&self) -> f32;
    fn mul(self, rhs: f32) -> Self;
}

impl Vector2D for (f32, f32) {
    fn normalize(&self) -> Self {
        let length = self.magnitude();
        if length == 0.0 {
            return (0.0, 0.0);
        }
        self.mul(1.0 / length)
    }

    fn dot(&self, other: &Self) -> f32 {
        let (x1, y1) = self;
        let (x2, y2) = other;
        x1 * x2 + y1 * y2
    }

    fn magnitude(&self) -> f32 {
        let (x, y) = self;
        (x.powi(2) + y.powi(2)).sqrt()
    }

    fn mul(self, rhs: f32) -> Self {
        let (x, y) = self;
        (x * rhs, y * rhs)
    }
}
