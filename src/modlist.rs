use futures_util::{pin_mut, stream::StreamExt};
use std::error::Error;

use atrium_api::{
    app::bsky::graph::listitem,
    types::string::{Datetime, Did},
};
use bsky_sdk::BskyAgent;
use futures_core::Stream;
use ipld_core::ipld::Ipld;

pub struct ModList(String);

impl ModList {
    pub fn new(list: String) -> Self {
        Self(list)
    }

    pub async fn add(&self, agent: &BskyAgent, did: Did) -> Result<(), Box<dyn Error>> {
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

    pub async fn add_stream(
        &self,
        agent: &BskyAgent,
        dids: impl Stream<Item = Did>,
    ) -> Result<(), Box<dyn Error>> {
        pin_mut!(dids);
        while let Some(did) = dids.next().await {
            self.add(agent, did).await?;
        }

        Ok(())
    }
}
