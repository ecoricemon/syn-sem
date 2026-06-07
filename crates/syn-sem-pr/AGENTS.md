# AGENTS.md

## Crate Focus

`syn-sem-pr` owns the Rust source program representation for `syn-sem`.
`pr` means Program Representation. This crate should describe items,
signatures, fields, bodies, and type occurrences in a form other phases can
consume.

## Boundaries

Keep this crate focused on the program representation layer during the current
extraction step.

Current integrations may refer to definitions, scopes, and imports through
`syn-sem-name` ids and data, but name-resolution ownership is not fixed
architecture.

Type inference, constant evaluation, monomorphization, and backend-specific
lowering responsibilities are not fixed as before or after this crate. Keep the
current implementation incremental and avoid baking in permanent phase ordering.

## Representation Rules

Prefer Rust-shaped semantic data over backend-oriented or fully lowered
representations.

Represent source-level declarations and bodies explicitly enough that later
phases do not need to recover structure from raw AST nodes.

Leave room for future block/body desugaring IR. That IR may be added here or
split into a sibling crate when the design becomes concrete.

Keep ownership and lifetimes tied to the shared top-level context; avoid making
this crate construct deep context chains.

## Primary Public Items

- `ProgramRepr`: program representation produced from AST and name-resolution data.
- `Item`, `Signature`, `Body`, `Field`, `Variant`, `AssocItem`, and `Type`:
  current source-level program representation components.
