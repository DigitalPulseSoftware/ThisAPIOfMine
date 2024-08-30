use actix_web::{post, web, HttpResponse, Responder};

use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use taom_database::{dynamic, FromRow};
use uuid::Uuid;

use crate::config::ApiConfig;
use crate::data::token::Token;
use crate::database::QUERIES;
use crate::errors::api::ErrorCause;
use crate::errors::api::{ErrorCode, RequestError, RouteError};

#[derive(Deserialize)]
struct CreatePlayerParams {
    nickname: String,
}

#[derive(Serialize)]
struct CreatePlayerResponse {
    uuid: Uuid,
    token: Token,
}

#[post("/v1/players")]
async fn create(
    pg_pool: web::Data<deadpool_postgres::Pool>,
    config: web::Data<ApiConfig>,
    params: web::Json<CreatePlayerParams>,
) -> Result<impl Responder, RouteError> {
    let nickname = params.nickname.trim();

    if nickname.is_empty() {
        return Err(RouteError::InvalidRequest(RequestError::new(
            ErrorCode::NicknameEmpty,
            "Nickname cannot be empty".to_string(),
        )));
    }

    if nickname.len() > config.player_nickname_maxlength {
        return Err(RouteError::InvalidRequest(RequestError::new(
            ErrorCode::NicknameToolong,
            format!(
                "Nickname size exceeds maximum size of {}",
                config.player_nickname_maxlength
            ),
        )));
    }

    if !config.player_allow_non_ascii {
        if let Some(char) = nickname
            .chars()
            .find(|&x| !x.is_ascii_alphanumeric() && x != ' ' && x != '_')
        {
            return Err(RouteError::InvalidRequest(RequestError::new(
                ErrorCode::NicknameForbiddenCharacters,
                format!("Nickname can only have ascii characters (invalid character {char})"),
            )));
        }
    }

    let uuid = Uuid::new_v4();

    let pg_client = pg_pool.get().await?;

    let Ok(token) = Token::generate(OsRng) else {
        return Err(RouteError::ServerError(
            ErrorCause::Internal,
            ErrorCode::TokenGenerationFailed,
        ));
    };

    // let transaction = pg_client.transaction().await?;
    let player_id = QUERIES
        .prepare::<i32>("create-player")
        .query_single(&pg_client, [dynamic(&uuid), &nickname])
        .await?;

    QUERIES
        .prepare::<()>("create-token")
        .execute(&pg_client, [dynamic(&token), &player_id])
        .await?;
    // transaction.commit().await?;

    Ok(HttpResponse::Ok().json(CreatePlayerResponse { uuid, token }))
}

#[derive(Deserialize)]
struct AuthenticationParams {
    token: String,
}

#[derive(FromRow, Serialize)]
struct AuthenticationResponse {
    uuid: Uuid,
    nickname: String,
}

#[post("/v1/player/auth")]
async fn auth(
    pg_pool: web::Data<deadpool_postgres::Pool>,
    params: web::Json<AuthenticationParams>,
) -> Result<impl Responder, RouteError> {
    let pg_client = pg_pool.get().await?;
    let player_id = validate_player_token(&pg_client, &params.token).await?;

    let auth_response = QUERIES
        .prepare::<AuthenticationResponse>("find-play-info")
        .query_one(&pg_client, [dynamic(&player_id)])
        .await?
        .ok_or(RouteError::InvalidRequest(RequestError::new(
            ErrorCode::AuthenticationInvalidToken,
            format!("No player has the id '{player_id}'"),
        )))?;

    // Update last connection time in a separate task as its result won't affect the route
    tokio::spawn(async move { update_player_connection(&pg_client, player_id).await });

    Ok(HttpResponse::Ok().json(auth_response))
}

pub async fn validate_player_token(
    pg_client: &deadpool_postgres::Client,
    token: &str,
) -> Result<i32, RouteError> {
    if token.is_empty() {
        return Err(RouteError::InvalidRequest(RequestError::new(
            ErrorCode::EmptyToken,
            "The token is empty".to_string(),
        )));
    }

    if token.len() > 64 {
        return Err(RouteError::InvalidRequest(RequestError::new(
            ErrorCode::AuthenticationInvalidToken,
            format!("The given token '{token}' is invalid (too long)"),
        )));
    }

    let player_id = QUERIES
        .prepare::<i32>("find-token")
        .query_one(pg_client, [dynamic(&token)])
        .await?
        .ok_or(RouteError::InvalidRequest(RequestError::new(
            ErrorCode::AuthenticationInvalidToken,
            format!("No player has the token '{token}'"),
        )))?;

    Ok(player_id)
}

async fn update_player_connection(pg_client: &deadpool_postgres::Client, player_id: i32) {
    if let Err(err) = QUERIES
        .prepare::<()>("update-player-connection")
        .execute(pg_client, [dynamic(&player_id)])
        .await
    {
        log::error!("Failed to update player {player_id} connection time: {err}");
    }
}
