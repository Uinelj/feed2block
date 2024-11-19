use atrium_api::types::string::Cid;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct State {
    pub last_post_seen: Cid,
}
