pub mod common;
pub mod context;
pub mod data;
pub mod expr;
pub mod file;
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
pub use item::*;
pub use lit::*;
pub use pat::*;
pub use path::*;
pub use restriction::*;
pub use stmt::*;
pub use ty::*;

#[cfg(test)]
pub(crate) mod test_util {
    use crate::{FromSyn, SyntaxContext};
    use any_intern::DroplessInterner;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU32, Ordering::Relaxed},
    };
    use syn::parse::Parse;
    use syn_locator::LocateEntry;

    pub(crate) fn parse<'scx, T: Parse + LocateEntry, U: FromSyn<'scx, T>>(
        scx: &'scx SyntaxContext,
        text: &str,
    ) -> U {
        static ID: AtomicU32 = AtomicU32::new(0);
        let id = ID.fetch_add(1, Relaxed);
        let file_path = PathBuf::from(id.to_string());

        scx.insert_virtual_source::<T>(file_path.clone(), text.into());
        let source = scx.get_source(&file_path);
        let syn: &T = source.syn.downcast_ref().unwrap();

        U::from_syn(scx, syn)
    }

    pub(crate) fn create_context() -> SyntaxContext {
        let interner = DroplessInterner::new();
        SyntaxContext::new(interner)
    }
}
