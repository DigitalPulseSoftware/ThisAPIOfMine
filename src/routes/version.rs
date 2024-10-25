use actix_web::{get, web};
use actix_web::{HttpResponse, Responder};
use cached::CachedAsync;
use serde::Deserialize;

use crate::app_data::AppData;
use crate::config::ApiConfig;
use crate::errors::api::{ErrorCause, RouteError};
use crate::errors::codes::ServerErrorCode;
use crate::game_data::{Assets, GameRelease, GameVersion};

#[derive(Deserialize)]
struct VersionQuery {
    platform: String,
}

#[derive(Clone)]
pub(crate) enum CachedReleased {
    Updater(Assets),
    Game(GameRelease),
}

#[get("/game_version")]
async fn game_version(
    app_data: web::Data<AppData>,
    config: web::Data<ApiConfig>,
    ver_query: web::Query<VersionQuery>,
) -> Result<impl Responder, RouteError> {
    let VersionQuery { platform } = ver_query.0;
    let AppData { cache, fetcher } = app_data.as_ref();
    let mut cache = cache.lock().await;

    // TODO: remove .cloned
    let results_updater_release = cache
        .try_get_or_set_with("latest_updater_release", || async {
            fetcher
                .get_latest_updater_release()
                .await
                .map(CachedReleased::Updater)
        })
        .await
        .cloned();
    let updater_release = match results_updater_release {
        Ok(CachedReleased::Updater(updater_release)) => updater_release,
        Ok(CachedReleased::Game(_)) => unreachable!(),
        Err(err) => return Err(RouteError::ServerError(ErrorCause::Internal, err.into())),
    };

    // TODO: remove .cloned
    let results_game_release = cache
        .try_get_or_set_with("latest_game_release", || async {
            fetcher
                .get_latest_game_release()
                .await
                .map(CachedReleased::Game)
        })
        .await
        .cloned();
    let game_release = match results_game_release {
        Ok(CachedReleased::Game(game_release)) => game_release,
        Ok(CachedReleased::Updater(_)) => unreachable!(),
        Err(err) => return Err(RouteError::ServerError(ErrorCause::Internal, err.into())),
    };

    let updater_filename = format!("{}_{}", platform, config.updater_filename);

    let (Some(updater), Some(binary)) = (
        updater_release.get(&updater_filename),
        game_release.binaries.get(&platform),
    ) else {
        let msg = format!("No updater or game binary release found for platform '{platform}'");
        log::error!("{msg}");
        return Err(RouteError::InvalidRequest(
            ServerErrorCode::NotFoundPlatform(platform),
            msg,
        ));
    };

    Ok(HttpResponse::Ok().json(GameVersion {
        assets: game_release.assets,
        assets_version: game_release.assets_version.to_string(),
        binaries: binary.clone(),
        updater: updater.clone(),
        version: game_release.version.to_string(),
    }))
}
