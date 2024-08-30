#[doc(hidden)]
pub use taom_database_macro::FromRow;
use tokio_postgres::types::FromSql;
use tokio_postgres::{Error, Row};

pub trait FromRow: Sized {
    fn from_row(row: Row) -> Result<Self, Error>;
}

impl FromRow for Row {
    #[inline]
    fn from_row(row: Row) -> Result<Self, Error> {
        Ok(row)
    }
}

macro_rules! impl_from_row_for_from_sql {
    ($($type:ty),+) => {
        $(
            impl FromRow for $type {
                #[inline]
                fn from_row(row: Row) -> Result<Self, Error> {
                    row.try_get(0)
                }
            }
        )+
    };
}

impl_from_row_for_from_sql!(i8, i16, i32, i64, String);

macro_rules! impl_from_row_for_tuple {
    () => {
        impl FromRow for () {
            #[inline]
            fn from_row(_: Row) -> Result<Self, Error> {
                Ok(())
            }
        }
    };
    ($($idx:literal -> $T:ident;)+) => {
        impl<$($T,)+> FromRow for ($($T,)+)
        where
            $($T: for<'r> FromSql<'r>,)+
        {
            #[inline]
            fn from_row(row: Row) -> Result<Self, Error> {
                Ok(($(row.try_get($idx as usize)?,)+))
            }
        }
    };
}

impl_from_row_for_tuple! {}
impl_from_row_for_tuple! {
    0 -> T1;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
    3 -> T4;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
    3 -> T4;
    4 -> T5;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
    3 -> T4;
    4 -> T5;
    5 -> T6;
}

impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
    3 -> T4;
    4 -> T5;
    5 -> T6;
    6 -> T7;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
    3 -> T4;
    4 -> T5;
    5 -> T6;
    6 -> T7;
    7 -> T8;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
    3 -> T4;
    4 -> T5;
    5 -> T6;
    6 -> T7;
    7 -> T8;
    8 -> T9;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
    3 -> T4;
    4 -> T5;
    5 -> T6;
    6 -> T7;
    7 -> T8;
    8 -> T9;
    9 -> T10;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
    3 -> T4;
    4 -> T5;
    5 -> T6;
    6 -> T7;
    7 -> T8;
    8 -> T9;
    9 -> T10;
    10 -> T11;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
    3 -> T4;
    4 -> T5;
    5 -> T6;
    6 -> T7;
    7 -> T8;
    8 -> T9;
    9 -> T10;
    10 -> T11;
    11 -> T12;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
    3 -> T4;
    4 -> T5;
    5 -> T6;
    6 -> T7;
    7 -> T8;
    8 -> T9;
    9 -> T10;
    10 -> T11;
    11 -> T12;
    12 -> T13;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
    3 -> T4;
    4 -> T5;
    5 -> T6;
    6 -> T7;
    7 -> T8;
    8 -> T9;
    9 -> T10;
    10 -> T11;
    11 -> T12;
    12 -> T13;
    13 -> T14;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
    3 -> T4;
    4 -> T5;
    5 -> T6;
    6 -> T7;
    7 -> T8;
    8 -> T9;
    9 -> T10;
    10 -> T11;
    11 -> T12;
    12 -> T13;
    13 -> T14;
    14 -> T15;
}
impl_from_row_for_tuple! {
    0 -> T1;
    1 -> T2;
    2 -> T3;
    3 -> T4;
    4 -> T5;
    5 -> T6;
    6 -> T7;
    7 -> T8;
    8 -> T9;
    9 -> T10;
    10 -> T11;
    11 -> T12;
    12 -> T13;
    13 -> T14;
    14 -> T15;
    15 -> T16;
}
