/// Rust namespace used for name lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Namespace {
    /// Type namespace: modules, types, traits, type aliases, and type parameters.
    Type,

    /// Value namespace: functions, constants, statics, locals, and const parameters.
    Value,

    /// Macro namespace.
    Macro,

    /// Lifetime namespace.
    Lifetime,
}
