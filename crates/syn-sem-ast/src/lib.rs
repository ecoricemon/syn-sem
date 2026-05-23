pub mod common;
pub mod context;
pub mod data;
pub mod expr;
pub mod file;
pub mod generics;
pub mod item;
pub mod lit;
pub mod pat;
pub mod path;
pub mod restriction;
pub mod stmt;
pub mod ty;

pub use common::*;
pub use context::*;
pub use data::*;
pub use expr::*;
pub use file::*;
pub use generics::*;
pub use item::*;
pub use lit::*;
pub use pat::*;
pub use path::*;
pub use restriction::*;
pub use stmt::*;
pub use ty::*;

//pub(crate) type Map<K, V> = fxhash::FxHashMap<K, V>;
pub(crate) type AppendOnlyMap<K, V> = elsa::FrozenMap<K, V, fxhash::FxBuildHasher>;

#[cfg(test)]
pub(crate) mod test_util {
    use crate::{FromSyn, InputDesc, SyntaxCx};
    use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
    use syn::parse::Parse;
    use syn_locator::LocateEntry;

    pub(crate) fn parse<'scx, T: Parse + LocateEntry + 'static, U: FromSyn<'scx, T>>(
        scx: &'scx SyntaxCx,
        text: &str,
    ) -> U {
        // Creates a unique file path.
        static ID: AtomicU32 = AtomicU32::new(0);
        let id = ID.fetch_add(1, Relaxed);
        let file_path = id.to_string().into_boxed_str();

        // Parses `T` and generates `U`.
        scx.parse_virtual_syntax::<T>(file_path.clone(), text.into());
        let source = scx.get_source(&file_path).unwrap();
        let syn = source.syntax::<T>().unwrap();
        U::from_syn(
            scx,
            InputDesc {
                file_path: &file_path,
                input: syn,
            },
        )
    }
}
