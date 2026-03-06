use crate::{
    semantic::{eval, infer},
    SharedLock, SharedLockGuard,
};
use any_intern::{DroplessInterner, Interned};
use logic_eval::Intern;
use logic_eval_util::symbol::SymbolTable;
use std::{
    fmt::{self, Display},
    result::Result as StdResult,
};

// Self-referential type
#[derive(Debug)]
pub struct GlobalCx<'gcx> {
    pub interner: DroplessInterner,
    config: SharedLock<Config>,
    lasting_symbols: SharedLock<LastingSymbols<'gcx>>,
}

impl<'gcx> GlobalCx<'gcx> {
    pub fn configure(&self, config: Config) {
        *self.config.lock().unwrap() = config;
    }

    pub fn get_config(&self) -> SharedLockGuard<'_, Config> {
        self.config.lock().unwrap()
    }

    pub(crate) fn lasting_symbols(&'gcx self) -> SharedLockGuard<'gcx, LastingSymbols<'gcx>> {
        self.lasting_symbols.lock().unwrap()
    }
}

impl Default for GlobalCx<'_> {
    fn default() -> Self {
        Self {
            interner: DroplessInterner::new(),
            config: SharedLock::new(Config::default()),
            lasting_symbols: SharedLock::new(LastingSymbols::default()),
        }
    }
}

impl<'gcx> Intern for GlobalCx<'gcx> {
    type InternedStr<'a>
        = any_intern::Interned<'a, str>
    where
        Self: 'a;

    fn intern_formatted_str<T: Display + ?Sized>(
        &self,
        value: &T,
        upper_size: usize,
    ) -> StdResult<Self::InternedStr<'_>, fmt::Error> {
        self.interner.intern_formatted_str(value, upper_size)
    }

    fn intern_str(&self, text: &str) -> Self::InternedStr<'_> {
        self.interner.intern(text)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub load: ConfigLoad,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            load: ConfigLoad::all(),
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ConfigLoad: u32 {
        const CORE = 1 << 0;
        const STD = 1 << 1;
    }
}

/// Symbols may live across task boundary.
///
/// Therefore, symbols in this type can be added within one task and consumed within another task.
#[derive(Debug, Default)]
pub(crate) struct LastingSymbols<'gcx> {
    pub(crate) infer_type_symbols: SymbolTable<Interned<'gcx, str>, infer::Type<'gcx>>,
    pub(crate) eval_value_symbols: SymbolTable<Interned<'gcx, str>, eval::Value<'gcx>>,
}
