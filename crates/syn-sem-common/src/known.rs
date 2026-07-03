//! Well-known library source catalog.
//!
//! This module is intentionally a source catalog only. Phase crates decide whether and how to parse
//! these sources for a given analysis run.

/// One well-known library source entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownLibrary {
    /// Stable library name.
    pub name: &'static str,
    /// Virtual source path for this library.
    pub path: &'static str,
    /// Rust source used for the current lightweight known-library model.
    pub source: &'static str,
}

/// Selected well-known libraries for a caller-controlled analysis setup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownLibraries {
    /// Whether to include `core`.
    pub core: bool,
    /// Whether to include `std`.
    pub std: bool,
}

impl KnownLibraries {
    /// Returns selected library sources in dependency order.
    pub fn sources(self) -> Vec<KnownLibrary> {
        let mut sources = Vec::new();
        if self.core || self.std {
            sources.push(CORE);
        }
        if self.std {
            sources.push(STD);
        }
        sources
    }
}

/// Lightweight `core` source for early semantic inference.
pub const CORE: KnownLibrary = KnownLibrary {
    name: "core",
    path: "__syn_sem_known_core.rs",
    source: CORE_SOURCE,
};

/// Lightweight `std` source for early semantic inference.
pub const STD: KnownLibrary = KnownLibrary {
    name: "std",
    path: "__syn_sem_known_std.rs",
    source: STD_SOURCE,
};

/// Lightweight `core` model.
pub const CORE_SOURCE: &str = r#"
pub mod core {
    pub mod ops {
        pub trait Add<Rhs> {
            type Output;
        }

        impl Add<usize> for usize {
            type Output = usize;
        }
    }
}
"#;

/// Lightweight `std` model.
pub const STD_SOURCE: &str = r#"
pub mod std {
    pub use core::ops;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_no_known_libraries() {
        // Proves callers can opt out of loading slow well-known library inputs.
        assert_eq!(
            KnownLibraries {
                core: false,
                std: false,
            }
            .sources(),
            &[]
        );
    }

    #[test]
    fn selects_core_only() {
        // Proves `core` can be loaded without `std`.
        assert_eq!(
            KnownLibraries {
                core: true,
                std: false,
            }
            .sources(),
            &[CORE]
        );
    }

    #[test]
    fn selects_std_after_core() {
        // Proves `std` pulls in `core` first because it re-exports core items.
        assert_eq!(
            KnownLibraries {
                core: false,
                std: true,
            }
            .sources(),
            &[CORE, STD]
        );
    }
}
