use core::fmt::Display;

use tokio_postgres::Error;

use crate::Query;

pub enum PoolError {
    Creation,
    Connection,
}

#[derive(Debug)]
pub enum QueryError<R> {
    PreparationFailed(Query<'static>),
    HasMoreThanOneRow(/*first_row:*/ R),
    ExecuteFailed(Error),
    HasNoRow,
}

impl<R> Display for QueryError<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PreparationFailed(_) => write!(f, "failure during the preparation of the query"),
            Self::HasMoreThanOneRow(_) => write!(f, "collect more than one result"),
            Self::ExecuteFailed(err) => write!(f, "{err}"),
            Self::HasNoRow => write!(f, "no row found"),
        }
    }
}
