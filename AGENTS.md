# AGENTS.md

## Project Direction

We are refactoring `crates/syn-sem` by extracting focused sub-crates.

Current extracted crates:

- `syn-sem-common`: shared infrastructure, including `CommonCx`, string
interning, interned file paths/source, and abstract files.
- `syn-sem-forest`: raw `syn` syntax forest, source locations, parent lookup,
and syntax identity.
- `syn-sem-ast`: semantic AST wrapper over `syn`; semantic code should gradually
stop depending directly on raw `syn`.
- `syn-sem-name`: name-resolution model, including definitions, scopes,
namespaces, bindings, and imports.
- `syn-sem-top`: temporary top-level crate.

`syn-sem` remains the facade/orchestrator while internals are migrated.

We are implementing `syn-sem-top` instead of modifying `syn-sem` in the middle
of migration.

## Context Model

Use a flattened context hierarchy.

`CommonCx` is the bottom/shared infrastructure context. It owns common
facilities such as string interning and shared file/source identifiers, but it
is not the only context we expect to have.

Future phase contexts should be siblings at the top-level session/root layer,
not deeply nested inside one another. For example, name, AST, semantic,
inference, and evaluation contexts may all exist at the same session level and
borrow the contexts they need.

Self-referential ownership and lifetime wiring should be handled only at the
top-level session/root layer. Lower-level contexts should not try to own parent
contexts or construct deep context chains. `TopCx` is expected to be the
self-referential root like the old `GlobalCx`.

Context binding names should use conventional names, with no exceptions:

- `CommonCx` binds as `ccx`
- `SyntaxCx` binds as `scx`
- `TopCx` binds as `tcx`

Context lifetime names depend on where they appear:

- `TopCx` is the self-referential top-level exception; write it as
  `TopCx<'tcx>`.
- Other non-top-level context types use `'cx`; for example, write
  `FooCx<'cx>`.
- Other places use conventional lifetime names that match the referent; for
  example, write `&'ccx CommonCx` or `&'tcx TopCx`.

## Interning Rules

Use lifetime-bearing interned strings through `syn-sem-common` aliases:

```rust
InternedStr<'cx>
FilePath<'cx>
SourceCode<'cx>
```

Do not use `RawInterned<str>`.

Do not store shared file paths as `Box<str>`.

## Crate Boundaries

Keep extracted crates focused.

- `syn-sem-common` should not depend on AST, forest, name-resolution, or
semantic crates.
- `syn-sem-name` should stay AST-agnostic in production dependencies.
- AST-based name-resolution tests may use `syn-sem-ast` as a dev-dependency.
- New reusable infrastructure should usually be extracted before being wired
deeply into `syn-sem`.

## Name Resolution Direction

Do not stretch `PathTree` further as the long-term name resolver.

The desired model is:

```text
NameDb
  DefId
  ScopeId
  Namespace
  Binding
  Import
```

Resolution should be use-site based, scope-aware, and namespace-aware.

Rust namespaces must stay separate:

- type namespace
- value namespace
- macro namespace
- lifetime namespace

Generic parameters should be represented as definitions, not found by ad hoc
ancestor syntax walks.

`PathTree` may remain temporarily during migration, but new name-resolution work
should move toward `syn-sem-name`.

## Testing Expectations

For each extracted crate, prefer focused checks:

```sh
cargo check -p <crate>
cargo test -p <crate>
cargo rustdoc -p <crate> -- -D missing_docs
```

Run `cargo fmt` before finalizing changes.
When adding public items to extracted crates, add rustdoc comments.

## Style Notes

Keep `lib.rs` clean.

Use conventional context variable names:
e.g., `TopCx` should be bound as `tcx`, `SyntaxCx` as `scx`, `CommonCx` as
`ccx`.
If the conventional name is already taken, suggest another concise name.

Prefer incremental refactors. Each step should compile and pass focused tests
before moving to the next step.

Avoid broad rewrites unless they directly support the current extraction step.
