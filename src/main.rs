use std::{future::Future, path::PathBuf, time::Duration};

use atrium_api::{
    types::string::{AtIdentifier, Handle},
    xrpc::{
        http::{Request, Response},
        HttpClient, XrpcClient,
    },
};
use atrium_xrpc_client::reqwest::{ReqwestClient, ReqwestClientBuilder};
use bsky_sdk::{
    agent::config::{Config, FileStore},
    BskyAgent,
};
use clap::Parser;
use feed_generator::from_feed;
use followers::from_followers;
use futures_util::stream::StreamExt;
use modlist::ModList;
use tower::limit::{rate::Rate, RateLimit};
use tracing::{info, warn};

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

struct RateLimitBskyClient(RateLimit<ReqwestClient>);

impl HttpClient for RateLimitBskyClient {
    #[doc = " Send an HTTP request and return the response."]
    fn send_http(
        &self,
        request: Request<Vec<u8>>,
    ) -> impl Future<
        Output = core::result::Result<
            Response<Vec<u8>>,
            Box<dyn std::error::Error + Send + Sync + 'static>,
        >,
    > + Send {
        self.0.get_ref().send_http(request)
    }
}

impl XrpcClient for RateLimitBskyClient {
    #[doc = " The base URI of the XRPC server."]
    fn base_uri(&self) -> String {
        self.0.get_ref().base_uri()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    // client
    let client = RateLimitBskyClient(RateLimit::new(
        ReqwestClient::new("https://bsky.social"),
        Rate::new(1, Duration::from_secs(1)),
    ));

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
