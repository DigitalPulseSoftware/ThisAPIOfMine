#[doc(hidden)]
pub use taom_database_macro::FromRow;
use tokio_postgres::{Error, Row};

pub trait FromRow: Sized {
    fn from_row(row: Row) -> Result<Self, Error>;
}
