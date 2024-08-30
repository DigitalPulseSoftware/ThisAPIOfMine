use deadpool_postgres::Client;
use tokio_postgres::Row;

use crate::prepare::Prepare;
use crate::FromRow;

use super::query::Query;

pub struct ConstQueryMap<K, const N: usize>([(K, Query<'static>); N]);

unsafe impl<K: Send + Sync, const N: usize> Sync for ConstQueryMap<K, N> {}

impl<K: Eq, const N: usize> ConstQueryMap<K, N> {
    pub const fn new(queries: [(K, Query<'static>); N]) -> Self {
        Self(queries)
    }

    pub fn prepare<'c, R: FromRow = Row>(&self, k: K, client: &'c Client) -> Prepare<'c, R> {
        self.try_prepare::<R>(k, client).expect("item should exist")
    }

    pub fn try_prepare<'c, R: FromRow = Row>(&self, k: K, client: &'c Client) -> Option<Prepare<'c, R>> {
        for (key, query) in &self.0 {
            if &k == key {
                return Some(Prepare::new(client, query.to_owned()));
            }
        }

        None
    }
}
