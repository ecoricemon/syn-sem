# AGENTS.md

## Crate Focus

`syn-sem-name` owns the reusable name-resolution model for extracted
`syn-sem` crates: definitions, scopes, namespaces, bindings, imports, and
lookup results.

## Boundaries

Keep this crate AST-agnostic in production dependencies. Higher layers may
attach AST-specific identities through opaque origins, but this crate should not
depend on `syn`, `syn-sem-forest`, or `syn-sem-ast` for its core model.

Do not add type inference, evaluation, monomorphization, or backend lowering
responsibilities here.

## Model Rules

Rust namespaces must stay separate:

- type namespace
- value namespace
- macro namespace
- lifetime namespace

Generic parameters should be represented as definitions, not recovered through
ad hoc syntax ancestry.

Name resolution should be use-site based, scope-aware, and namespace-aware.

## Primary Public Items

- `NameDb`: name-resolution database containing scopes, definitions, and
  imports.
- `DefId`, `ScopeId`, and `ImportId`: stable ids for database entries.
- `Def`, `DefKind`, `Visibility`, and `Origin`: definition metadata.
- `Scope`, `ScopeKind`, `Bindings`, and `Binding`: lexical scope and
  namespace-partitioned binding model.
- `Namespace`: Rust namespace selector for lookup.
- `Import`, `ImportKind`, and `ImportStatus`: import model and resolution
  state.
- `ResolveResult`: result of resolving a name in a namespace from a use site.
- `Name`: interned name text tied to shared common infrastructure.
