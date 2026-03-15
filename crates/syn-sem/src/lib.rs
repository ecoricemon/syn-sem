#![doc = include_str!("../README.md")]

pub mod ds;
pub(crate) mod etc;
pub(crate) mod semantic;
pub(crate) mod syntax;

// TODO
// * Remove tree::item::Local. It should not be in the path tree. Prepare something another for
//   holding type info of local variables.
//   - Why?
//     We cannot put local variables in the path tree if the function is restricted by some bounds.
//     We should use logic for that instead of the path tree.
//   - Take account of other items in function blocks.
//     A function block can contain types, consts, and others like module. They may need to be in
//     the path tree.
// * `PathTree::search_type` -> it is supposed to find an item in the type namespace. Is it
//   doing like so? Also, rename the function to clarify it searches in the type namespace.
// * `PathTree::search_item` -> it is supposed to find an item in the value namespace. Is it
//   doing like so? Also, rename the function to clarify it searches in the value namespace.

// === Re-exports ===

pub use etc::util::{cargo_crate_name, get_crate_name, set_crate_name, PathSegments};
pub use logic_eval::{Intern, Name, Term};
pub use semantic::{
    analyze::{Analyzer, Semantics},
    entry::{AnalysisSession, Analyzed, Config, ConfigLoad, GlobalCx},
    eval::Evaluated,
    helper,
    logic::{term, ImplLogic, Logic},
    tree::{
        pub_filter as filter, ArrayLen, Brief, NodeIndex, PathId, PubPathTree as PathTree, Type,
        TypeArray, TypeId, TypeMut, TypePath, TypeRef, TypeScalar, TypeTuple, UniqueTypes,
    },
};
pub use syntax::common::{AttributeHelper, IdentifySyn, SynId};

pub mod item {
    pub use super::semantic::tree::{
        Block, Const, Field, Fn, Local, Mod, Param, PubItem as Item, Struct, Trait, TypeAlias, Use,
    };
}
pub mod value {
    pub use super::semantic::eval::{ConstGeneric, Enum, Field, Fn, Scalar, Value};
}
pub mod locator {
    pub use syn_locator::{clear, enable_thread_local};
}

use std::{
    collections::{HashMap, HashSet},
    error::Error as StdError,
    result::Result as StdResult,
    sync::{Mutex, MutexGuard},
};

pub type Result<T> = StdResult<T, Error>;
pub type Error = Box<dyn StdError + Send + Sync>;
pub type TriResult<T, Se> = StdResult<T, TriError<Se>>;
pub(crate) type SharedLock<T> = Mutex<T>;
pub(crate) type SharedLockGuard<'a, T> = MutexGuard<'a, T>;

#[derive(Debug)]
pub enum TriError<Se> {
    Soft(Se),
    Hard(Box<dyn StdError + Send + Sync>),
}

pub trait TriResultHelper<T, S> {
    fn on_soft_err<F: FnOnce(S) -> Result<T>>(self, f: F) -> Result<T>;

    fn map_soft_err<F: FnOnce(S) -> U, U>(self, f: F) -> TriResult<T, U>;

    /// If soft error, make it hard error.
    fn elevate_err(self) -> Result<T>;
}

impl<T, S> TriResultHelper<T, S> for TriResult<T, S> {
    fn on_soft_err<F: FnOnce(S) -> Result<T>>(self, f: F) -> Result<T> {
        match self {
            Ok(t) => Ok(t),
            Err(TriError::Soft(s)) => f(s),
            Err(TriError::Hard(e)) => Err(e),
        }
    }

    fn map_soft_err<F: FnOnce(S) -> U, U>(self, f: F) -> TriResult<T, U> {
        match self {
            Ok(t) => Ok(t),
            Err(TriError::Soft(s)) => Err(TriError::Soft(f(s))),
            Err(TriError::Hard(e)) => Err(TriError::Hard(e)),
        }
    }

    fn elevate_err(self) -> Result<T> {
        match self {
            Ok(t) => Ok(t),
            Err(TriError::Soft(_)) => Err("elavated soft error".into()),
            Err(TriError::Hard(e)) => Err(e),
        }
    }
}

impl<S> From<Error> for TriError<S> {
    fn from(value: Error) -> Self {
        Self::Hard(value)
    }
}

#[macro_export]
macro_rules! err {
    (soft, $expr:expr) => {
        Err( $crate::error!(soft, $expr) )
    };
    (hard, $($t:tt)*) => {
        Err( $crate::error!(hard, $($t)*) )
    };
    ($($t:tt)*) => {
        Err( $crate::error!($($t)*) )
    };
}

