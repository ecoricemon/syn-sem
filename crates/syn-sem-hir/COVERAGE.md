# syn-sem-hir / HIR Coverage Matrix

This document tracks how the current HIR source spine covers the semantic AST
surface supported by `syn-sem-ast`.

`syn-sem-hir` owns HIR source spine construction plus HIR lowering for upper
semantic phases. Many rows are intentionally "indexed with AST source" rather
than fully HIR-native. Retained AST references are construction/source anchors
for spans, diagnostics, and source mapping, not data later phases should inspect
directly.

## Status Legend

- `Indexed`: HIR creates a stable id and records HIR-level links.
- `AST source`: HIR keeps an explicit `&ast::...` source reference as an anchor.
- `Partial`: HIR records only selected native facts.
- `Missing`: no HIR entry is created yet.

## Items

| AST surface | HIR coverage | Current HIR data | AST exposure | Notes |
| --- | --- | --- | --- | --- |
| `Item::Const` | Indexed | `Item { name, visibility, def, parent_scope }`, `ItemKind::Const { ty, init }` | None at item layer | Initializer is indexed as an `ExprId`. |
| `Item::Enum` | Indexed | `Item { name, visibility, def, parent_scope }`, `ItemKind::Enum { generics, variants }` | None at item layer | Variants are separately indexed. Type parameter trait bounds are HIR-native. |
| `Item::Fn` | Indexed | `Item { name, visibility, def, parent_scope }`, `ItemKind::Fn { generics, signature, block }` | None at item layer | Function body is a `BlockId`; signature input parameter patterns link to `PatId`s. |
| `Item::Impl` | Indexed | `Item { name: None, visibility, def, parent_scope }`, `ItemKind::Impl { generics, trait_, self_ty, items }` | None at item layer | `trait_` is a HIR-native type path with generic arguments. Associated item def links are currently mostly absent because impl scope is not exposed through `DefScopes`. |
| `Item::Mod` | Indexed | `Item { name, visibility, def, parent_scope }`, `ItemKind::Mod { is_inline, scope, items }` | None at item layer | Inline module children are indexed. File-backed module children are not represented by `HirBuilder::build` today. |
| `Item::Struct` | Indexed | `Item { name, visibility, def, parent_scope }`, `ItemKind::Struct { generics, fields }` | None at item layer | Fields are separately indexed. Type parameter trait bounds are HIR-native. |
| `Item::Trait` | Indexed | `Item { name, visibility, def, parent_scope }`, `ItemKind::Trait { generics, items }` | None at item layer | Associated item def links are currently mostly absent because trait scope is not exposed through `DefScopes`. |
| `Item::Type` | Indexed | `Item { name, visibility, def, parent_scope }`, `ItemKind::Type { generics, ty }` | None at item layer | Alias target gets a `TypeId`. |
| `Item::Use` | Partial | `Item { visibility, parent_scope }`, `ItemKind::Use` | None at item layer | Import declarations are not linked to `ImportId` or `DefKind::Use` aliases yet. |

## Associated Items

| AST surface | HIR coverage | Current HIR data | AST exposure | Notes |
| --- | --- | --- | --- | --- |
| `ImplItem::Const` | Indexed | `AssocItem { name, def }`, `AssocItemKind::ImplConst { ty, init }` | None at associated item layer | Initializer is indexed as an `ExprId`. |
| `ImplItem::Fn` | Indexed | `AssocItem { name, def }`, `AssocItemKind::ImplFn { signature, block }` | None at associated item layer | Function body is a `BlockId`. |
| `ImplItem::Type` | Indexed | `AssocItem { name, def }`, `AssocItemKind::ImplType { ty }` | None at associated item layer | Assigned type gets a `TypeId`. |
| `TraitItem::Const` | Indexed | `AssocItem { name, def }`, `AssocItemKind::TraitConst { ty, default }` | None at associated item layer | Default expression, when present, is indexed as an `ExprId`. |
| `TraitItem::Fn` | Indexed | `AssocItem { name, def }`, `AssocItemKind::TraitFn { signature, default }` | None at associated item layer | Default block, when present, is a `BlockId`. |
| `TraitItem::Type` | Indexed | `AssocItem { name, def }`, `AssocItemKind::TraitType { default }` | None at associated item layer | Default type, when present, gets a `TypeId`. |

## Declarations Inside Items

| AST surface | HIR coverage | Current HIR data | AST exposure | Notes |
| --- | --- | --- | --- | --- |
| `Signature` | Indexed | `Signature { source, params }`, `SignatureParam { ty, pat }` | None at signature layer | `params[0]` is the output type with no pattern; `params[1..]` are input parameters with `PatId`s. |
| `Field` | Indexed | `Field { name, visibility, ty, source }` | None at field layer | Struct field visibility is HIR-native; variant fields use private visibility. |
| `Variant` | Indexed | `Variant { name, def, fields, discriminant }` | None at variant layer | Variant payload fields are indexed; explicit discriminant is indexed as an `ExprId`. |
| `Generics` | Indexed | `Generics { params, predicates }`, `GenericParam::{Type, Const, Unsupported}`, `WherePredicate::TypeBound`, `TypeParamBound::Trait` | None at item layer | Inline type parameter bounds are lowered into generic predicates alongside source where-clause predicates. |
| `Visibility` | Partial | `Visibility::{Public, Restricted, Private}` on items and fields | Restricted paths are HIR-native segment lists | Semantic visibility interactions belong to name-resolution data. |

