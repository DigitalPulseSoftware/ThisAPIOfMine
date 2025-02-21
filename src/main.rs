use std::borrow::Cow;

use actix_governor::{Governor, GovernorConfig, GovernorConfigBuilder};
use actix_web::{middleware, web, App, HttpServer};
use cached::TimedCache;
use confy::ConfyError;
use tokio::sync::Mutex;
use tokio_postgres::NoTls;

use crate::app_data::AppData;
use crate::config::ApiConfig;
use crate::errors::Result;
use crate::fetcher::Fetcher;

mod app_data;
mod config;
mod data;
mod deku_helper;
mod errors;
mod fetcher;
mod game_data;
mod metaprog;
mod routes;

const CONFIG_FILE: Cow<'static, str> = Cow::Borrowed("tsom_api_config.toml");

async fn setup_pg_pool(api_config: &ApiConfig) -> Result<deadpool_postgres::Pool> {
    use deadpool_postgres::{Config, ManagerConfig, RecyclingMethod, Runtime};

    let mut pg_config = Config::new();
    pg_config.host = Some(api_config.db_host.clone());
    pg_config.password = Some(api_config.db_password.unsecure().to_string());
    pg_config.user = Some(api_config.db_user.clone());
    pg_config.dbname = Some(api_config.db_database.clone());
    pg_config.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });

    let pool = pg_config.create_pool(Some(Runtime::Tokio1), NoTls)?;

    // Try to connect to database to test if the database exist
    let _ = pool.get().await?;

    Ok(pool)
}

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
    let pg_pool = match setup_pg_pool(&config).await {
        Ok(pool) => web::Data::new(pool),
        Err(err) => {
            use deadpool_postgres::{CreatePoolError, PoolError};

            if err.is::<CreatePoolError>() {
                panic!("an error occured during the creation of the pool")
            } else if let Some(err) = err.downcast::<PoolError>() {
                panic!("failed to connect to database: {err}")
            } else {
                unreachable!()
            }
        }
    };

    let bind_address = format!("{}:{}", config.listen_address, config.listen_port);

    let data_config = web::Data::new(AppData {
        cache: Mutex::new(TimedCache::with_lifespan(config.cache_lifespan.as_secs())), // 5min
        fetcher,
    });
    let config = web::Data::new(config);

    let governor_conf = GovernorConfig::default();

    let player_create_governor_conf = GovernorConfigBuilder::default()
        .seconds_per_request(10)
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
