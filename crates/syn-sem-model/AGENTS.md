# AGENTS.md

## Crate Focus

`syn-sem-model` owns the Rust-shaped semantic program model built after AST
collection and name resolution. It should describe items, signatures, fields,
bodies, and type references in a form later semantic phases can consume.

## Boundaries

Keep this crate focused on the semantic model layer between `syn-sem-ast` /
`syn-sem-name` and later analysis phases.

Do not add name-resolution ownership here. Refer to definitions, scopes, and
imports through `syn-sem-name` ids and data.

Do not add type inference, constant evaluation, monomorphization, or
backend-specific lowering responsibilities here. Those belong in later sibling
phase crates.

## Model Rules

Prefer Rust-shaped semantic data over backend-oriented or fully lowered
representations.

Model source-level declarations and bodies explicitly enough that later phases
do not need to recover structure from raw AST nodes.

Use `DefId` and related ids from `syn-sem-name` to connect model data to name
resolution results.

Keep ownership and lifetimes tied to the shared top-level context; avoid making
this crate construct deep context chains.

## Primary Public Items

- `Model`: semantic program model produced from AST and name-resolution data.
- Item, signature, body, field, and type-reference models will be added here as
  extraction progresses.
