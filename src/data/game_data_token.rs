use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct GameDataToken {
    pub player_uuid: Uuid,
    // JWT fields
    pub exp: u64,
    pub iat: u64,
    pub sub: String, //< "access" or "refresh"
}

impl GameDataToken {
    pub fn new_access(player_uuid: Uuid, duration: Duration) -> Self {
        Self::new("access", player_uuid, duration)
    }

    pub fn new_refresh(player_uuid: Uuid, duration: Duration) -> Self {
        Self::new("refresh", player_uuid, duration)
    }

    fn new(token_type: &str, player_uuid: Uuid, duration: Duration) -> Self {
        let now = jsonwebtoken::get_current_timestamp();
        Self {
            player_uuid,
            exp: now + duration.as_secs(),
            iat: now,
            sub: token_type.to_owned(),
        }
    }
}
