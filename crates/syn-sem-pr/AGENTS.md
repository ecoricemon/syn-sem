# Instructions

## Role

- Own the Rust source program representation.
- Treat `pr` as Program Representation.
- Represent items, signatures, fields, bodies, and type occurrences for other phases.

## Boundaries

- Keep this crate focused on the current program representation layer.
- Refer to definitions, scopes, and imports through `syn-sem-name` ids/data when needed.
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

## Primary Public Items

- `ProgramRepr`: program representation produced from AST and name-resolution data.
- `Item`, `Signature`, `Body`, `Field`, `Variant`, `AssocItem`, `Type`: source-level components.
