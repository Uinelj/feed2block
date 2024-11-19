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

mod feed_generator;
mod followers;
mod modlist;
mod state;

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
    let args = Args::parse();
    let agent = BskyAgent::builder()
        .config(Config::load(&FileStore::new(args.output)).await?)
        .build()
        .await?;

    let authors = from_feed(
        &agent,
        "at://did:plc:hhj2b7rqtaffsbd7a52dhf4j/app.bsky.feed.generator/aaad6k4xw5agi".into(),
    )
    .await?;

    let list = ModList::new(
        "at://did:plc:hhj2b7rqtaffsbd7a52dhf4j/app.bsky.graph.list/3lbckk67rxd2r".into(),
    );

    let debuglist = ModList::new(
        "at://did:plc:hhj2b7rqtaffsbd7a52dhf4j/app.bsky.graph.list/3lbafrng3b32u".into(),
    );

    for author in authors {
        println!("banning {:?}", &author.handle);
        list.add(&agent, author.did).await?;
    }

    let followers_stream = from_followers(
        &agent,
        AtIdentifier::Handle(Handle::new("cnews.bsky.social".into()).unwrap()),
    )
    .await;

    debuglist
        .add_stream(&agent, followers_stream.map(|o| o.did.clone()))
        .await?;
    Ok(())
}
