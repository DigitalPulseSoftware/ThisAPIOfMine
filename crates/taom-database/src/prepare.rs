use core::marker::PhantomData;

use deadpool_postgres::Client;
use futures::{Stream, StreamExt};
use tokio_postgres::types::ToSql;
use tokio_postgres::{Error, Statement};

use crate::{FromRow, Query};

/// Prepare and send the request and allow to change the row value of each result
/// from the database with its generic `R`
pub struct Prepare<R> {
    query: Query<'static>,
    _phantom: PhantomData<R>
}

impl<R: FromRow> Prepare<R> {
    pub fn new(query: Query<'static>) -> Self {
        Self {
            query,
            _phantom: PhantomData
        }
    }

    async fn prepare(&self, client: &Client) -> Result<Statement, Error> {
        client.prepare_typed_cached(self.query.query(), self.query.types()).await
    }

    pub async fn query_iter(&self, client: Client, params: Vec<&(dyn ToSql+Sync)>) -> Result<impl Stream, Error> {
        let statement = self.prepare(&client).await?;
        let result = client.query_raw(&statement, params).await?;

        Ok(result.map(|row| R::from_row(row?)))
    }
}
