use clap::{ArgAction, Parser};
use log::LevelFilter;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};
use sshattrick::{store_path, AppResult, SshattrickGame};

const DEFAULT_PORT: u16 = 3020;

#[derive(Parser, Debug)]
#[clap(name="ssHattrick", about = "Hockey in the terminal via ssh", author, version, long_about = None)]
struct Args {
    #[clap(long, short = 'p', action=ArgAction::Set, help = "Set port to listen on")]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let logfile_path = store_path("sshattrick.log")?;
    let logfile = FileAppender::builder()
        .append(false)
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build(logfile_path)?;

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;

    log4rs::init_config(config)?;

    let port = Args::parse().port.unwrap_or(DEFAULT_PORT);
    let game = SshattrickGame::new();
    frittura_ssh_core::run_server(game, port).await?;

    Ok(())
}
