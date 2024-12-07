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
    cursor: Option<String>,
) -> impl Stream<Item = (Object<ProfileViewData>, Option<String>)> + '_ {
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
        let mut cursor = cursor;
        for i in 0.. {
            let batch = get_batch(actor.clone(), cursor).await.unwrap();
            info!(msg="getting batch", nb=i, cursor=?batch.cursor);
            cursor = batch.cursor.clone();
            info!(msg="got followers", nb=&batch.data.followers.len());
            for follower in batch.data.followers {
                yield (follower, cursor.clone());
            }
            if cursor.is_none() {
                break;
            }
        }
    }
}
