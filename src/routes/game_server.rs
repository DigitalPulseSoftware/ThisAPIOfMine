use actix_web::{get, patch, post, web, HttpRequest, HttpResponse, Responder};
use jsonwebtoken::{decode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use tokio_postgres::types::Type;

use crate::config::ApiConfig;
use crate::data::game_data_token::GameDataToken;
use crate::errors::api::RouteError;
use crate::errors::codes::ServerErrorCode;

fn validate_token(
    req: &HttpRequest,
    config: &ApiConfig,
    token_type: &str,
) -> Result<GameDataToken, RouteError> {
    let header = req
        .headers()
        .get(actix_web::http::header::AUTHORIZATION)
        .ok_or(RouteError::InvalidRequest(
            ServerErrorCode::InvalidToken(None),
            "Missing token".to_string(),
        ))?;

    let jwt = header
        .to_str()
        .ok()
        .and_then(|str| str.strip_prefix("Bearer "))
        .ok_or_else(|| {
            log::error!("Token error, failed to transform AUTHORIZATION header to a string");
            RouteError::InvalidRequest(ServerErrorCode::InvalidToken(None), "Invalid token".into())
        })?;

    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.set_required_spec_claims(&["exp", "iat", "sub"]);

    let token = decode::<GameDataToken>(
        jwt,
        &DecodingKey::from_secret(config.game_api_secret.unsecure().as_bytes()),
        &validation,
    )
    .map_err(|err| {
        RouteError::InvalidRequest(
            ServerErrorCode::InvalidToken(Some(err.to_string())),
            "Invalid token".to_string(),
        )
    })?;

    if token.claims.sub != token_type {
        log::error!(
            "Expected {token_type} token but received {}",
            token.claims.sub
        );
        return Err(RouteError::InvalidRequest(
            ServerErrorCode::InvalidToken(None),
            format!("Expected {token_type} token"),
        ));
    }

    Ok(token.claims)
}

#[derive(Serialize)]
struct RefreshTokenResponse {
    access_token: String,
    access_token_expires_in: u64,
    refresh_token: String,
}

#[post("/game_server/v1/refresh")]
async fn refresh_access_token(
    req: HttpRequest,
    config: web::Data<ApiConfig>,
) -> Result<impl Responder, RouteError> {
    let refresh_token = validate_token(&req, &config, "refresh")?;

    let access_token = GameDataToken::new_access(
        refresh_token.player_db_id,
        refresh_token.player_uuid,
        config.game_api_access_token_duration,
    );
    let refresh_token = GameDataToken::new_refresh(
        refresh_token.player_db_id,
        refresh_token.player_uuid,
        config.game_api_refresh_token_duration,
    );

    let access_token_jwt = jsonwebtoken::encode(
        &Header::default(),
        &access_token,
        &EncodingKey::from_secret(config.game_api_secret.unsecure().as_bytes()),
    )?;
    let refresh_token_jwt = jsonwebtoken::encode(
        &Header::default(),
        &refresh_token,
        &EncodingKey::from_secret(config.game_api_secret.unsecure().as_bytes()),
    )?;

    Ok(HttpResponse::Ok().json(RefreshTokenResponse {
        access_token: access_token_jwt,
        access_token_expires_in: config.game_api_access_token_duration.as_secs(),
        refresh_token: refresh_token_jwt,
    }))
}

#[derive(Serialize)]
struct GetShipResponse {
    ship_data: serde_json::Value,
}

#[get("/game_server/v1/player_ship/{ship_slot}")]
async fn player_ship_get(
    req: HttpRequest,
    config: web::Data<ApiConfig>,
    path: web::Path<i32>,
    pg_pool: web::Data<deadpool_postgres::Pool>,
) -> Result<impl Responder, RouteError> {
    let access_token = validate_token(&req, &config, "access")?;

    let pg_client = pg_pool.get().await?;
    let get_player_ship = pg_client
        .prepare_typed_cached(
            "SELECT data FROM player_ships WHERE player_id = $1 AND slot = $2",
            &[Type::INT4, Type::INT4],
        )
        .await?;

    let row = pg_client
        .query_opt(&get_player_ship, &[&access_token.player_db_id, &*path])
        .await?;

    Ok(match row {
        Some(row) => HttpResponse::Ok().json(GetShipResponse {
            ship_data: row.get(0),
        }),
        None => HttpResponse::NotFound().finish(),
    })
}

#[derive(Deserialize)]
struct ShipPatchParams {
    data: serde_json::Value,
}

#[patch("/game_server/v1/player_ship/{ship_slot}")]
async fn player_ship_patch(
    req: HttpRequest,
    path: web::Path<i32>,
    params: web::Json<ShipPatchParams>,
    config: web::Data<ApiConfig>,
    pg_pool: web::Data<deadpool_postgres::Pool>,
) -> Result<impl Responder, RouteError> {
    let access_token = validate_token(&req, &config, "access")?;

    let pg_client = pg_pool.get().await?;
    let insert_player_ship = pg_client
        .prepare_typed_cached(
            "INSERT INTO player_ships(player_id, slot, last_update, data) VALUES($1, $2, NOW(), $3) ON CONFLICT(player_id, slot) DO UPDATE SET last_update = NOW(), data = EXCLUDED.data",
            &[Type::INT4, Type::INT4, Type::JSONB],
        )
        .await?;

    pg_client
        .execute(
            &insert_player_ship,
            &[&access_token.player_db_id, &*path, &params.data],
        )
        .await?;

    Ok(HttpResponse::Ok().finish())
}
