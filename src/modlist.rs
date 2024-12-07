use futures_util::{pin_mut, stream::StreamExt};
use std::error::Error;
use tracing::warn;

use atrium_api::{
    app::bsky::graph::listitem,
    types::string::{Datetime, Did},
    xrpc::XrpcClient,
};
use bsky_sdk::BskyAgent;
use futures_core::Stream;
use ipld_core::ipld::Ipld;

pub struct ModList(String);

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
}
