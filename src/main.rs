use std::collections::HashMap;
use std::sync::Mutex;

use actix_web::{get, middleware, web, App, HttpServer};
use actix_web::{HttpResponse, Responder};
use cached::{CachedAsync, TimedCache};
use game_data::{Asset, GameRelease};
use serde::Deserialize;

use crate::config::ApiConfig;
use crate::fetcher::Fetcher;
use crate::game_data::GameVersion;

mod config;
mod fetcher;
mod game_data;

#[derive(Deserialize)]
struct VersionQuery {
    platform: String,
}

struct AppData {
    cache: Mutex<TimedCache<&'static str, CachedReleased>>,
    config: ApiConfig,
    fetcher: Fetcher,
}

#[derive(Clone)]
enum CachedReleased {
    Updater(HashMap<String, Asset>),
    Game(GameRelease),
}

#[get("/game_version")]
async fn game_version(
    app_data: web::Data<AppData>,
    ver_query: web::Query<VersionQuery>,
) -> impl Responder {
    let AppData {
        cache,
        config,
        fetcher,
    } = app_data.as_ref();
    let mut cache = cache.lock().unwrap();

    // TODO: remove .cloned
    let Ok(CachedReleased::Updater(updater_release)) = cache
        .try_get_or_set_with("latest_updater_release", || async {
            fetcher
                .get_latest_updater_release()
                .await
                .map(CachedReleased::Updater)
        })
        .await
        .cloned()
    else {
        return HttpResponse::InternalServerError().finish();
    };

    // TODO: remove .cloned
    let Ok(CachedReleased::Game(game_release)) = cache
        .try_get_or_set_with("latest_game_release", || async {
            fetcher
                .get_latest_game_release()
                .await
                .map(CachedReleased::Game)
        })
        .await
        .cloned()
    else {
        return HttpResponse::InternalServerError().finish();
    };

    let updater_filename = format!("{}_{}", ver_query.platform, config.updater_filename);

    let (Some(updater), Some(binary)) = (updater_release.get(&updater_filename), game_release.binaries.get(&ver_query.platform)) else {
        eprintln!(
            "no updater or game binary release found for platform {}",
            ver_query.platform
        );
        return HttpResponse::NotFound().finish();
    };

    HttpResponse::Ok().json(web::Json(GameVersion {
        assets: game_release.assets,
        assets_version: game_release.assets_version.to_string(),
        binaries: binary.clone(),
        updater: updater.clone(),
        version: game_release.version.to_string(),
    }))
}

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    let config: ApiConfig = confy::load_path("tsom_api_config.toml").unwrap();
    let fetcher = Fetcher::from_config(&config).unwrap();

    std::env::set_var("RUST_LOG", "info,actix_web=info");
    env_logger::init();

    let bind_address = format!("{}:{}", config.listen_address, config.listen_port);

    let data_config = web::Data::new(AppData {
        cache: Mutex::new(TimedCache::with_lifespan(config.cache_lifespan)), // 5min
        config,
        fetcher,
    });

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(data_config.clone())
            .service(game_version)
    })
    .bind(bind_address)?
    .run()
    .await
}
