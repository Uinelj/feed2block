use std::error::Error;

use atrium_api::types::string::Did;
use futures_util::{pin_mut, StreamExt};
use subwatch::SubWatcher;
use tracing::info;
use url::Url;
mod subwatch;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    let jetstream: Url = r#"wss://jetstream2.us-east.bsky.network/"#.parse()?;
    let wi: Did = "did:plc:klqgiogdcdyurckdikyxq76r".parse()?; // akkes
    let wi: Did = "did:plc:p7gxyfr5vii5ntpwo7f6dhe2".parse()?; // AOC
                                                               // let wi: Did = "did:plc:p7gxyfr5vii5ntpwo7f6dhe2".parse()?;
    let sw = SubWatcher::new(jetstream, wi).await;

    let s = sw.stream().await;
    pin_mut!(s);
    while let Some(x) = s.next().await {
        info!(msg="got event", event=?x);
    }
    Ok(())
}
