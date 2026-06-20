# Instructions

## Goal

- Provide the temporary top-level semantic-analysis API during migration.
- Grow toward semantic IR, operations, and helper queries.

## Role

- Wire extracted crates together.
- Own parsing, name-collection input preparation, name collection, and HIR construction.

## Boundaries

- Keep backend-specific lowering separable.
- Do not require Naga as a core dependency for generated IR unless explicitly chosen.
- Avoid rebuilding old `syn-sem` internals here.

## Model

- Prefer simple orchestration and incremental migration.
- Keep `Semantics` as the aggregate exposing `NameDb` and HIR.

## Primary Public Items

- `TopCx`, `Semantics`.
