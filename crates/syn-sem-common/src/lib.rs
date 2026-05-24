mod context;

pub use context::*;

use any_intern::Interned;
use std::{collections::HashMap, error::Error as StdError};

/// Error type shared by internal `syn-sem` crates.
pub type Error = Box<dyn StdError + Send + Sync>;

/// Result type shared by internal `syn-sem` crates.
pub type Result<T> = std::result::Result<T, Error>;

/// Hash map type used by internal `syn-sem` crates.
pub type Map<K, V> = HashMap<K, V, fxhash::FxBuildHasher>;

/// String interned in [`CommonCx`].
///
/// The `'ccx` lifetime is tied to the [`CommonCx`] / [`StringInterner`] that produced it.
pub type InternedStr<'ccx> = Interned<'ccx, str>;

/// Interned absolute or virtual source file path.
pub type FilePath<'ccx> = InternedStr<'ccx>;

/// Interned source text.
pub type SourceCode<'ccx> = InternedStr<'ccx>;

/// Interned known library name, such as `core` or `std`.
pub type LibraryName<'ccx> = InternedStr<'ccx>;
