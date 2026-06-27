# Instructions

## Role

- Own compile-time constant evaluation for upper semantic phases.
- Consume HIR, name facts, and inference facts through focused APIs.
- Expose evaluated constant facts for orchestration by `syn-sem-top`.

## Boundaries

- Do not depend on `syn`, `syn-sem-ast`, or raw syntax trees.
- Do not own type inference, name resolution, monomorphization, backend lowering, or diagnostics.
- Do not make `syn-sem-infer` depend on this crate.

## Model

- Treat evaluation and inference as phases that may iterate to a fixed point.
- Return unknown results when required type or value facts are not available yet.
- Keep value facts explicit and queryable by HIR expression or const argument.

## Primary Public Items

- `EvalDb`, `ConstValue`.
