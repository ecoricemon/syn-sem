# AGENTS.md

## Goal

`syn-sem-top` should provide the temporary top-level API for semantic analysis
during migration.

The analysis output should eventually include generated semantic IR, support
additional semantic operations such as monomorphization, and provide helper APIs
that make client inspection easy.

## Crate Focus

`syn-sem-top` wires extracted crates together and provides the temporary
top-level semantic-analysis surface during migration.

## Boundaries

Keep backend-specific lowering separable from this crate. Generated IR should
not require Naga as a core dependency unless explicitly chosen.

Avoid rebuilding old `syn-sem` internals inside this crate.

## Model Rules

Prefer simple top-level orchestration and incremental migration.

## Primary Public Items

- `TopCx`: top-level root context that owns shared infrastructure and phase
  contexts.
- `Semantics`: durable semantic-analysis output for generated IR, diagnostics,
  and helper queries.
- `IrProgram`: generated semantic IR, independent from backend representations.
