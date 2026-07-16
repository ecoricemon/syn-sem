/// Extension helpers for order-preserving deduplication on small vectors.
pub trait VecUniqueExt<T> {
    /// Pushes `value` only when the vector does not already contain it.
    ///
    /// Returns whether the value was inserted.
    fn push_unique(&mut self, value: T) -> bool
    where
        T: PartialEq;

    /// Pushes `value` only when no existing item has the same derived key.
    ///
    /// Returns whether the value was inserted.
    fn push_unique_by_key<K>(&mut self, value: T, key: impl Fn(&T) -> K) -> bool
    where
        K: PartialEq;
}

impl<T> VecUniqueExt<T> for Vec<T> {
    fn push_unique(&mut self, value: T) -> bool
    where
        T: PartialEq,
    {
        if self.contains(&value) {
            return false;
        }
        self.push(value);
        true
    }

    fn push_unique_by_key<K>(&mut self, value: T, key: impl Fn(&T) -> K) -> bool
    where
        K: PartialEq,
    {
        let value_key = key(&value);
        if self.iter().any(|existing| key(existing) == value_key) {
            return false;
        }
        self.push(value);
        true
    }
}
