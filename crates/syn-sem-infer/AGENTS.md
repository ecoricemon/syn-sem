# Instructions

## Role

- Own type inference for upper semantic phases.
- Use `syn-sem-pr` for program shape.
- Use `syn-sem-name` for definition, scope, resolution, and semantic visibility facts.

## Boundaries

- Do not depend on `syn`, `syn-sem-ast`, or raw syntax trees.
- Do not add constant evaluation, monomorphization, backend lowering, or diagnostics ownership here.
- Treat missing inference inputs as possible `syn-sem-pr` V2 requirements before adding workarounds.

## Model

- Start with expression type inference needs.
- Keep inference inputs explicit and query owning crates for facts.
- Preserve Rust type path syntax during lowering; do not treat name lookup as
  final type resolution.
- Keep generic substitution, qualified paths, projections, and trait-based
  associated item lookup solver-friendly.
- Prefer small vertical slices that reveal representation requirements.

## Primary Public Items

- None yet; this crate is a skeleton.
