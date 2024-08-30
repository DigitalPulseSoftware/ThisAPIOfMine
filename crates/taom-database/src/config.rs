use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;

use crate::error::PoolError;

#[derive(Default)]
pub struct ConfigBuilder<'b> {
    host: Option<&'b str>,
    password: Option<&'b str>,
    user: Option<&'b str>,
    database: Option<&'b str>,
    recycling_method: RecyclingMethod,
    runtime: Option<Runtime>,
}

impl<'b> ConfigBuilder<'b> {
    pub fn host(mut self, host: &'b str) -> Self {
        self.host = Some(host);
        self
    }
    pub fn password(mut self, password: &'b str) -> Self {
        self.password = Some(password);
        self
    }
    pub fn user(mut self, user: &'b str) -> Self {
        self.user = Some(user);
        self
    }
    pub fn database(mut self, database: &'b str) -> Self {
        self.database = Some(database);
        self
    }
    pub fn recycling_method(mut self, recycling_method: RecyclingMethod) -> Self {
        self.recycling_method = recycling_method;
        self
    }
    pub fn runtime(mut self, runtime: Runtime) -> Self {
        self.runtime = Some(runtime);
        self
    }

    pub async fn build(self) -> Result<Pool, PoolError> {
        let mut pg_config = Config::new();
        pg_config.host = self.host.map(str::to_string);
        pg_config.password = self.password.map(str::to_string);
        pg_config.user = self.user.map(str::to_string);
        pg_config.dbname = self.database.map(str::to_string);
        pg_config.manager = Some(ManagerConfig {
            recycling_method: self.recycling_method,
        });

        let pool = pg_config
            .create_pool(self.runtime, NoTls)
            .map_err(|_| PoolError::Creation)?;

        // Try to connect to database to test if the database exist
        let _ = pool.get().await.map_err(|_| PoolError::Connection)?;

        Ok(pool)
    }
}
