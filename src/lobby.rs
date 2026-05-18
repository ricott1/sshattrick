use std::time::{Duration, Instant};

use crate::game::Game;
use crate::tui::Tui;

pub const FRIEND_CODE_LEN: usize = 6;
pub const LOBBY_IDLE_KICK: Duration = Duration::from_secs(60);
pub const LOBBY_IDLE_WARNING_REMAINING: Duration = Duration::from_secs(10);

#[derive(Debug, Default, Clone, Copy)]
pub struct LobbyStats {
    pub connected: usize,
    pub ongoing_games: usize,
}

/// Public view of the lobby state, used by `ui::render_lobby` to pick the
/// right hints/header. `PlayerMode` keeps the live game state too, which
/// the rendering layer doesn't need.
pub enum LobbyView<'a> {
    Idle,
    AutoQueue,
    ShowingCode {
        code: &'a str,
        typed: &'a str,
        last_attempt_failed: bool,
    },
}

pub enum PlayerMode {
    Idle,
    AutoQueue,
    Practicing(Box<Game>),
    ShowingCode(Box<FriendCodeState>),
}

#[derive(Debug)]
pub struct FriendCodeState {
    pub code: String,
    pub typed: String,
    pub last_attempt_failed: bool,
}

impl FriendCodeState {
    pub fn new(code: String) -> Self {
        Self {
            code,
            typed: String::new(),
            last_attempt_failed: false,
        }
    }
}

pub struct PendingPlayer {
    pub tui: Tui,
    pub mode: PlayerMode,
    /// True when the lobby view needs to be re-rendered (mode change, resize,
    /// new connection, or lobby stats changed). Practising players ignore
    /// this and always redraw at the game tick rate.
    pub dirty: bool,
    /// Last keystroke from this player. Drives the lobby inactivity kick
    /// (LOBBY_IDLE_KICK) and the warning countdown. Resize events are
    /// excluded - some terminals fire spurious WINCH events from tile-manager
    /// animations and we don't want those to count as "the user is here".
    pub last_input_at: Instant,
}

impl PendingPlayer {
    pub fn new(tui: Tui) -> Self {
        Self {
            tui,
            mode: PlayerMode::Idle,
            dirty: true,
            last_input_at: Instant::now(),
        }
    }
}

pub fn generate_invite_code() -> String {
    use rand::distr::{Distribution, Uniform};
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let dist = Uniform::new(0, ALPHABET.len()).expect("non-empty alphabet");
    let mut rng = rand::rng();
    (0..FRIEND_CODE_LEN)
        .map(|_| ALPHABET[dist.sample(&mut rng)] as char)
        .collect()
}
