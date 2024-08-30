use core::marker::PhantomData;

use deadpool_postgres::Client;
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use tokio_postgres::types::BorrowToSql;
use tokio_postgres::{Error, Statement};

use crate::error::QueryError;
use crate::{FromRow, Query};

/// Prepare and send the request and allow to change the row value of each result
/// from the database with its generic `R`
pub struct Prepare<'c, R> {
    query: Query<'static>,
    client: &'c Client,
    _phantom: PhantomData<R>,
}

impl<'c, R: FromRow> Prepare<'c, R> {
    pub fn new(client: &'c Client, query: Query<'static>) -> Self {
        Self {
            query,
            client,
            _phantom: PhantomData,
        }
    }

    /// Return a Stream (it's like an async iterator) of all the row get by
    /// the query or a `PreparationFailed`.
    pub async fn query_iter<I, P>(
        &self,
        params: P,
    ) -> Result<impl Stream<Item = Result<R, Error>>, QueryError<R>>
    where
        I: BorrowToSql,
        P: IntoIterator<Item = I>,
        P::IntoIter: ExactSizeIterator,
    {
        let statement = self.prepare().await?;
        let result = self.client
            .query_raw(&statement, params)
            .await
            .map_err(|_| QueryError::PreparationFailed(self.query.clone()))?;

        Ok(result.map(|row| R::from_row(row?)))
    }

    /// Return the first result of the query.
    /// If the query result has only one row, it will be stored in `Ok(Some)`,
    /// if the query has no result, it will return `Ok(None)`,
    /// and if the query result has more than one row, the first row will be
    /// stored in `Err(QueryError::HasMoreThanOneRow)`.
    pub async fn query_one<I, P>(
        &self,
        params: P,
    ) -> Result<Option<R>, QueryError<R>>
    where
        I: BorrowToSql,
        P: IntoIterator<Item = I>,
        P::IntoIter: ExactSizeIterator,
    {
        let stream = self.query_iter(params).await?;
        pin_mut!(stream);

        let row = match stream.try_next().await {
            Ok(Some(row)) => row,
            Ok(None) => return Ok(None),
            Err(_) => return Err(QueryError::HasNoRow),
        };

        match stream.try_next().await {
            Ok(Some(_)) | Err(_) => return Err(QueryError::HasMoreThanOneRow(row)),
            Ok(None) => (),
        }

        Ok(Some(row))
    }

    /// Returns the only result of the query.
    /// If the query has no result, it will return `Err(HasNoRow)`, but if
    /// the query result has more than one row, the first row will be stored
    /// in `Err(QueryError::HasMoreThanOneRow)`.
    pub async fn query_single<I, P>(&self, params: P) -> Result<R, QueryError<R>>
    where
        I: BorrowToSql,
        P: IntoIterator<Item = I>,
        P::IntoIter: ExactSizeIterator,
    {
        let stream = self.query_iter(params).await?;
        pin_mut!(stream);

        let row = match stream.try_next().await {
            Ok(Some(row)) => row,
            _ => return Err(QueryError::HasNoRow),
        };

        match stream.try_next().await {
            Ok(Some(_)) | Err(_) => return Err(QueryError::HasMoreThanOneRow(row)),
            Ok(None) => (),
        }

        Ok(row)
    }

    /// Execute the query and return the amount of row update or `QueryError::ExecuteFailed`
    /// in case of error.
    pub async fn execute<I, P>(&self, params: P) -> Result<usize, QueryError<R>>
    where
        I: BorrowToSql,
        P: IntoIterator<Item = I>,
        P::IntoIter: ExactSizeIterator,
    {
        let statement = self.prepare().await?;
        match self.client.execute_raw(&statement, params).await {
            Ok(n) => Ok(n as usize),
            Err(err) => Err(QueryError::ExecuteFailed(err)),
        }
    }

    async fn prepare(&self) -> Result<Statement, QueryError<R>> {
        self.client
            .prepare_typed_cached(self.query.query(), self.query.types())
            .await
            .map_err(|_| QueryError::PreparationFailed(self.query.clone()))
    }
}
