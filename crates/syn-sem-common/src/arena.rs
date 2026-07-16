use std::ops::{Index, IndexMut};

/// Builder for dense id-indexed arenas that need parent-first reservation.
///
/// `ArenaBuilder` stores incomplete slots during construction, then freezes into a dense `Vec<T>`
/// once every reserved id has been filled. This keeps partial arena state out of finalized query
/// data structures.
#[derive(Debug)]
pub struct ArenaBuilder<I, T> {
    slots: Vec<Option<T>>,
    new_id: fn(usize) -> I,
    id_index: fn(I) -> usize,
}

impl<I, T> ArenaBuilder<I, T>
where
    I: Copy,
{
    /// Creates an empty arena builder using crate-local id conversion functions.
    pub fn new(new_id: fn(usize) -> I, id_index: fn(I) -> usize) -> Self {
        Self {
            slots: Vec::new(),
            new_id,
            id_index,
        }
    }

    /// Returns the id that would be reserved or pushed next.
    pub fn next_id(&self) -> I {
        (self.new_id)(self.slots.len())
    }

    /// Reserves one slot and returns its id.
    pub fn reserve(&mut self) -> I {
        let id = self.next_id();
        self.slots.push(None);
        id
    }

    /// Pushes a fully constructed value and returns its id.
    pub fn push(&mut self, value: T) -> I {
        let id = self.reserve();
        self.fill(id, value);
        id
    }

    /// Fills a reserved slot.
    ///
    /// Panics if `id` does not name an existing reserved slot or if the slot was
    /// already filled.
    pub fn fill(&mut self, id: I, value: T) {
        let index = (self.id_index)(id);
        let slot = self
            .slots
            .get_mut(index)
            .expect("arena id must name a reserved slot");
        assert!(slot.is_none(), "arena slot must be filled at most once");
        *slot = Some(value);
    }

    /// Finishes construction and returns a dense arena.
    ///
    /// Panics if any reserved slot was left unfilled.
    pub fn finish(self) -> Vec<T> {
        self.slots
            .into_iter()
            .map(|slot| slot.expect("arena slot must be filled before finish"))
            .collect()
    }

    fn slot(&self, id: I) -> &Option<T> {
        &self.slots[(self.id_index)(id)]
    }

    fn slot_mut(&mut self, id: I) -> &mut Option<T> {
        &mut self.slots[(self.id_index)(id)]
    }
}

impl<I, T> Index<I> for ArenaBuilder<I, T>
where
    I: Copy,
{
    type Output = T;

    fn index(&self, id: I) -> &Self::Output {
        self.slot(id).as_ref().expect("arena slot must be filled")
    }
}

impl<I, T> IndexMut<I> for ArenaBuilder<I, T>
where
    I: Copy,
{
    fn index_mut(&mut self, id: I) -> &mut Self::Output {
        self.slot_mut(id)
            .as_mut()
            .expect("arena slot must be filled")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct TestId(usize);

    impl TestId {
        fn new(index: usize) -> Self {
            Self(index)
        }

        fn index(self) -> usize {
            self.0
        }
    }

    #[test]
    fn reserves_ids_then_freezes_dense_values() {
        // Proves reserved ids can be filled before freezing into dense arena values.
        let mut arena = ArenaBuilder::new(TestId::new, TestId::index);
        let parent = arena.reserve();
        let child = arena.push("child");

        arena.fill(parent, "parent");

        assert_eq!(parent.index(), 0);
        assert_eq!(child.index(), 1);
        assert_eq!(arena[parent], "parent");
        assert_eq!(arena[child], "child");
        assert_eq!(arena.finish(), vec!["parent", "child"]);
    }

    #[test]
    #[should_panic(expected = "arena slot must be filled before finish")]
    fn finish_rejects_unfilled_slots() {
        // Proves `finish` rejects any reserved slot that was never filled.
        let mut arena: ArenaBuilder<TestId, &str> = ArenaBuilder::new(TestId::new, TestId::index);
        arena.reserve();

        let _ = arena.finish();
    }

    #[test]
    #[should_panic(expected = "arena slot must be filled at most once")]
    fn fill_rejects_double_fill() {
        // Proves each reserved slot can be filled only once.
        let mut arena = ArenaBuilder::new(TestId::new, TestId::index);
        let id = arena.reserve();

        arena.fill(id, "first");
        arena.fill(id, "second");
    }
}
