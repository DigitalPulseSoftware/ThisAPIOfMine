use taom_database::{ConstQueryMap, Query};
use tokio_postgres::types::Type;

pub const QUERIES: ConstQueryMap<&str, 9> = ConstQueryMap::new([
    // CONNECTION
    ("find-player-info", Query::params(
        "SELECT uuid, nickname FROM players WHERE id = $1",
        &[Type::INT4]
    )),
    ("get-player-permissions", Query::params(
        "SELECT permission FROM player_permissions WHERE player_id = $1",
        &[Type::INT4]
    )),

    // SHIPS
    ("get-player-ship", Query::params(
        "SELECT data FROM player_ships WHERE player_id = $1 AND slot = $2",
        &[Type::INT4, Type::INT4]
    )),
    ("insert-player-ship", Query::params(
        "INSERT INTO player_ships(player_id, slot, last_update, data) VALUES($1, $2, NOW(), $3) ON CONFLICT(player_id, slot) DO UPDATE SET last_update = NOW(), data = EXCLUDED.data",
        &[Type::INT4, Type::INT4, Type::JSONB]
    )),

    // PLAYERS
    ("create-player", Query::params(
        "INSERT INTO players(uuid, creation_time, nickname) VALUES($1, NOW(), $2) RETURNING id",
        &[Type::UUID, Type::VARCHAR]
    )),
    ("create-token", Query::params(
        "INSERT INTO player_tokens(token, player_id) VALUES($1, $2)",
        &[Type::VARCHAR, Type::INT4]
    )),
    ("find-player-info", Query::params(
        "SELECT uuid, nickname FROM players WHERE id = $1",
        &[Type::INT4]
    )),
    ("find-token", Query::params(
        "SELECT player_id FROM player_tokens WHERE token = $1",
        &[Type::VARCHAR]
    )),
    ("update-player-connection", Query::params(
        "UPDATE players SET last_connection_time = NOW() WHERE id = $1",
        &[Type::INT4]
    )),
]);
