use std::path::PathBuf;

use bsky_sdk::agent::config::FileStore;
use bsky_sdk::BskyAgent;
use clap::Parser;
use tracing::info;

/// Generate config from auth.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// bsky handle (foo.bsky.social)
    #[arg(short, long)]
    identifier: String,

    /// app password (not your account password!)
    #[arg(short, long)]
    app_password: String,

    /// where to put the config
    #[arg(short, long, default_value = "config.json")]
    output: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let agent = BskyAgent::builder().build().await?;
    agent.login(args.identifier, args.app_password).await?;
    info!(msg = "saving config", location = ?args.output);
    agent
        .to_config()
        .await
        .save(&FileStore::new(args.output))
        .await?;
    Ok(())
}
