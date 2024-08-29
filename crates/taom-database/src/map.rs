use core::borrow::Borrow;

use super::query::Query;

pub struct ConstQueryMap<K, const N: usize>([(K, Query<'static>); N]);

impl<K: Eq, const N: usize> ConstQueryMap<K, N> {
    pub const fn new(queries: [(K, Query<'static>); N]) -> Self {
        Self(queries)
    }

    pub fn prepare<Q>(&self, k: Q) -> &Query
    where
        K: Borrow<Q>,
        Q: PartialEq<K>
    {
        self.try_prepare(k).expect("to have element")
    } 

    pub fn try_prepare<Q>(&self, k: Q) -> Option<&Query>
    where
        K: Borrow<Q>,
        Q: PartialEq<K>
    {
        for (key, query) in &self.0 {
            if &k == key {
                return Some(query);
            }
        }

        None
    } 
}
