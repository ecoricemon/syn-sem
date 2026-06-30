# Instructions

## Goal

- Provide the temporary top-level semantic-analysis API during migration.
- Grow toward semantic IR, operations, helper queries, and phase orchestration.

## Role

- Wire extracted crates together.
- Own parsing, name collection, HIR construction, type inference, and constant
  evaluation orchestration.

## Boundaries

- Keep backend-specific lowering separable.
- Do not require Naga as a core dependency for generated IR unless explicitly chosen.
- Avoid rebuilding old `syn-sem` internals here.

## Model

- Prefer simple orchestration and incremental migration.
- Keep `Semantics` as the aggregate exposing `NameDb`, HIR, `InferDb`, and
  `EvalDb`.

## Entry Points

- Start from `TopCx`; return and query `Semantics`.
