use tokio_postgres::Error;

pub enum PoolError {
    Creation,
    Connection,
}

pub enum QueryError<R> {
    PreparationFailed,
    HasMoreThenOneRow(/*first_row:*/ R),
    ExecuteFailed(Error),
    HasNoRow,
    GetRow,
}
