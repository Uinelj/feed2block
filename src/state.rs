use std::collections::HashMap;

use atrium_api::types::string::{AtIdentifier, Cid, Did};
use serde::{Deserialize, Serialize};

use crate::modlist::ModList;

pub type States = HashMap<Did, State>;

/// Did->modlist state:
/// last cursor returned when backfilling
/// last timestamp delivered by the jetstream
///
/// Those can be approximate since we'll likely won't be writing ts+cursor at each update.
#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub modlist: ModList,
    cursor: Option<String>,
    jetstream_ts: Option<i64>,
}

impl State {
    pub fn new(modlist: ModList, cursor: Option<String>, jetstream_ts: Option<i64>) -> Self {
        Self {
            modlist,
            cursor,
            jetstream_ts,
        }
    }

    pub fn cursor(&self) -> Option<&str> {
        self.cursor.as_deref()
    }

    /// Updates cursor.
    ///
    /// I might change the signature to accept Option<String>: I don't to avoid overwriting a commit accidentally
    pub fn set_cursor(&mut self, cursor: String) {
        self.cursor = Some(cursor)
    }
}
