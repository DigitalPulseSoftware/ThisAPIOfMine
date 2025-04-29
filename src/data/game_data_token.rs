use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn default_as_false() -> bool {
    false
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameDataToken {
    pub player_db_id: i32,
    pub player_uuid: Uuid,
    #[serde(default = "default_as_false")]
    pub is_readonly: bool,
    // JWT fields
    pub exp: u64,
    pub iat: u64,
    pub sub: String, //< "access" or "refresh"
}

impl GameDataToken {
    #[inline]
    pub fn new_access(
        player_db_id: i32,
        player_uuid: Uuid,
        duration: Duration,
        is_readonly: bool,
    ) -> Self {
        Self::new("access", player_db_id, player_uuid, duration, is_readonly)
    }

    #[inline]
    pub fn new_refresh(
        player_db_id: i32,
        player_uuid: Uuid,
        duration: Duration,
        is_readonly: bool,
    ) -> Self {
        Self::new("refresh", player_db_id, player_uuid, duration, is_readonly)
    }

    fn new(
        token_type: &str,
        player_db_id: i32,
        player_uuid: Uuid,
        duration: Duration,
        is_readonly: bool,
    ) -> Self {
        debug_assert!(valid_token_type(token_type));

        let now = jsonwebtoken::get_current_timestamp();
        Self {
            player_db_id,
            player_uuid,
            is_readonly,
            exp: now + duration.as_secs(),
            iat: now,
            sub: token_type.to_string(),
        }
    }
}

fn valid_token_type(tt: &str) -> bool {
    tt == "access" || tt == "refresh"
}
