use std::fmt;
use tokio_postgres::types::Type;

/// Representation of sql query,
/// store the query string and the types of all argument like a big pointer.
#[derive(Clone)]
pub struct Query<'q>(&'q str, &'q [Type]);

impl<'q> Query<'q> {
    #[inline]
    pub const fn params(query: &'q str, types: &'q [Type]) -> Self {
        Self(query, types)
    }

    #[inline]
    pub const fn new(query: &'q str) -> Self {
        Self(query, &[])
    }

    pub fn query(&self) -> &'q str {
        self.0
    }

    pub(crate) fn types(&self) -> &'q [Type] {
        self.1
    }
}

impl<'q> fmt::Display for Query<'q> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0, f)
    }
}

impl<'q> fmt::Debug for Query<'q> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.0, f)
    }
}
