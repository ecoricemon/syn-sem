use my_utils::ds::OptVec;
use std::{
    fmt::{self, Debug},
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone)]
pub struct GenOptVec<T> {
    inner: OptVec<GenValue<T>>,
    gen_: u64,
}

impl GenOptVec<()> {
    const GEN_IGNORE: u64 = 0;
}

impl<T> GenOptVec<T> {
    pub fn new() -> Self {
        Self {
            inner: OptVec::new(),
            gen_: 1,
        }
    }

    pub fn len(&self) -> usize {
        // Number of occupied slots
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn truncate(&mut self, len: usize) {
        self.inner.truncate(len);
    }

    pub fn next_index(&self) -> GenIndex {
        GenIndex {
            index: self.inner.next_index(),
            gen_: self.gen_,
        }
    }

    pub fn add(&mut self, value: T) -> GenIndex {
        let gen_ = self.gen_;
        self.gen_ += 1;

        let gen_value = GenValue { value, gen_ };

        let index = self.inner.add(gen_value);
        GenIndex { index, gen_ }
    }

    pub fn take(&mut self, index: GenIndex) -> Option<T> {
        let GenIndex {
            index,
            gen_: in_gen,
        } = index;
        let gv = self.inner.take(index)?;

        if in_gen == gv.gen_ || in_gen == GenOptVec::GEN_IGNORE {
            Some(gv.value)
        } else {
            self.inner.set(index, Some(gv));
            None
        }
    }

    pub fn get(&self, index: GenIndex) -> Option<&T> {
        let GenIndex {
            index,
            gen_: in_gen,
        } = index;
        let gv = self.inner.get(index)?;

        if in_gen == gv.gen_ || in_gen == GenOptVec::GEN_IGNORE {
            Some(&gv.value)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: GenIndex) -> Option<&mut T> {
        let GenIndex {
            index,
            gen_: in_gen,
        } = index;
        let gv = self.inner.get_mut(index)?;

        if in_gen == gv.gen_ || in_gen == GenOptVec::GEN_IGNORE {
            Some(&mut gv.value)
        } else {
            None
        }
    }
}

impl<T> Default for GenOptVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct GenValue<T> {
    value: T,
    gen_: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenIndex {
    index: usize,
    gen_: u64,
}

impl GenIndex {
    pub const fn ignore_gen(index: usize) -> Self {
        Self {
            index,
            gen_: GenOptVec::GEN_IGNORE,
        }
    }

    pub const fn into_inner(self) -> usize {
        self.index
    }
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct BoxedSlice<T>(Vec<T>);

impl<T> Deref for BoxedSlice<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl<T> DerefMut for BoxedSlice<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut_slice()
    }
}

impl<T: Debug> Debug for BoxedSlice<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.as_slice().fmt(f)
    }
}

impl<Item> FromIterator<Item> for BoxedSlice<Item> {
    fn from_iter<T: IntoIterator<Item = Item>>(iter: T) -> Self {
        let vec = Vec::from_iter(iter);
        Self(vec)
    }
}

impl<T> IntoIterator for BoxedSlice<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a BoxedSlice<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<T, const N: usize> From<[T; N]> for BoxedSlice<T> {
    fn from(value: [T; N]) -> Self {
        let vec = Vec::from(value);
        Self(vec)
    }
}

impl<T> From<Vec<T>> for BoxedSlice<T> {
    fn from(value: Vec<T>) -> Self {
        Self(value)
    }
}
