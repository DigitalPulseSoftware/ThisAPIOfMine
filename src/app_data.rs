use cached::TimedCache;
use tokio::sync::Mutex;

use crate::config::ApiConfig;
use crate::fetcher::Fetcher;
use crate::version::CachedReleased;

pub struct AppData {
    pub cache: Mutex<TimedCache<&'static str, CachedReleased>>,
    pub config: ApiConfig,
    pub fetcher: Fetcher,
}
