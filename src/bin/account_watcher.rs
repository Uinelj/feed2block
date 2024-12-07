use atrium_api::app::bsky::actor::get_profile;
use atrium_api::types::string::{AtIdentifier, Did};
use bsky_sdk::{
    agent::config::{Config, FileStore},
    BskyAgent,
};
use clap::{command, Parser};
use feed2block::{
    followers::from_followers, modlist::ModList, ratelimit::RateLimited, subwatch::SubWatcher,
};
use futures_util::{pin_mut, StreamExt};
use std::fs;
use std::{error::Error, path::PathBuf};
use tracing::{info, warn};

const JETSTREAM_URL: &str = "wss://jetstream2.us-east.bsky.network/";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// bsky handle (foo.bsky.social)
    #[arg(short, long, env)]
    account: String,

    // modlist
    #[arg(short, long, env)]
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

    #[arg(long, default_value = "cursor.txt")]
    cursor: PathBuf,
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
        cursor,
    } = Args::parse();

    info!(acc = account, modlist = modlist);
    let client = RateLimited::default();
    let agent = BskyAgent::builder()
        .config(Config::load(&FileStore::new(config)).await.unwrap())
        .client(client)
        .build()
        .await
        .unwrap();

    let did = agent
        .api
        .app
        .bsky
        .actor
        .get_profile(get_profile::Parameters {
            data: get_profile::ParametersData {
                actor: AtIdentifier::Handle(account.parse()?),
            },
            extra_data: ipld_core::ipld::Ipld::Null,
        })
        .await?
        .did
        .clone();

    // let watch_identifier: Did = account.parse().expect("invalid did");
    let list = ModList::new(modlist);

    if backfill {
        let last_cursor = match fs::read_to_string(&cursor) {
            Ok(c) => {
                info!(msg = "read cursor from file", cursor = c);
                Some(c)
            }
            Err(e) => {
                warn!(msg = "no cursor found: starting from scratch", error = ?e);
                None
            }
        };
        let follower_stream = from_followers(&agent, AtIdentifier::Did(did.clone()), last_cursor)
            .await
            .map(|(f, cursor)| (f.did.clone(), cursor));

        pin_mut!(follower_stream);

        info!(msg = "backfilling from start");
        let last_cursor = list.add_stream(&agent, follower_stream).await?;
        if let Some(c) = last_cursor {
            info!(msg = "writing last cursor", cursor = c);
            fs::write(cursor, c)?;
        }
        info!(msg = "backfilling done");
    }

    let event_stream = SubWatcher::new(JETSTREAM_URL.parse().unwrap(), did).await;
    info!(msg = "connected to event_stream", url = JETSTREAM_URL,);

    let did_stream = event_stream.stream().await.map(|x| (x.from, None));

    list.add_stream(&agent, did_stream).await?;

    Ok(())
}
