# AGENTS.md

## Crate Focus

`syn-sem-macros` provides procedural macros used by extracted `syn-sem` crates.

## Boundaries

Keep this crate limited to small compile-time helpers. Do not put runtime
semantic-analysis logic, shared data models, or phase-specific behavior here.

Macros should generate straightforward code and keep their expansion behavior
easy to inspect.

## Model Rules

Prefer explicit compile-time checks over generated runtime behavior when adding
macros for crate invariants.

## Primary Public Items

- `CheckDropless`: derive macro that emits a compile-time assertion that the
  target type does not need drop.
