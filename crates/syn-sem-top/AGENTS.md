# Instructions

## Goal

- Provide the temporary top-level semantic-analysis API during migration.
- Grow toward generated semantic IR, additional semantic operations, and helper queries.

## Role

- Wire extracted crates together.
- Own top-level orchestration for parsing, preparing name-collection inputs,
  invoking name collection, and HIR construction.

## Boundaries

- Keep backend-specific lowering separable.
- Do not require Naga as a core dependency for generated IR unless explicitly chosen.
- Avoid rebuilding old `syn-sem` internals here.

## Model

- Prefer simple top-level orchestration.
- Prefer incremental migration.
- Keep `Semantics` as the aggregate that exposes `NameDb` and the current HIR
  container.

## Primary Public Items

- `TopCx`: top-level root context for shared infrastructure and phase contexts.
- `Semantics`: durable semantic-analysis output for IR, diagnostics, and queries.
