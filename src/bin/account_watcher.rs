use std::{error::Error, path::PathBuf};

use atrium_api::types::string::{AtIdentifier, Did};
use bsky_sdk::{
    agent::config::{Config, FileStore},
    BskyAgent,
};
use clap::{command, Parser};
use feed2block::{
    followers::from_followers, modlist::ModList, ratelimit::RateLimited, subwatch::SubWatcher,
};
use futures_util::{pin_mut, FutureExt, StreamExt};
use tracing::info;

const JETSTREAM_URL: &str = "wss://jetstream2.us-east.bsky.network/";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// bsky handle (foo.bsky.social)
    #[arg(short, long)]
    account: String,

    // modlist
    #[arg(short, long)]
    modlist: String,

    /// backfill
    #[arg(
        short,
        long,
        default_value = "false",
        help = "backfills the entier followers"
    )]
    backfill: bool,

    #[arg(short, long, default_value = "config.json")]
    config: PathBuf,
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();
    let Args {
        account,
        modlist,
        backfill,
        config,
    } = Args::parse();

    let client = RateLimited::default();
    let agent = BskyAgent::builder()
        .config(Config::load(&FileStore::new(config)).await.unwrap())
        .client(client)
        .build()
        .await
        .unwrap();

    let watch_identifier: Did = account.parse().expect("invalid did");
    let list = ModList::new(modlist);

    if backfill {
        let follower_stream = from_followers(&agent, AtIdentifier::Did(watch_identifier.clone()))
            .await
            .map(|f| f.did.clone());

        pin_mut!(follower_stream);

        info!(msg = "backfilling from start");
        list.add_stream(&agent, follower_stream).await?;
        info!(msg = "backfilling done");
    }

    let event_stream = SubWatcher::new(JETSTREAM_URL.parse().unwrap(), watch_identifier).await;
    info!(msg = "connected to event_stream", url = JETSTREAM_URL,);

    let did_stream = event_stream.stream().await.map(|x| x.from);

    list.add_stream(&agent, did_stream).await?;

    Ok(())
}
