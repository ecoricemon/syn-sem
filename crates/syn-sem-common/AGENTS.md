# Instructions

## Role

- Own shared infrastructure for extracted `syn-sem` crates: context state,
  string interning, source identifiers, abstract source files, arenas, and
  AST-node identity helpers.

## Boundaries

- Keep this crate independent from syntax, AST, name-resolution, semantic,
  inference, evaluation, and backend crates.
- Do not add domain-specific semantic concepts here.
- Add only genuinely shared infrastructure below phase crates.

## Model

- Do not expose or store `RawInterned<str>` in crate APIs.
- Keep interned paths and text tied to their producing `CommonCx` or interner.

## Entry Points

- Start from `CommonCx` for shared context state and interned source data.
