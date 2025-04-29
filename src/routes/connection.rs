use actix_web::{HttpResponse, Responder, post, web};
use deadpool_postgres::tokio_postgres::types::Type;
use futures::{StreamExt, TryStreamExt};
use jsonwebtoken::{EncodingKey, Header};
use serde::Deserialize;
use tokio_postgres::Row;
use uuid::Uuid;

use crate::config::ApiConfig;
use crate::data::connection_token::{ConnectionToken, PrivateConnectionToken, ServerAddress};
use crate::data::game_data_token::GameDataToken;
use crate::data::player_data::PlayerData;
use crate::errors::api::{ErrorCause, RouteError};
use crate::errors::codes::ServerErrorCode;
use crate::routes::players::validate_player_token;

const DEV_TOKEN: &[u8] = const_base::decode!(
    "Unsecure+Developer+Token+Giving+Admin+Perms=",
    const_base::Config::B64
);

#[derive(Deserialize)]
struct GameConnectionParams {
    token: String,
    dev: Option<bool>,
}

#[post("/v1/game/connect")]
async fn game_connect(
    config: web::Data<ApiConfig>,
    pg_pool: web::Data<deadpool_postgres::Pool>,
    params: web::Json<GameConnectionParams>,
) -> Result<impl Responder, RouteError> {
    let pg_client = pg_pool.get().await?;
    let player_id = validate_player_token(&pg_client, &params.token).await?;

    let is_dev = params.dev.unwrap_or(false);

    // TODO(SirLynix): to do this with only one query
    let find_player_info = pg_client
        .prepare_typed_cached(
            "SELECT uuid, nickname FROM players WHERE id = $1",
            &[Type::INT4],
        )
        .await?;

    let player_result = pg_client
        .query_opt(&find_player_info, &[&player_id])
        .await?
        .ok_or(RouteError::InvalidRequest(
            ServerErrorCode::InvalidId,
            format!("No player has the id '{player_id}'"),
        ))?;

    let uuid: Uuid = player_result.try_get(0)?;
    let nickname: String = player_result.try_get(1)?;

    let permissions: Vec<String>;
    if !is_dev {
        let get_player_permissions = pg_client
            .prepare_typed_cached(
                "SELECT permission FROM player_permissions WHERE player_id = $1",
                &[Type::INT4],
            )
            .await?;

        permissions = pg_client
            .query_raw(&get_player_permissions, &[&player_id])
            .await?
            .map(|row: Result<Row, tokio_postgres::Error>| row.and_then(|row| row.try_get(0)))
            .try_collect()
            .await?;
    } else {
        permissions = vec!["admin".into(), "dev".into()];
    }

    let player_data = PlayerData::new(uuid, nickname, permissions);

    let refresh_token = GameDataToken::new_refresh(
        player_id,
        uuid,
        config.game_api_refresh_token_duration,
        is_dev,
    );
    let refresh_token_jwt = jsonwebtoken::encode(
        &Header::default(),
        &refresh_token,
        &EncodingKey::from_secret(config.game_api_secret.unsecure().as_bytes()),
    )?;

    let private_token = PrivateConnectionToken::new(
        config.game_api_url.as_str(),
        refresh_token_jwt.as_str(),
        player_data,
    );

    let server_address = if !is_dev {
        ServerAddress::new(config.game_server_address.as_str(), config.game_server_port)
    } else {
        ServerAddress::new("localhost", config.game_server_port)
    };

    // force connection token key to be zero in dev mode to ensure it can't be used to connect to a regular server
    let connection_token_key = chacha20poly1305::Key::from_slice(if !is_dev {
        &config.connection_token_key
    } else {
        DEV_TOKEN
    });

    let token = ConnectionToken::generate(
        connection_token_key,
        config.connection_token_duration,
        server_address,
        private_token,
    )
    .map_err(|_| {
        RouteError::ServerError(ErrorCause::Internal, ServerErrorCode::TokenGenerationFailed)
    })?;

    Ok(HttpResponse::Ok().json(token))
}
