mod big_text;
pub mod img_lines;
mod lobby;
mod matchmaker;
mod ssh_game;
mod tui;
mod ui;

pub use ssh_game::SshattrickGame;
pub use sshattrick_core::types::AppResult;
pub use sshattrick_core::utils::store_path;
