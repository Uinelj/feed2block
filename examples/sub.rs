use std::error::Error;

use atrium_api::types::string::{AtIdentifier, Did};
use futures_util::StreamExt;
use tokio_tungstenite::connect_async;

#[derive(Debug)]
enum Event {
    Follow,
    Unfollow,
}

#[derive(Debug)]
struct Follow {
    from: AtIdentifier,
    to: AtIdentifier,
    event: Event,
}

impl TryFrom<serde_json::Value> for Follow {
    type Error = serde_json::Error;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let to = value
            .get("commit")
            .and_then(|v| v.get("record"))
            .and_then(|v| v.get("subject"))
            .and_then(|v| v.as_str()).map(|v| Did::new(v.into()))
            .unwrap()
            .unwrap();

        let from = value
            .get("did")
            .and_then(|v| v.as_str()).map(|v| Did::new(v.into()))
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

        Ok(Self {
            from: AtIdentifier::Did(from),
            to: AtIdentifier::Did(to),
            event,
        })
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bgs = "bsky.network";
    // let (mut stream, _) = connect_async(format!("wss://{bgs}/xrpc/{NSID}")).await?;
    // louis' did:
    let did = "did:plc:p7gxyfr5vii5ntpwo7f6dhe2";

    let (mut stream, _) = connect_async(r#"wss://jetstream2.us-east.bsky.network/subscribe?wantedCollections=app.bsky.graph.follow&wantedDids=did:plc:hhj2b7rqtaffsbd7a52dhf4j"#.to_string())
    .await?;
    // ""
    while let Some(item) = stream.next().await {
        let item: serde_json::Value = serde_json::from_slice(&item.unwrap().into_data()).unwrap();
        let subject = &item["commit"]["record"]["subject"];
        let origin = &item["did"];
        dbg!(&item["commit"]["collection"]);
        if &serde_json::Value::String(did.into()) == subject {
            let f: Follow = item.try_into().unwrap();
            // println!("{:#?}", f);
        }
        // let subject = item["commit"]["record"]["subject"];
    }
    Ok(())
}
