use std::borrow::Cow;

use actix_governor::{Governor, GovernorConfig, GovernorConfigBuilder};
use actix_web::{middleware, web, App, HttpServer};
use cached::TimedCache;
use confy::ConfyError;
use deadpool_postgres::{RecyclingMethod, Runtime};
use taom_database::config::ConfigBuilder;
use taom_database::error::PoolError;
use tokio::sync::Mutex;

use crate::app_data::AppData;
use crate::config::ApiConfig;
use crate::errors::Result;
use crate::fetcher::Fetcher;

mod app_data;
mod config;
mod data;
mod database;
mod deku_helper;
mod errors;
mod fetcher;
mod game_data;
mod metaprog;
mod routes;

const CONFIG_FILE: Cow<'static, str> = Cow::Borrowed("tsom_api_config.toml");

#[actix_web::main]
async fn main() -> Result<(), std::io::Error> {
    std::env::set_var("RUST_LOG", "info,actix_web=info");
    env_logger::init();

    let mut args = std::env::args();
    args.next(); // skip the executable name

    let config_file = args.next().map(Cow::Owned).unwrap_or(CONFIG_FILE);

    log::info!("Reading the config file {config_file}");
    let config = match confy::load_path::<ApiConfig>(config_file.as_ref()) {
        Ok(config) => config,
        Err(ConfyError::BadTomlData(err)) => panic!(
            "an error occured on the parsing of the file {config_file}:\n{}",
            err.message()
        ),
        Err(ConfyError::GeneralLoadError(err)) => panic!(
            "an error occured on the loading of the file {config_file}:\n{}",
            err.kind()
        ),
        Err(_) => {
            panic!("wrong data in the file, failed to load config, please check {config_file}")
        }
    };
    let fetcher = Fetcher::from_config(&config).unwrap();

    log::info!("Connection to the database");
    let pg_pool = ConfigBuilder::default()
        .host(config.db_host.as_str())
        .password(config.db_password.unsecure())
        .user(config.db_user.as_str())
        .database(config.db_database.as_str())
        .recycling_method(RecyclingMethod::Fast)
        .runtime(Runtime::Tokio1)
        .build()
        .await;

    let pg_pool = match pg_pool {
        Ok(pool) => web::Data::new(pool),
        Err(PoolError::Creation) => panic!("an error occured during the creation of the pool"),
        Err(PoolError::Connection) => panic!("failed to connect to database"),
    };

    let bind_address = format!("{}:{}", config.listen_address, config.listen_port);

    let data_config = web::Data::new(AppData {
        cache: Mutex::new(TimedCache::with_lifespan(config.cache_lifespan.as_secs())), // 5min
        fetcher,
    });
    let config = web::Data::new(config);

    let governor_conf = GovernorConfig::default();

    let player_create_governor_conf = GovernorConfigBuilder::default()
        .per_second(10)
        .burst_size(1)
        .finish()
        .unwrap();

    log::info!("Server starting at the address {bind_address}");
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(Governor::new(&governor_conf))
            .app_data(data_config.clone())
            .app_data(config.clone())
            .app_data(pg_pool.clone())
            .service(routes::version::game_version)
            .service(routes::players::auth)
            .service(routes::connection::game_connect)
            .service(routes::game_server::refresh_access_token)
            .service(routes::game_server::player_ship_get)
            .service(routes::game_server::player_ship_patch)
            .service(
                web::scope("")
                    .wrap(Governor::new(&player_create_governor_conf))
                    .service(routes::players::create),
            )
    })
    .bind(bind_address)?
    .run()
    .await
}
