use std::fmt;

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub(crate) usize);

        impl $name {
            /// Creates an id from a raw index.
            ///
            /// This is intended for tests and serialization boundaries. Normal code should obtain
            /// ids from [`NameDb`](crate::NameDb).
            pub const fn new(index: usize) -> Self {
                Self(index)
            }

            /// Returns the raw index represented by this id.
            pub const fn index(self) -> usize {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type! {
    /// Stable identity for a named definition.
    DefId
}

id_type! {
    /// Stable identity for a lexical or item scope.
    ScopeId
}

id_type! {
    /// Stable identity for an import declaration.
    ImportId
}
