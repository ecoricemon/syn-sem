# Instructions

## Role

- Own type inference for upper semantic phases.
- Keep solver/logic modules internal to inference.
- Use HIR for source and lowered inference input.
- Query `syn-sem-name` for definition, scope, resolution, and visibility facts.

## Boundaries

- Do not depend on `syn`, `syn-sem-ast`, or raw syntax trees.
- Do not expose logic as a separate upper phase.
- Do not add constant evaluation, monomorphization, backend lowering, or diagnostics ownership here.
- Treat missing inference inputs as possible HIR requirements before adding workarounds.

## Model

- Start with expression type inference needs.
- Keep inference inputs explicit and query owning crates for facts.
- Prefer `*_hir_type` query names for HIR-linked inference facts.
- Preserve Rust type path syntax during lowering; name lookup is not final type resolution.
- Keep generic substitution, qualified paths, projections, and trait-based associated
  lookup solver-friendly.
- Feed logic-backed solving HIR, `syn-sem-name`, and inference facts, not raw syntax.
- Prefer small vertical slices that reveal HIR requirements.

## Primary Public Items

- `InferDb`, `Type`, `TypeId`, `PrimitiveType`.
- `PathType`, `QSelf`, `PathTypeResolution`, `ProjectionType`.
- `ProjectionNormalizationResult`.
