use crate::game::Game;
use crate::tui::Tui;

pub const FRIEND_CODE_LEN: usize = 6;

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
}

impl PendingPlayer {
    pub fn new(tui: Tui) -> Self {
        Self {
            tui,
            mode: PlayerMode::Idle,
            dirty: true,
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
