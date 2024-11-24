use std::path::PathBuf;

use atrium_api::types::string::{AtIdentifier, Handle};
use bsky_sdk::{
    agent::config::{Config, FileStore},
    BskyAgent,
};
use clap::Parser;
use feed_generator::from_feed;
use followers::from_followers;
use futures_util::stream::StreamExt;
use modlist::ModList;
use ratelimit::RateLimited;
use tracing::info;

mod feed_generator;
mod followers;
mod modlist;
mod ratelimit;
mod state;
pub mod subwatch;

/// Generate config from auth.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    // where the config is located
    #[arg(short, long, default_value = "config.json")]
    output: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    // client
    let client = RateLimited::default();

    let agent = BskyAgent::builder()
        .config(Config::load(&FileStore::new(args.output)).await?)
        .client(client)
        .build()
        .await?;

    let authors = from_feed(
        &agent,
        "at://did:plc:hhj2b7rqtaffsbd7a52dhf4j/app.bsky.feed.generator/aaad6k4xw5agi".into(),
    )
    .await?;

    let genders = ModList::new(
        "at://did:plc:hhj2b7rqtaffsbd7a52dhf4j/app.bsky.graph.list/3lbckk67rxd2r".into(),
    );

    info!(msg = "adding to gender blocklist");
    for author in authors {
        genders.add(&agent, author.did).await?;
    }

    let cnews = ModList::new(
        "at://did:plc:hhj2b7rqtaffsbd7a52dhf4j/app.bsky.graph.list/3lbd7snb23r2y".into(),
    );
    let followers_stream = from_followers(
        &agent,
        AtIdentifier::Handle(Handle::new("cnews.bsky.social".into()).unwrap()),
    )
    .await;

    info!(msg = "adding to cnews blocklist");
    cnews
        .add_stream(&agent, followers_stream.map(|o| o.did.clone()))
        .await?;
    Ok(())
}
