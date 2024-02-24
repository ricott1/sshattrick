use clap::{ArgAction, Parser};
use sshattrick::server::GameServer;

#[derive(Parser, Debug)]
#[clap(name="ssHattrick", about = "Hockey in the terminal via ssh", author, version, long_about = None)]
struct Args {
    #[clap(long, short = 'p', action=ArgAction::Set, help = "Set port to listen on")]
    port: Option<u16>,
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut game_server = GameServer::new();

    let port = Args::parse().port.unwrap_or(2020);

    game_server.run(port).await.expect("Failed running server");
}
