use crossterm::event::{KeyEvent, MouseEvent};
use image::Rgba;
use ratatui::style::{Style, Stylize};
use std::fmt::Display;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub type AppResult<T> = Result<T, anyhow::Error>;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameSide {
    #[default]
    Red,
    Blue,
}

impl Display for GameSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Red => write!(f, "Red"),
            Self::Blue => write!(f, "Blue"),
        }
    }
}

impl GameSide {
    pub fn bar_style(&self) -> Style {
        match self {
            GameSide::Red => Style::new().red(),
            GameSide::Blue => Style::new().blue(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Palette {
    #[default]
    Dark,
    Light,
    Basket,
    Alt,
}

impl Palette {
    pub fn next(&self) -> Self {
        match self {
            Palette::Dark => Palette::Light,
            Palette::Light => Palette::Basket,
            Palette::Basket => Palette::Alt,
            Palette::Alt => Palette::Dark,
        }
    }

    pub fn skate_trace_color(&self) -> Rgba<u8> {
        match self {
            Palette::Dark => Rgba([55, 55, 85, 255]),
            Palette::Light => Rgba([145, 215, 255, 255]),
            Palette::Basket => Rgba([55, 55, 85, 255]),
            Palette::Alt => Rgba([105, 55, 55, 255]),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Orientation {
    #[default]
    Up,
    UpLeft,
    Left,
    DownLeft,
    Down,
    DownRight,
    Right,
    UpRight,
}

impl Orientation {
    pub const MAX: usize = 8;

    pub fn next(self) -> Self {
        ((self as usize + 1) % Self::MAX).into()
    }

    pub fn previous(self) -> Self {
        ((self as usize + Self::MAX - 1) % Self::MAX).into()
    }
}

impl From<usize> for Orientation {
    fn from(value: usize) -> Self {
        match value {
            0 => Orientation::Up,
            1 => Orientation::UpLeft,
            2 => Orientation::Left,
            3 => Orientation::DownLeft,
            4 => Orientation::Down,
            5 => Orientation::DownRight,
            6 => Orientation::Right,
            7 => Orientation::UpRight,
            _ => panic!("Invalid orientation"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TerminalEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Quit,
}

impl Future for TerminalEvent {
    type Output = Self;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(*self)
    }
}
