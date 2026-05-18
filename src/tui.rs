use crate::constants::UI_SCREEN_SIZE;
use crate::game::Game;
use crate::ssh::SSHWriterProxy;
use crate::types::{AppResult, TerminalEvent};
use crate::ui;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle};
use ratatui::layout::Rect;
use ratatui::prelude::CrosstermBackend;
use ratatui::{Terminal, TerminalOptions, Viewport};
use tokio::sync::mpsc::Receiver;

#[derive(Debug)]
pub struct Tui {
    username: String,
    terminal: Terminal<CrosstermBackend<SSHWriterProxy>>,
    events: Receiver<TerminalEvent>,
}

impl Tui {
    pub fn new(
        username: String,
        writer: SSHWriterProxy,
        events: Receiver<TerminalEvent>,
    ) -> AppResult<Self> {
        let backend = CrosstermBackend::new(writer);
        let opts = TerminalOptions {
            viewport: Viewport::Fixed(Rect {
                x: 0,
                y: 0,
                width: UI_SCREEN_SIZE.0,
                height: UI_SCREEN_SIZE.1,
            }),
        };
        let terminal = Terminal::with_options(backend, opts)?;
        let mut tui = Self {
            username,
            terminal,
            events,
        };
        tui.init()?;
        Ok(tui)
    }

    fn init(&mut self) -> AppResult<()> {
        crossterm::execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture,
            SetTitle("ssHattrick"),
            Clear(ClearType::All),
            Hide
        )?;
        Ok(())
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    /// Returns the next terminal event. If the channel is closed (client
    /// disconnected) we surface `Quit` so the game loop can wind down.
    pub async fn next(&mut self) -> TerminalEvent {
        self.events.recv().await.unwrap_or(TerminalEvent::Quit)
    }

    pub fn draw(&mut self, game: &Game) -> AppResult<()> {
        self.terminal
            .draw(|frame| ui::render(frame, game).expect("Error while rendering game."))?;
        Ok(())
    }

    pub async fn push_data(&mut self) -> AppResult<()> {
        self.terminal.backend_mut().writer_mut().send().await?;
        Ok(())
    }

    pub async fn exit(&mut self) -> AppResult<()> {
        crossterm::execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            Clear(ClearType::All),
            Show
        )?;
        self.terminal.backend_mut().writer_mut().send().await?;
        Ok(())
    }
}