## Types

| AST surface | HIR coverage | Current HIR data | AST exposure | Notes |
| --- | --- | --- | --- | --- |
| `Type::Array` | Indexed | `Type { kind: TypeKind::Array { elem, len }, scope }` | Retained source anchor only | Element type is indexed as `TypeSource::Nested`; length is indexed as an `ExprId`. |
| `Type::Infer` | Indexed | `Type { kind: TypeKind::Infer, scope }` | Retained source anchor only | Represents `_` without requiring later phases to inspect AST. |
| `Type::Path` | Indexed | `Type { kind: TypeKind::Path, scope }` with optional qualified self type, segments, and generic argument shape | Retained source anchor only | Qualified self types link to nested `TypeId`s; type and associated-type generic arguments also link to nested `TypeId`s; associated type constraints keep HIR-native bounds; const args and associated consts use `ConstArg::{Lit, Path, Expr}`. |
| `Type::Reference` | Indexed | `Type { kind: TypeKind::Reference { elem, is_mut }, scope }` | Retained source anchor only | Referenced type is indexed as `TypeSource::Nested`. |
| `Type::Slice` | Indexed | `Type { kind: TypeKind::Slice { elem }, scope }` | Retained source anchor only | Element type is indexed as `TypeSource::Nested`. |
| `Type::Tuple` | Indexed | `Type { kind: TypeKind::Tuple { elems }, scope }` | Retained source anchor only | Tuple element types are indexed as `TypeSource::Nested`. |

Current `TypeSource` roles are `ConstType`, `SignatureParam`,
`ImplSelf`, `StructField`, `VariantField`, `TypeAlias`, `AssocConstType`,
`AssocTypeValue`, `GenericParamDefault`, `ConstGenericParam`, and
`WherePredicateSubject`; nested type
entries use `Nested`.

## Blocks, Statements, Expressions, and Patterns

| AST surface | HIR coverage | Current HIR data | AST exposure | Notes |
| --- | --- | --- | --- | --- |
| Function block bodies | Indexed | `Block { scope, stmts }` linked directly from function-like owners | Retained source anchor only | Block contents link to `StmtId`s in source order. |
| Const initializers | Indexed | `ExprId` links into the expression arena | Retained source anchor only | Includes free consts, impl consts, trait const defaults, and variant discriminants. |
| `Stmt::{Local, Item, Expr}` | Partial | `Stmt { kind, scope }` with `StmtKind::{Local, Item, Expr}` | Retained source anchor only | Local and expression statements link to `LocalId` and `ExprId`; block-local item statements are classified but do not link to `ItemId` yet. |
| `Local` and `LocalInit` | Partial | `Local { pat, init, scope }` | None at local layer | Local patterns link to `PatId`; local initializer expressions link to `ExprId`. |
| `Pat` variants | Partial by design | `Pat { kind, scope }` with `PatKind::{Ident, Path, Struct, Reference, Tuple, Type, Unsupported}` | Retained source anchor only | Identifier, path, struct, reference, tuple, and type-annotated patterns are HIR-native; literal, rest, and slice patterns intentionally remain `Unsupported` until a consumer needs them. |
| `Expr` variants | Indexed | `Expr { kind, scope }` plus child `ExprId`, `BlockId`, `SignatureId`, and `TypeId` links | Retained source anchor only | Operators are still source-anchored where the AST has not exposed a HIR-native operator kind. |

Current expression, statement, local, and pattern entries use stable ids as the
HIR source spine. Full pattern coverage is not a goal by itself; add unsupported
pattern variants when an upper-phase consumer needs them. Block-local item links
are still future work.

## Current Lowering Roles

Current generic predicate integration lives under `src/lower/` as HIR generics
lowering. Future body/control-flow lowering and inference preprocessing should
be added as HIR lowering layers while keeping source spine construction
separate.

## Current Test Coverage

Existing `syn-sem-hir` tests now cover the main source-spine rows in this matrix:
supported item kinds, associated item kinds, `TypeSource` roles, block handles,
source-expression ids, inline and file-backed module shape, struct and
variant fields, variant discriminants, generic predicates, const generic
arguments, associated const arguments, and simple `DefId` linking behavior.

They still do not prove full HIR-native conversion for every pattern variant or
block-local item statement because those rows remain intentionally partial.
`Type`, `Block`, `Stmt`, `Local`, `Pat`, and `Expr` still expose AST payloads as
the current source boundary.
