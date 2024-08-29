use tokio_postgres::types::Type;

pub struct Query<'q> (&'q str, &'q [Type]);

impl<'q> Query<'q> {
    #[inline]
    pub fn params(query: &'q str, types: &'q [Type]) -> Self {
        Self(query, types)
    }
    
    #[inline]
    pub fn new(query: &'q str) -> Self {
        Self(query, &[])
    }

    pub fn query(&self) -> &'q str {
        self.0
    }

    pub(crate) fn types(&self) -> &'q [Type] {
        self.1
    }
}
