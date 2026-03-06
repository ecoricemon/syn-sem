use crate::Map;
use std::{
    collections::{hash_map::Entry, vec_deque, VecDeque},
    fmt,
    hash::Hash,
};

pub trait Identify {
    type Id: Eq + Hash;

    fn id(&self) -> Self::Id;
}

pub struct OnceQueue<T: Identify> {
    queue: VecDeque<T>,

    /// Stores the number of identical values in the queue.
    cnt: Map<T::Id, u32>,
}

impl<T: Identify> OnceQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            cnt: Map::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears the queue itself and insertion history.
    pub fn reset(&mut self) {
        self.queue.clear();
        self.cnt.clear();
    }

    pub fn iter(&self) -> vec_deque::Iter<'_, T> {
        self.queue.iter()
    }

    pub fn is_pushable(&self, value: &T) -> bool {
        !self.cnt.contains_key(&value.id())
    }

    pub fn count(&self, value: &T) -> u32 {
        self.cnt.get(&value.id()).cloned().unwrap_or(0)
    }

    /// Appends the value at the end of the queue if the queue has not seen the value before.
    ///
    /// If the queue has seen the value, then returns it within error.
    pub fn push_back(&mut self, value: T) -> Result<(), T> {
        self.push(value, |queue, value| queue.push_back(value))
    }

    /// Appends the value at the beginning of the queue if the queue has not seen the value before.
    ///
    /// If the queue has seen the value, then returns it within error.
    pub fn push_front(&mut self, value: T) -> Result<(), T> {
        self.push(value, |queue, value| queue.push_front(value))
    }

    /// Appends the value at the end of the queue regardless of whether the queue has seen the
    /// value before.
    pub fn push_back_force(&mut self, value: T) {
        self.push_force(value, |queue, value| queue.push_back(value))
    }

    /// Appends the value at the beginning of the queue regardless of whether the queue has seen
    /// the value before.
    pub fn push_front_force(&mut self, value: T) {
        self.push_force(value, |queue, value| queue.push_front(value))
    }

    /// Removes the last value, then returns it.
    pub fn pop_back(&mut self) -> Option<T> {
        self.pop(|queue| queue.pop_back())
    }

    /// Removes the first value, then returns it.
    pub fn pop_front(&mut self) -> Option<T> {
        self.pop(|queue| queue.pop_front())
    }

    fn push<F>(&mut self, value: T, push: F) -> Result<(), T>
    where
        F: FnOnce(&mut VecDeque<T>, T),
    {
        match self.cnt.entry(value.id()) {
            Entry::Vacant(entry) => {
                push(&mut self.queue, value);
                entry.insert(1);
                Ok(())
            }
            // regardless of count, seen value cannot be added.
            Entry::Occupied(_) => Err(value),
        }
    }

    fn push_force<F>(&mut self, value: T, push_force: F)
    where
        F: FnOnce(&mut VecDeque<T>, T),
    {
        self.cnt
            .entry(value.id())
            .and_modify(|c| *c += 1)
            .or_insert(1);
        push_force(&mut self.queue, value);
    }

    fn pop<F>(&mut self, pop: F) -> Option<T>
    where
        F: FnOnce(&mut VecDeque<T>) -> Option<T>,
    {
        let value = pop(&mut self.queue)?;

        let c = self.cnt.get_mut(&value.id()).unwrap();
        *c -= 1;

        Some(value)
    }
}

impl<T: Identify> Default for OnceQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Identify + fmt::Debug> fmt::Debug for OnceQueue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.queue.fmt(f)
    }
}

impl<'a, T: Identify> IntoIterator for &'a OnceQueue<T> {
    type Item = &'a T;
    type IntoIter = vec_deque::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
