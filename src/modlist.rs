use async_stream::stream;
use futures_util::{pin_mut, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing::{info, warn};

use atrium_api::{
    app::bsky::graph::{get_list, listitem},
    types::string::{Datetime, Did},
    xrpc::XrpcClient,
};
use bsky_sdk::BskyAgent;
use futures_core::Stream;
use ipld_core::ipld::Ipld;

#[derive(Serialize, Deserialize)]
pub struct ModList(String);

/// TODO: batch add
impl ModList {
    pub fn new(list: String) -> Self {
        Self(list)
    }

    /// add did to modlist
    pub async fn add<T: XrpcClient + Send + Sync>(
        &self,
        agent: &BskyAgent<T>,
        did: Did,
    ) -> Result<(), Box<dyn Error>> {
        agent
            .create_record(listitem::Record {
                data: listitem::RecordData {
                    created_at: Datetime::now(),
                    list: self.0.clone(),
                    subject: did,
                },
                extra_data: Ipld::Null,
            })
            .await?;
        Ok(())
    }

    /// Consume a stream of dids, adding each of them into the modlist
    pub async fn add_stream<T: XrpcClient + Send + Sync>(
        &self,
        agent: &BskyAgent<T>,
        dids: impl Stream<Item = (Did, Option<String>)>,
    ) -> Result<Option<String>, Box<dyn Error>> {
        pin_mut!(dids);
        let mut last_cursor = None;
        while let Some((did, cursor)) = dids.next().await {
            if let Some(c) = cursor {
                last_cursor = Some(c);
            }
            // info!(msg = "adding to list", list = self.0, did = ?did);
            self.add(agent, did).await?;
        }

        if last_cursor.is_none() {
            warn!(msg = "no last cursor found!");
        }

        Ok(last_cursor)
    }

    /// gets members of provided list.
    /// set cursor to a cursor if you want to skip a part of the list.
    pub async fn get_members<T: XrpcClient + Send + Sync>(
        list: String,
        agent: &BskyAgent<T>,
        cursor: Option<String>,
    ) -> impl Stream<Item = Did> + '_ {
        let get_batch = |list: String, cursor: Option<String>| async {
            agent
                .api
                .app
                .bsky
                .graph
                .get_list(get_list::Parameters {
                    data: get_list::ParametersData {
                        cursor,
                        limit: None,
                        list,
                    },
                    extra_data: Ipld::Null,
                })
                .await
        };

        stream! {
            let mut cursor = cursor;
            for i in 0.. {

                // TODO fix
                let batch = get_batch(list.clone(), cursor).await.unwrap();
                info!(msg="getting batch", nb=i, cursor=?batch.cursor);
                cursor = batch.cursor.clone();
                info!(msg="got members", nb=&batch.data.items.len());
                for member in batch.data.items {
                    yield member.data.subject.data.did;
                }

                if cursor.is_none() {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bsky_sdk::agent::{
        config::{Config, FileStore},
        BskyAgentBuilder,
    };
    use futures_util::{pin_mut, StreamExt};

    use crate::{modlist::ModList, ratelimit::RateLimited};

    #[tokio::test]
    async fn test_get() {
        let client = RateLimited::default();
        let agent = BskyAgentBuilder::default()
            .config(Config::load(&FileStore::new("config.json")).await.unwrap())
            .client(client)
            .build()
            .await
            .unwrap();

        let modlist = ModList::new(
            "at://did:plc:hhj2b7rqtaffsbd7a52dhf4j/app.bsky.graph.list/3lbd7snb23r2y".into(),
        );

        let stream = ModList::get_members(modlist.0, &agent, None).await;
        pin_mut!(stream);
        while let Some(x) = stream.next().await {
            println!("{x:?}");
        }
    }
}
