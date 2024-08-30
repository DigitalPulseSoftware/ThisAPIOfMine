// allow to write `fn foo<T = Bar>();`
//                        ^^^^^^^
#![allow(invalid_type_param_default)]

pub use from_row::FromRow;
pub use map::ConstQueryMap;
pub use query::Query;
use tokio_postgres::types::ToSql;

mod from_row;
mod map;
mod prepare;
mod query;

#[inline(always)]
pub fn dynamic<'a, T: ToSql + Sync>(v: &'a T) -> &'a (dyn ToSql + Sync) {
    v
}
