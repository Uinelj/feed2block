use std::error::Error;

use atrium_api::types::string::{AtIdentifier, Did};
use futures_core::Stream;
use futures_util::{future, StreamExt, TryStreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::info;
use url::Url;

#[derive(Debug)]
pub enum Event {
    Follow,
    Unfollow,
}

#[derive(Debug)]
pub struct Follow {
    pub from: Did,
    to: Did,
    event: Event,
}

impl Follow {
    pub fn from(&self) -> &str {
        self.from.as_ref()
    }
}

impl TryFrom<serde_json::Value> for Follow {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let to = value
            .get("commit")
            .and_then(|v| v.get("record"))
            .and_then(|v| v.get("subject"))
            .and_then(|v| v.as_str())
            .and_then(|v| Some(Did::new(v.into())))
            .unwrap()
            .unwrap();

        let from = value
            .get("did")
            .and_then(|v| v.as_str())
            .and_then(|v| Some(Did::new(v.into())))
            .unwrap()
            .unwrap();

        let event = value
            .get("commit")
            .and_then(|v| v.get("operation"))
            .and_then(|v| v.as_str())
            .expect("missing event");

        let event = match event {
            "create" => Event::Follow,
            "delete" => Event::Unfollow,
            _ => panic!("unsupported event"),
        };

        Ok(Self { from, to, event })
    }
}

type FollowStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct SubWatcher {
    watch_identifier: Did,
    stream: FollowStream,
}

impl SubWatcher {
    pub async fn new(jetstream: Url, watch_identifier: Did) -> Self {
        let mut jetstream = jetstream.join("subscribe").unwrap();
        jetstream
            .query_pairs_mut()
            .append_pair("wantedCollections", "app.bsky.graph.follow")
            .append_pair("wanteddids", watch_identifier.as_str())
            .finish();
        // let jetstream2 = "wss://jetstream2.us-east.bsky.network/subscribe?wantedCollections=app.bsky.graph.follow";

        info!(msg = "is eq", eq = jetstream.as_str());
        info!(msg="opening stream", url=?jetstream.as_str());

        let (stream, _) = connect_async(jetstream.as_str()).await.unwrap();
        Self {
            watch_identifier,
            stream,
        }
    }

    pub async fn stream(self) -> impl Stream<Item = Follow> {
        self.stream.filter_map(move |item| {
            let watch_identifier = self.watch_identifier.clone();
            async move {
                let item = item.unwrap();
                let item: serde_json::Value = serde_json::from_slice(&item.into_data()).unwrap();
                // info!(item=?item);
                let subject = &item["commit"]["record"]["subject"]
                    .as_str()
                    .unwrap_or("none");

                if watch_identifier.as_str() == *subject {
                    Some(item.try_into().unwrap())
                } else {
                    None
                }
            }
        })
    }
}
