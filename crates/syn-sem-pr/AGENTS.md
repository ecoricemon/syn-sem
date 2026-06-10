# Instructions

## Role

- Own the upper-phase-facing Rust source program representation.
- Treat `pr` as Program Representation.
- Represent source program shape and name-fact connection ids for other phases.

## Boundaries

- Keep this crate focused on the current program representation layer.
- Store ids that connect represented source shape to facts owned by `syn-sem-name`.
- Do not duplicate resolved definition, scope, import, or semantic visibility facts here.
- Do not treat current name-resolution ownership as fixed architecture.
- Do not bake in permanent phase ordering around inference, evaluation,
  monomorphization, or backend lowering.

## Model

- Prefer Rust-shaped semantic data over backend-oriented or fully lowered forms.
- Represent source declarations and bodies explicitly.
- Do not make later phases recover source structure from raw AST nodes.
- Leave room for future block/body desugaring IR.
- Keep ownership and lifetimes tied to the shared top-level context.
- Do not construct deep context chains here.
- Do not add type inference, constant evaluation, monomorphization, or backend lowering here.

## Possible V2 Directions

- May model generics in representation-native data.
- May expose function parameters and pattern names in a more upper-phase-friendly form.
- May give `Body` statement, expression, or pattern entry points.
- May further refine path and type occurrence representation for upper semantic phases.
- May link `use` items to import or alias information without owning name resolution.
- May add traversal helpers when they encode real program-shape relationships.

## Primary Public Items

- `ProgramRepr`: program representation produced from AST and name-resolution data.
- `Item`, `Signature`, `Body`, `Field`, `Variant`, `AssocItem`, `Type`: source-level components.
