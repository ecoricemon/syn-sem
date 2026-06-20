# Instructions

## Role

- Own the in-progress HIR layer for upper semantic phases.
- Treat `syn-sem-hir` as the package, crate, and directory name for this layer.
- Build source spine, lowered semantic input, and name-fact connection ids.

## Boundaries

- Keep this crate focused on HIR construction and HIR lowering.
- Store ids for source shapes whose facts are owned by `syn-sem-name`.
- Do not duplicate definition, scope, import, resolution, or visibility facts.
- Do not resolve source paths into final semantic paths during source-spine construction.
- Lower sugar into HIR facts or obligations, not resolved call trees.
- Leave trait, method, type, inference, evaluation, monomorphization, and backend
  behavior to later phases.
- Do not treat current name-resolution ownership or phase ordering as permanent.

## Model

- Keep source-shaped spine and lowered/infer-friendly facts distinct.
- Represent declarations, blocks, and expressions explicitly in stable arenas.
- Put generic predicate integration, body/control-flow lowering, and inference
  preprocessing in HIR lowering layers.
- Add unsupported pattern variants on demand instead of filling coverage ahead of consumers.
- Do not make later phases recover source structure from raw AST nodes.
- Keep ownership and lifetimes tied to the shared top-level context.
- Do not construct deep context chains here.

## Compatibility

- Use `Hir` and `HirBuilder` as the public entry points.

## Primary Public Items

- `Hir`, `HirBuilder`.
- `Item`, `Signature`, `Block`, `Field`, `Variant`, `AssocItem`, `Type`.
- `Generics`, `GenericParam`, `TypeParamBound`.
