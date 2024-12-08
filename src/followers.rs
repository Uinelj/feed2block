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

#[cfg(test)]
mod tests {
    use atrium_api::types::string::{AtIdentifier, Handle};
    use bsky_sdk::agent::{
        config::{Config, FileStore},
        BskyAgentBuilder,
    };
    use futures_util::{pin_mut, StreamExt};

    use crate::{followers::from_followers, ratelimit::RateLimited};

    #[tokio::test]
    async fn test_last_follow() {
        let client = RateLimited::default();
        let agent = BskyAgentBuilder::default()
            .config(Config::load(&FileStore::new("config.json")).await.unwrap())
            .client(client)
            .build()
            .await
            .unwrap();

        let actor = AtIdentifier::Handle(Handle::new("cnews.bsky.social".into()).unwrap());
        let mut followers = from_followers(&agent, actor, None).await.take(5);
        pin_mut!(followers);

        while let Some(x) = followers.next().await {
            dbg!(&x.0.handle);
        }
    }
}
