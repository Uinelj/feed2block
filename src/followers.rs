//! from account

use async_stream::stream;
use atrium_api::{
    app::bsky::{actor::defs::ProfileViewData, graph::get_followers},
    types::{string::AtIdentifier, LimitedNonZeroU8, Object},
    xrpc::XrpcClient,
};
use bsky_sdk::BskyAgent;
use futures_core::Stream;
use ipld_core::ipld::Ipld;
use tracing::info;

pub async fn from_followers<T: XrpcClient + Send + Sync>(
    agent: &BskyAgent<T>,
    actor: AtIdentifier,
) -> impl Stream<Item = Object<ProfileViewData>> + '_ {
    let get_batch = |actor: AtIdentifier, cursor: Option<_>| async {
        agent
            .api
            .app
            .bsky
            .graph
            .get_followers(get_followers::Parameters {
                data: get_followers::ParametersData {
                    actor,
                    cursor,
                    limit: Some(LimitedNonZeroU8::MAX),
                },
                extra_data: Ipld::Null,
            })
            .await
    };

    stream! {
        let mut cursor = None;
        for i in 0.. {
            info!(msg="getting batch", nb=i);
            let batch = get_batch(actor.clone(), cursor).await.unwrap();
            cursor = batch.cursor.clone();
            for follower in batch.data.followers {
                yield follower;
            }
            if cursor.is_none() {
                break;
            }
        }
    }
}
