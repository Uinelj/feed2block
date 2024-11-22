use std::error::Error;

use futures_util::StreamExt;
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let bgs = "bsky.network";
    // let (mut stream, _) = connect_async(format!("wss://{bgs}/xrpc/{NSID}")).await?;
    // louis' did:
    let did = "did:plc:p7gxyfr5vii5ntpwo7f6dhe2";

    let (mut stream, _) = connect_async(format!(
        // r#"wss://jetstream2.us-east.bsky.network/subscribe?wantedCollections=app.bsky.graph.follow&wantedDids=did:plc:hhj2b7rqtaffsbd7a52dhf4j"#
        r#"wss://jetstream2.us-east.bsky.network/subscribe?wantedCollections=app.bsky.graph.follow"#,
    ))
    .await?;
    // ""
    while let Some(item) = stream.next().await {
        let item: serde_json::Value = serde_json::from_slice(&item.unwrap().into_data()).unwrap();
        let subject = &item["commit"]["record"]["subject"];
        let origin = &item["did"];

        if &serde_json::Value::String(did.into()) == subject {
            println!("{:#?}", item);
        }
        // let subject = item["commit"]["record"]["subject"];
    }
    Ok(())
}
