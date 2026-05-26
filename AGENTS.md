# AGENTS.md

## Project Direction

We are refactoring `crates/syn-sem` by extracting focused sub-crates.

`syn-sem` remains the facade/orchestrator while internals are migrated.

We are implementing `syn-sem-top` instead of modifying `syn-sem` in the middle
of migration.

Each extracted crate may have its own `AGENTS.md` with crate-local role,
boundary, model, and public-item guidance. Prefer the nearest crate-local file
for details specific to that crate.

## Instruction Maintenance

Keep `AGENTS.md` files stable and lightweight. Do not treat them as detailed
design records.

When API direction changes, prefer following the user's latest explicit
instruction in the active task rather than updating these files for every design
choice.

Only update an `AGENTS.md` file when a decision is expected to remain durable
across many future tasks.

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

## Naming Conventions

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

## Cross-Crate Boundaries

Keep extracted crates focused.

Respect each crate's local boundary rules before adding dependencies or moving
public items.

New reusable infrastructure should usually be extracted before being wired
deeply into `syn-sem`.

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

Prefer incremental refactors. Each step should compile and pass focused tests
before moving to the next step.

Avoid broad rewrites unless they directly support the current extraction step.
