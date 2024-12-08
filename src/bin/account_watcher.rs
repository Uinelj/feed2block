use atrium_api::types::string::AtIdentifier;
use atrium_api::xrpc::XrpcClient;
use atrium_api::{app::bsky::actor::get_profile, types::string::Did};
use bsky_sdk::{
    agent::config::{Config, FileStore},
    BskyAgent,
};
use clap::{command, Parser};
use feed2block::state::State;
use feed2block::{
    followers::from_followers, modlist::ModList, ratelimit::RateLimited, state::States,
    subwatch::SubWatcher,
};
use futures_util::{pin_mut, StreamExt};
use std::fs::{File, OpenOptions};
use std::{error::Error, path::PathBuf};
use tokio::sync::oneshot;
use tokio::{select, signal, task};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

const JETSTREAM_URL: &str = "wss://jetstream2.us-east.bsky.network/";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// bsky handle (foo.bsky.social)
    #[arg(short, long, env)]
    account: String,

    // modlist
    #[arg(short, long, env)]
    modlist: String,

    /// backfill
    #[arg(
        short,
        long,
        default_value = "false",
        help = "backfills the entier followers"
    )]
    backfill: bool,

    #[arg(short, long, default_value = "config.json")]
    config: PathBuf,

    #[arg(long, default_value = "cursor.json")]
    cursor: PathBuf,
}

async fn run_backfill<T: Send + Sync + XrpcClient>(
    agent: &BskyAgent<T>,
    did: &Did,
    did_state: &mut State,
    last_added: Option<Did>,
) -> Result<(), Box<dyn Error>> {
    let last_cursor = match did_state.cursor() {
        Some(c) => {
            info!(msg = "read cursor from file", cursor = c);
            Some(c)
        }
        None => {
            warn!(msg = "no cursor found: starting from scratch");
            None
        }
    };

    let follower_stream = from_followers(
        &agent,
        AtIdentifier::Did(did.clone()),
        last_cursor.map(String::from), // could we accept Option<&str>?
    )
    .await
    .map(|(f, cursor)| (f.did.clone(), cursor));

    pin_mut!(follower_stream);

    info!(
        msg = "backfilling",
        start_cursor = last_cursor,
        last_added = ?last_added
    );
    let last_cursor = if let Some(last_added) = last_added {
        did_state
            .modlist
            .add_stream_shortcircuit(&agent, follower_stream, last_added)
            .await?
    } else {
        did_state
            .modlist
            .add_stream(&agent, follower_stream)
            .await?
    };

    if let Some(c) = last_cursor {
        info!(msg = "writing last cursor", cursor = c);
        did_state.set_cursor(c);
    };

    Ok(())
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // install global collector configured based on RUST_LOG env var.
    tracing_subscriber::fmt::init();
    let Args {
        account,
        modlist,
        backfill,
        config,
        cursor,
    } = Args::parse();

    let token = CancellationToken::new();

    info!(acc = account, modlist = modlist);
    let client = RateLimited::default();
    let agent = BskyAgent::builder()
        .config(Config::load(&FileStore::new(config)).await.unwrap())
        .client(client)
        .build()
        .await
        .unwrap();

    // load states
    let mut states = match File::open(&cursor) {
        Ok(r) => serde_json::from_reader(r)?,
        Err(_) => States::new(),
    };

    // get did of handle
    let did = agent
        .api
        .app
        .bsky
        .actor
        .get_profile(get_profile::Parameters {
            data: get_profile::ParametersData {
                actor: AtIdentifier::Handle(account.parse()?),
            },
            extra_data: ipld_core::ipld::Ipld::Null,
        })
        .await?
        .did
        .clone();

    // load state for did
    let did_state =
        states
            .entry(did.clone())
            .or_insert(State::new(ModList::new(modlist.clone()), None, None));

    if backfill {
        // get last added to modlist
        let last_added = ModList::get_last_member(modlist.clone(), &agent)
            .await
            .map(|f| f.did);
        info!(msg = "got last added", did = ?last_added);
        run_backfill(&agent, &did, did_state, last_added).await?;
        info!(msg = "backfilling done, writing state", state_path = ?cursor);

        let w = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&cursor)?;

        // drop mutable ref to be able to write the state
        let _ = did_state;
        serde_json::to_writer(w, &states)?;
    }

    let cloned_token = token.clone();
    let (tx, rx) = oneshot::channel();

    let jetstream = task::spawn(async move {
        // load state for did
        let did_state =
            states
                .entry(did.clone())
                .or_insert(State::new(ModList::new(modlist), None, None));
        let event_stream = SubWatcher::new(JETSTREAM_URL.parse().unwrap(), did).await;
        info!(msg = "connected to event_stream", url = JETSTREAM_URL,);

        let did_stream = event_stream.stream().await.map(|x| (x.from, None));

        select! {
            _ =
                did_state
                    .modlist
                    .add_stream(&agent, did_stream)
                     => {}
            _ = cloned_token.cancelled() => {
                info!(msg="got cancellation");
                tx.send(states).unwrap()
            }
        }
    });

    match signal::ctrl_c().await {
        Ok(()) => {
            token.cancel();
            let w = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(cursor)?;

            let states = rx.await?;
            info!(msg = "got states back", "states"=?&states);
            serde_json::to_writer(w, &states)?;
            info!(msg = "shutting down!")
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
            // we also shut down in case of error
        }
    }
    Ok(())
}
