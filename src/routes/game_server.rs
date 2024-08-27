use actix_web::{get, patch, post, web, HttpRequest, HttpResponse, Responder};
use jsonwebtoken::{decode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_postgres::types::Type;

use crate::data::game_data_token::GameDataToken;
use crate::errors::api::{ErrorCode, RequestError};
use crate::{config::ApiConfig, errors::api::RouteError};

fn validate_token(
    req: &HttpRequest,
    config: &ApiConfig,
    token_type: &str,
) -> Result<GameDataToken, RouteError> {
    let jwt: &str = match req.headers().get(actix_web::http::header::AUTHORIZATION) {
        Some(value) => match value.to_str() {
            Ok(str) => match str.strip_prefix("Bearer ") {
                Some(str) => str,
                None => {
                    return Err(RouteError::InvalidRequest(RequestError::new(
                        ErrorCode::InvalidToken,
                        "invalid token".into(),
                    )))
                }
            },
            Err(err) => {
                log::error!("Token error, failed to transform to string: {}", err);
                return Err(RouteError::InvalidRequest(RequestError::new(
                    ErrorCode::InvalidToken,
                    "invalid token".into(),
                )));
            }
        },
        None => {
            return Err(RouteError::InvalidRequest(RequestError::new(
                ErrorCode::InvalidToken,
                "missing token".into(),
            )))
        }
    };

    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.set_required_spec_claims(&["exp", "iat", "sub"]);

    match decode::<GameDataToken>(
        &jwt,
        &DecodingKey::from_secret(config.game_api_secret.unsecure().as_bytes()),
        &validation,
    ) {
        Ok(token) => {
            if token.claims.sub != token_type {
                return Err(RouteError::InvalidRequest(RequestError::new(
                    ErrorCode::InvalidToken,
                    format!("Expected {} token", token_type),
                )));
            }

            Ok(token.claims)
        }
        Err(err) => {
            return Err(RouteError::InvalidRequest(RequestError::new(
                ErrorCode::InvalidToken,
                "Invalid token".to_string(),
            )));
        }
    }
}

#[derive(Serialize)]
struct RefreshTokenResponse {
    access_token: String,
    access_token_expires_in: u64,
    refresh_token: String,
}

#[post("/game_server/v1/refresh")]
async fn game_server_refresh_token(
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
    )
    .unwrap();
    let refresh_token_jwt = jsonwebtoken::encode(
        &Header::default(),
        &refresh_token,
        &EncodingKey::from_secret(config.game_api_secret.unsecure().as_bytes()),
    )
    .unwrap();

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
async fn game_server_player_ship_get(
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
        .query_one(&get_player_ship, &[&access_token.player_db_id, &*path])
        .await?;

    let ship_data = row.get(0);

    Ok(HttpResponse::Ok().json(GetShipResponse {
        ship_data,
    }))
}

#[derive(Deserialize)]
struct ShipPatchParams {
    data: serde_json::Value,
}

#[patch("/game_server/v1/player_ship/{ship_slot}")]
async fn game_server_player_ship_patch(
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
            "INSERT INTO player_ships(player_id, slot, data) VALUES($1, $2, $3) ON CONFLICT(player_id, slot) DO UPDATE SET data = EXCLUDED.data",
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
