//! Gets a user list from a generator
//!
//!

use std::error::Error;

use atrium_api::{
    app::bsky::{actor::defs::ProfileViewBasicData, feed::get_feed},
    xrpc::XrpcClient,
};
use bsky_sdk::BskyAgent;
use ipld_core::ipld::Ipld;
use tracing::info;

pub async fn from_feed<T: XrpcClient + Send + Sync>(
    agent: &BskyAgent<T>,
    feed: String,
) -> Result<Vec<ProfileViewBasicData>, Box<dyn Error>> {
    let gf = agent.api.app.bsky.feed.get_feed(get_feed::Parameters {
        data: get_feed::ParametersData {
            cursor: None,
            feed,
            limit: None,
        },
        extra_data: Ipld::Null,
    });

    let posts = gf.await?;
    let authors: Vec<_> = posts
        .feed
        .iter()
        .map(|p| p.post.author.data.clone())
        .collect();

    info!(msg = "got authors", len = authors.len());
    Ok(authors)
}
