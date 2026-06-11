# Instructions

## Role

- Own type inference for upper semantic phases.
- Complete inference inside this crate; use solver/logic modules as internal
  implementation pieces when trait, impl, bound, or projection solving is needed.
- Use `syn-sem-pr` for program shape.
- Use `syn-sem-name` for definition, scope, resolution, and semantic visibility facts.

## Boundaries

- Do not depend on `syn`, `syn-sem-ast`, or raw syntax trees.
- Do not expose logic as a separate upper phase over inference; keep it behind
  inference-owned APIs.
- Do not add constant evaluation, monomorphization, backend lowering, or diagnostics ownership here.
- Treat missing inference inputs as possible `syn-sem-pr` V2 requirements before adding workarounds.

## Model

- Start with expression type inference needs.
- Keep inference inputs explicit and query owning crates for facts.
- Preserve Rust type path syntax during lowering; do not treat name lookup as
  final type resolution.
- Keep generic substitution, qualified paths, projections, and trait-based
  associated item lookup solver-friendly.
- Keep logic-backed solving modular internally, but feed it `syn-sem-pr`,
  `syn-sem-name`, and inference facts rather than raw syntax.
- Prefer small vertical slices that reveal representation requirements.

## Primary Public Items

- `InferDb`: entry point for lowering represented type occurrences.
- `Type`, `TypeId`: inference type shapes and stable ids.
- `PathType`, `QSelf`, `PathTypeResolution`: source-shaped path types plus
  current solver-friendly resolution classification.
- `ProjectionObligation`: associated type projections that later solver/logic
  work must prove or normalize.
- `TraitBoundFact`: generic trait bounds lowered as solver/logic input facts.
- `ProjectionCandidate`: candidate trait selections derived from projection
  obligations and known trait bounds.
- `ProjectionMatch`: associated type projections matched against concrete trait members.
- `PrimitiveType`: primitive Rust type classification stored as `Type::Primitive`.
