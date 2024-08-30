use core::marker::PhantomData;

use deadpool_postgres::Client;
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
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

    pub async fn query_iter<'a, P>(&self, client: Client, params: P) -> Result<impl Stream<Item = Result<R, Error>>, Error>
    where
        P: IntoIterator<Item = &'a (dyn ToSql + Sync)>,
        P::IntoIter: ExactSizeIterator<Item = &'a (dyn ToSql + Sync)>
    {
        let statement = self.prepare(&client).await?;
        let result = client.query_raw(&statement, params).await?;

        Ok(result.map(|row| R::from_row(row?)))
    }

    pub async fn query_one<'a, P>(&self, client: Client, params: P) -> Result<Option<R>, Error>
    where
        P: IntoIterator<Item = &'a (dyn ToSql + Sync + 'a)>,
        P::IntoIter: ExactSizeIterator<Item = &'a (dyn ToSql + Sync + 'a)>,
    {
        let stream = self.query_iter(client, params).await?;
        pin_mut!(stream);

        let row = match stream.try_next().await? {
            Some(row) => row,
            None => return Ok(None),
        };

        // if stream.try_next().await?.is_some() {
        //     return Err(InternalError::HasMoreThenOneRow(row));
        // }

        Ok(Some(row))
    }

    pub async fn execute<'a, P>(&self, client: Client, params: P) -> Result<usize, Error>
    where
        P: IntoIterator<Item = &'a (dyn ToSql + Sync + 'a)>,
        P::IntoIter: ExactSizeIterator<Item = &'a (dyn ToSql + Sync + 'a)>,
    {
        let statement = self.prepare(&client).await?;
        client.execute_raw(&statement, params).await.map(|n| n as usize)
    }

    async fn prepare(&self, client: &Client) -> Result<Statement, Error> {
        client.prepare_typed_cached(self.query.query(), self.query.types()).await
    }
}
