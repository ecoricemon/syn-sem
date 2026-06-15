# Instructions

## Role

- Own the in-progress HIR layer for upper semantic phases.
- Treat `syn-sem-hir` as the package, crate, and directory name for this layer.
- Build HIR from AST inputs and `syn-sem-name` facts.
- Represent source spine, lowered semantic input, and name-fact connection ids
  for other phases.

## Boundaries

- Keep this crate focused on HIR construction and HIR lowering.
- Store ids that connect represented source shape to facts owned by `syn-sem-name`.
- Do not duplicate resolved definition, scope, import, or semantic visibility facts here.
- Do not resolve source paths into absolute or final semantic paths during source
  spine construction.
- Do not treat current name-resolution ownership as fixed architecture.
- Do not bake in permanent phase ordering around inference, evaluation,
  monomorphization, or backend lowering.

## Model

- Keep source-shaped spine and lowered/infer-friendly facts distinct inside HIR.
- Represent source declarations, source blocks, and source expressions explicitly
  in stable arenas.
- Put generic predicate integration, body/control-flow lowering, and inference
  preprocessing in HIR lowering layers.
- Add unsupported pattern variants on demand instead of filling coverage ahead of consumers.
- Do not make later phases recover source structure from raw AST nodes.
- Keep ownership and lifetimes tied to the shared top-level context.
- Do not construct deep context chains here.
- Do not add type inference, constant evaluation, monomorphization, or backend lowering here.

## Compatibility

- Use `Hir` and `HirBuilder` as the public entry points.

## Primary Public Items

- `Hir`: current public HIR container produced from AST and name data.
- `HirBuilder`: current public HIR builder.
- `Item`, `Signature`, `Block`, `Field`, `Variant`, `AssocItem`, `Type`: source-level components.
- `Generics`, `GenericParam`, `TypeParamBound`: item-level generic parameter
  and lowered bound facts for upper phases.
