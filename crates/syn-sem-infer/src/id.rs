//! Stable ids used by inference-owned arenas.

syn_sem_macros::define_id! {
    {
        /// Stable identity for one inference type.
        pub(crate) TypeId
    }
    {
        /// Stable identity for one structural inference-type class.
        pub(crate) TypeClassId
    }
}
