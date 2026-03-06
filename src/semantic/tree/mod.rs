mod format;
mod inner;
mod item;
mod node;
mod private;
mod public;
mod ty;

// === Re-exports ===

pub use format::Brief;
pub use item::{
    Block, Const, Enum, Field, Fn, Local, Mod, PubItem, Struct, Trait, TypeAlias, Use, Variant,
};
pub use node::NodeIndex;
pub use public::{pub_filter, PubPathTree};
pub use ty::{
    ArrayLen, Param, Type, TypeArray, TypeId, TypeMut, TypePath, TypeRef, TypeScalar, TypeTuple,
    UniqueTypes,
};

#[cfg(test)]
pub use ty::{OwnedParam, OwnedType};

pub(crate) use inner::{
    filter, PathTree, SearchTypeNotFound, SearchTypeNotReady, SearchTypeOk, SearchTypeResult,
};
pub(crate) use item::{
    EffectiveItemKind, ItemTrait, PathVis, PrivItem, RawConst, RawEnum, RawField, RawFn, RawLocal,
    RawMod, RawStruct, RawTrait, RawTypeAlias, RawUse, RawVariant,
};
pub(crate) use private::PrivPathTree;
pub(crate) use public::AsPrivPathTree;

use crate::{syntax::common::SynId, Map};
use std::fmt;

// === PathId ===

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathId {
    pub(crate) ni: node::NodeIndex,
    pub(crate) ii: item::ItemIndex,
}

impl PathId {
    pub const fn node_index(&self) -> node::NodeIndex {
        self.ni
    }
}

impl fmt::Display for PathId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({},{})", self.ni, self.ii)
    }
}

impl fmt::Debug for PathId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({},{})", self.ni, self.ii)
    }
}

// === SynToPath ===

#[derive(Debug, Default)]
pub struct SynToPath(Map<SynId, PathId>);

impl SynToPath {
    pub(super) fn new() -> Self {
        Self(Map::default())
    }

    pub(super) fn add_syn_to_path(&mut self, sid: SynId, pid: PathId) {
        if let Some(old_pid) = self.0.insert(sid, pid) {
            if old_pid != pid {
                panic!(
                    "syn-path id conflicts: syn: `{}`, old_pid: {old_pid}, pid: {pid}",
                    sid.content(),
                );
            }
        }
    }

    pub(super) fn get_path_id(&self, sid: SynId) -> Option<PathId> {
        self.0.get(&sid).cloned()
    }
}
