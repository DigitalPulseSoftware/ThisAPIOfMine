use core::marker::PhantomData;

use deadpool_postgres::Client;
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use tokio_postgres::types::ToSql;
use tokio_postgres::{Error, Statement};

use crate::error::QueryError;
use crate::{FromRow, Query};

/// Prepare and send the request and allow to change the row value of each result
/// from the database with its generic `R`
pub struct Prepare<R> {
    query: Query<'static>,
    _phantom: PhantomData<R>,
}

impl<R: FromRow> Prepare<R> {
    pub fn new(query: Query<'static>) -> Self {
        Self {
            query,
            _phantom: PhantomData,
        }
    }

    /// Return a Stream (it's like an async iterator) of all the row get by
    /// the query or a `PreparationFailed`.
    pub async fn query_iter<'a, P>(
        &self,
        client: Client,
        params: P,
    ) -> Result<impl Stream<Item = Result<R, Error>>, QueryError<R>>
    where
        P: IntoIterator<Item = &'a (dyn ToSql + Sync)>,
        P::IntoIter: ExactSizeIterator<Item = &'a (dyn ToSql + Sync)>,
    {
        let statement = self.prepare(&client).await?;
        let result = client
            .query_raw(&statement, params)
            .await
            .map_err(|_| QueryError::PreparationFailed)?;

        Ok(result.map(|row| R::from_row(row?)))
    }

    /// Return the first result of the query.
    /// If the query result has only one row, it will be stored in `Ok(Some)`,
    /// if the query has no result, it will return `Ok(None)`,
    /// and if the query result has more than one row, the first row will be
    /// stored in `Err(QueryError::HasMoreThenOneRow)`.
    pub async fn query_one<'a, P>(
        &self,
        client: Client,
        params: P,
    ) -> Result<Option<R>, QueryError<R>>
    where
        P: IntoIterator<Item = &'a (dyn ToSql + Sync + 'a)>,
        P::IntoIter: ExactSizeIterator<Item = &'a (dyn ToSql + Sync + 'a)>,
    {
        let stream = self.query_iter(client, params).await?;
        pin_mut!(stream);

        let row = match stream.try_next().await {
            Ok(Some(row)) => row,
            Ok(None) => return Ok(None),
            Err(_) => return Err(QueryError::GetRow),
        };

        match stream.try_next().await {
            Ok(Some(_)) | Err(_) => return Err(QueryError::HasMoreThenOneRow(row)),
            Ok(None) => (),
        }

        Ok(Some(row))
    }

    /// Execute the query and return the amount of row update or `QueryError::ExecuteFailed`
    /// in case of error.
    pub async fn execute<'a, P>(&self, client: Client, params: P) -> Result<usize, QueryError<R>>
    where
        P: IntoIterator<Item = &'a (dyn ToSql + Sync + 'a)>,
        P::IntoIter: ExactSizeIterator<Item = &'a (dyn ToSql + Sync + 'a)>,
    {
        let statement = self.prepare(&client).await?;
        match client.execute_raw(&statement, params).await {
            Ok(n) => Ok(n as usize),
            Err(err) => Err(QueryError::ExecuteFailed(err)),
        }
    }

    async fn prepare(&self, client: &Client) -> Result<Statement, QueryError<R>> {
        client
            .prepare_typed_cached(self.query.query(), self.query.types())
            .await
            .map_err(|_| QueryError::PreparationFailed)
    }
}
