use actix_web::{post, web, HttpResponse, Responder};
use futures::future::join;
use futures::TryStreamExt;
use jsonwebtoken::{EncodingKey, Header};
use serde::Deserialize;
use taom_database::dynamic;
use uuid::Uuid;

use crate::config::ApiConfig;
use crate::data::connection_token::{ConnectionToken, PrivateConnectionToken, ServerAddress};
use crate::data::game_data_token::GameDataToken;
use crate::data::player_data::PlayerData;
use crate::database::QUERIES;
use crate::errors::api::{ErrorCause, ErrorCode, RequestError, RouteError};
use crate::routes::players::validate_player_token;

#[derive(Deserialize)]
struct GameConnectionParams {
    token: String,
}

#[post("/v1/game/connect")]
async fn game_connect(
    config: web::Data<ApiConfig>,
    pg_pool: web::Data<deadpool_postgres::Pool>,
    params: web::Json<GameConnectionParams>,
) -> Result<impl Responder, RouteError> {
    let pg_client = pg_pool.get().await?;
    let player_id = validate_player_token(&pg_client, &params.token).await?;

    let queries = join(
        async {
            Ok(QUERIES
                .prepare::<(Uuid, String)>("find-player-info", &pg_client)
                .query_one([dynamic(&player_id)])
                .await?
                .ok_or(RouteError::InvalidRequest(RequestError::new(
                    ErrorCode::AuthenticationInvalidToken,
                    format!("No player has the id '{player_id}'"),
                )))?)
        },
        async {
            Ok(QUERIES
                .prepare::<String>("get-player-permissions", &pg_client)
                .query_iter([dynamic(&player_id)])
                .await?
                .try_collect::<Vec<String>>()
                .await?)
        },
    );

    let (uuid, player_data) = match queries.await {
        (Ok((uuid, nickname)), Ok(permissions)) => {
            (uuid, PlayerData::new(uuid, nickname, permissions))
        }
        (Err(err), _) | (_, Err(err)) => return Err(err),
    };

    let server_address =
        ServerAddress::new(config.game_server_address.as_str(), config.game_server_port);

    let refresh_token =
        GameDataToken::new_refresh(player_id, uuid, config.game_api_refresh_token_duration);
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

    let token = ConnectionToken::generate(
        config.connection_token_key.into(),
        config.connection_token_duration,
        server_address,
        private_token,
    )
    .map_err(|_| RouteError::ServerError(ErrorCause::Internal, ErrorCode::TokenGenerationFailed))?;

    Ok(HttpResponse::Ok().json(token))
}
