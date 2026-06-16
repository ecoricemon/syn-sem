# syn-sem-hir Coverage

## Keys

- `Indexed`: stable HIR id exists.
- `Native`: HIR-owned shape exists.
- `Lowered`: `lower::*` fact exists.
- `AST anchor`: retained `&ast::*` for source/diagnostics only.
- `Partial`: selected facts only.

## Summary

- Items: all supported `ast::Item` forms are `Indexed` + `Native`.
- Associated items: all supported impl/trait associated item forms are `Indexed` + `Native`.
- Declarations: signatures, fields, variants, generics, and visibility have HIR-native facts.
- Types: all current `ast::Type` forms are `Indexed` + `Native` + `AST anchor`.
- Bodies: blocks, statements, locals, patterns, and expressions are stable source-spine arenas.
- Lowering: generic predicates and selected body facts are exposed through `lower::*`.

## Item Gaps

- `Item::Mod`: inline children represented; file-backed module children are not built by `HirBuilder::build`.
- `Item::Impl`: associated item `DefId` links depend on name-layer scope coverage.
- `Item::Trait`: associated item `DefId` links depend on name-layer scope coverage.
- `Item::Use`: imports link to `ImportId`; alias definitions and resolution stay in `syn-sem-name`.
- Visibility: syntax shape is represented; semantic visibility stays in `syn-sem-name`.

## Declaration Coverage

- `Signature`: output type is `params[0]`; input params link to `PatId`.
- `Generics`: type params, const params, inline bounds, and where predicates are represented.
- Generic predicate lowering: inline type bounds + source where predicates -> `WherePredicate`.
- Unsupported generic params, bounds, predicates, and generic args remain explicit `Unsupported` variants.

## Type Coverage

- Covered type forms: array, infer, path, reference, slice, tuple.
- Path type coverage: `QSelf`, full source path segments, generic args, assoc type args, assoc const args, constraints.
- Const arg coverage: literal, path, expression.
- Type roles: `TypeSource` tracks declaration role plus `Nested`.
- Out of scope: semantic path classification and type resolution.

## Body Coverage

- `Block`: source block id, statement order, scope, lowered tail expression.
- `Stmt`: local, item, expression statement links.
- `Local`: source local id, `pat: PatId`, `init: Option<ExprId>`, scope.
- `lower::Body`: lowered blocks in source order.
- `lower::Block`: lowered statements + `tail_expr`.
- `lower::Local`: `local`, `pat`, introduced local `DefId`s, initializer.

## Pattern Coverage

- Supported: ident, reference, path, struct, tuple, type-annotated.
- `PatKind::Ident`: stores binding `DefId` when name collection provides it.
- Unsupported by design: literal, rest, slice, future forms.
- Add unsupported pattern variants only when a consumer needs their structure.

## Expression Coverage

- Current expression forms are indexed as `ExprId` with HIR-native `ExprKind`.
- Child links use `ExprId`, `BlockId`, `SignatureId`, and `TypeId`.
- Operator payload remains source-anchored where AST has no HIR-native operator kind.
- Method, field, path, and type resolution stay outside HIR.

## AST Anchors

- Retained on source-spine nodes for source mapping and diagnostics.
- Not intended as semantic input for upper phases.
- Non-diagnostic body consumers should prefer `lower::Body` facts.

## Tests

- Covered: item/associated item shapes, type roles, modules, fields, variants, generics, const args, associated args, simple `DefId` links.
- Covered: block handles, expression ids, lowered statement order, lowered local bindings, lowered tail expressions.
- Intentional gaps: full pattern coverage, AST-anchor removal, full control-flow lowering.