#[macro_export]
macro_rules! error {
    (soft, $expr:expr) => {
        $crate::TriError::Soft($expr)
    };
    (hard, $($t:tt)*) => {
        $crate::TriError::Hard( format!($($t)*).into() )
    };
    ($($t:tt)*) => {
        $crate::Error::from( format!($($t)*) )
    };
}

#[macro_export]
macro_rules! log {
    (E, $($t:tt)*) => {{
        if log::log_enabled!(log::Level::Error) {
            log::error!($($t)*);
        }
    }};
    (W, $($t:tt)*) => {{
        if log::log_enabled!(log::Level::Warn) {
            log::warn!($($t)*);
        }
    }};
    (I, $($t:tt)*) => {{
        if log::log_enabled!(log::Level::Info) {
            log::info!($($t)*);
        }
    }};
    (D, $($t:tt)*) => {{
        if log::log_enabled!(log::Level::Debug) {
            log::debug!($($t)*);
        }
    }};
    (T, $($t:tt)*) => {{
        if log::log_enabled!(log::Level::Trace) {
            log::trace!($($t)*);
        }
    }};
}

#[macro_export]
macro_rules! print {
    ($($t:tt)*) => {
        println!(
            "@ {}\n┗ {}",
            $crate::cur_path!(),
            format!($($t)*)
        )
    };
}

#[macro_export]
macro_rules! cur_path {
    () => {{
        struct S;
        let s = std::any::type_name::<S>();
        &s[..s.len() - 3]
    }};
}

#[macro_export]
macro_rules! pnode {
    ($ptree:expr, $($path:tt)*) => {{
        let key = format!($($path)*);
        let Some(node) = $ptree.search($crate::PathTree::ROOT, key.as_str()) else {
            panic!("failed to find the given path: `{}`", format!($($path)*));
        };
        node
    }};
}

#[macro_export]
macro_rules! pid {
    ($ptree:expr, $($path:tt)*) => {{
        let node = $crate::pnode!($ptree, $($path)*);
        let (ii, _) = $ptree.node(node)
            .iter()
            .next()
            .unwrap();
        node.to_path_id(ii)
    }};
}

#[macro_export]
macro_rules! pitem {
    ($ptree:expr, $($path:tt)*) => {{
        let pid = $crate::pid!($ptree, $($path)*);
        &$ptree.item(pid)
    }};
}

// === Hash map and set used within this crate ===

pub type Map<K, V> = HashMap<K, V, fxhash::FxBuildHasher>;
pub type Set<T> = HashSet<T, fxhash::FxBuildHasher>;

// === Enum of A or B ===

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Which2<A, B> {
    A(A),
    B(B),
}

// === Enum of A, B, or C ===

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Which3<A, B, C> {
    A(A),
    B(B),
    C(C),
}

impl<A, B, C, VA> FromIterator<Which3<A, B, C>> for Which3<VA, B, C>
where
    VA: FromIterator<A>,
{
    fn from_iter<T: IntoIterator<Item = Which3<A, B, C>>>(iter: T) -> Self {
        let mut filtered = None;
        let va = iter
            .into_iter()
            .map(|which| match which {
                Which3::A(a) => Some(a),
                Which3::B(b) => {
                    filtered = Some(Which3::B(b));
                    None
                }
                Which3::C(c) => {
                    filtered = Some(Which3::C(c));
                    None
                }
            })
            .take_while(|opt| opt.is_some())
            .map(|opt| opt.unwrap())
            .collect::<VA>();

        if let Some(filtered) = filtered {
            return filtered;
        }

        Which3::A(va)
    }
}

// === Option ===

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TriOption<T, U> {
    Some(T),
    NotYet(U),
    None,
}

impl<T, U> TryFrom<TriOption<T, U>> for Option<T> {
    type Error = Error;

    fn try_from(value: TriOption<T, U>) -> Result<Self> {
        match value {
            TriOption::Some(t) => Ok(Some(t)),
            TriOption::NotYet(_) => err!("`TriOption::NotYet` cannot become `Option`"),
            TriOption::None => Ok(None),
        }
    }
}

// === GetOwned ===

pub trait GetOwned<Id> {
    type Owned;
    fn get_owned(&self, id: Id) -> Self::Owned;
}

// === Type alias ===

pub(crate) type NameIn<'a> = logic_eval::Name<any_intern::Interned<'a, str>>;
pub(crate) type TermIn<'a> = logic_eval::Term<NameIn<'a>>;
pub(crate) type ExprIn<'a> = logic_eval::Expr<NameIn<'a>>;
pub(crate) type ClauseIn<'a> = logic_eval::Clause<NameIn<'a>>;
pub(crate) type PredicateIn<'a> = logic_eval::Predicate<NameIn<'a>>;
