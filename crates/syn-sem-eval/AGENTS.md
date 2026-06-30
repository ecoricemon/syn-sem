# Instructions

## Role

- Own compile-time constant evaluation for upper semantic phases.
- Consume HIR, name facts, and inference facts through focused APIs.
- Expose evaluated constant facts for fixed-point orchestration by `syn-sem-top`.

## Boundaries

- Do not depend on `syn`, `syn-sem-ast`, or raw syntax trees.
- Do not own type inference, name resolution, monomorphization, backend lowering, or diagnostics.
- Do not make `syn-sem-infer` depend on this crate.

## Model

- Treat evaluation as a phase that runs with inference until `syn-sem-top`
  reaches a fixed point.
- Return unknown results when required type or value facts are not available yet.
- Keep value facts explicit and queryable by HIR expression or const argument.

## Entry Points

- Start from `EvalDb` for collected constant values.
