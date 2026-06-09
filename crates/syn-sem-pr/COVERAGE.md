# syn-sem-pr Coverage Matrix

This document tracks how the current Rust source program representation covers
the semantic AST surface supported by `syn-sem-ast`.

`syn-sem-pr` is still a V1 program representation. Many rows are intentionally
"indexed with AST source" rather than fully repr-native. Type occurrences now
also carry representation-native shape through `TypeKind`; the retained AST
source reference is a construction/source anchor, not the data later phases
should inspect.

## Status Legend

- `Indexed`: the representation creates a stable representation id and records representation-level links.
- `AST source`: the representation keeps an explicit `&ast::...` source reference.
- `Partial`: the representation records only selected repr-native facts.
- `Missing`: no representation entry is created yet.

## Items

| AST surface | ProgramRepr coverage | Current representation data | AST exposure | Notes |
| --- | --- | --- | --- | --- |
| `Item::Const` | Indexed | `Item { name, source_visibility, def, parent_scope }`, `ItemKind::Const { ty, body }` | None at item layer | Initializer is a `BodyId` with `BodyKind::Expr`. |
| `Item::Enum` | Indexed | `Item { name, source_visibility, def, parent_scope }`, `ItemKind::Enum { variants }` | None at item layer | Variants are separately indexed. Generic data is not repr-native yet. |
| `Item::Fn` | Indexed | `Item { name, source_visibility, def, parent_scope }`, `ItemKind::Fn { signature, body }` | None at item layer | Body is a `BodyId` with `BodyKind::Block`. |
| `Item::Impl` | Indexed | `Item { name: None, source_visibility, def, parent_scope }`, `ItemKind::Impl { trait_, self_ty, items }` | None at item layer | `trait_` is a repr-native path. Associated item def links are currently mostly absent because impl scope is not exposed through `DefScopes`. |
| `Item::Mod` | Indexed | `Item { name, source_visibility, def, parent_scope }`, `ItemKind::Mod { is_inline, scope, items }` | None at item layer | Inline module children are indexed. File-backed module children are not represented by `ProgramReprBuilder::build` today. |
| `Item::Struct` | Indexed | `Item { name, source_visibility, def, parent_scope }`, `ItemKind::Struct { fields }` | None at item layer | Fields are separately indexed. Generic data is not repr-native yet. |
| `Item::Trait` | Indexed | `Item { name, source_visibility, def, parent_scope }`, `ItemKind::Trait { items }` | None at item layer | Associated item def links are currently mostly absent because trait scope is not exposed through `DefScopes`. |
| `Item::Type` | Indexed | `Item { name, source_visibility, def, parent_scope }`, `ItemKind::Type { ty }` | None at item layer | Alias target gets a `TypeId`. |
| `Item::Use` | Partial | `Item { source_visibility, parent_scope }`, `ItemKind::Use` | None at item layer | Import declarations are not linked to `ImportId` or `DefKind::Use` aliases yet. |

## Associated Items

| AST surface | ProgramRepr coverage | Current representation data | AST exposure | Notes |
| --- | --- | --- | --- | --- |
| `ImplItem::Const` | Indexed | `AssocItem { name, def }`, `AssocItemKind::ImplConst { ty, body }` | None at associated item layer | Initializer is a `BodyId` with owner `BodyOwner::AssocItem`. |
| `ImplItem::Fn` | Indexed | `AssocItem { name, def }`, `AssocItemKind::ImplFn { signature, body }` | None at associated item layer | Body is a `BodyId` with owner `BodyOwner::AssocItem`. |
| `ImplItem::Type` | Indexed | `AssocItem { name, def }`, `AssocItemKind::ImplType { ty }` | None at associated item layer | Assigned type gets a `TypeId`. |
| `TraitItem::Const` | Indexed | `AssocItem { name, def }`, `AssocItemKind::TraitConst { ty, default }` | None at associated item layer | Default expression, when present, is a `BodyId`. |
| `TraitItem::Fn` | Indexed | `AssocItem { name, def }`, `AssocItemKind::TraitFn { signature, default }` | None at associated item layer | Default block, when present, is a `BodyId`. |
| `TraitItem::Type` | Indexed | `AssocItem { name, def }`, `AssocItemKind::TraitType { default }` | None at associated item layer | Default type, when present, gets a `TypeId`. |

## Declarations Inside Items

| AST surface | ProgramRepr coverage | Current representation data | AST exposure | Notes |
| --- | --- | --- | --- | --- |
| `Signature` | Indexed | `Signature { source, types }` | None at signature layer | Return type and parameter types each get `TypeId`; parameter names/patterns/generics are not repr-native yet. |
| `Field` | Indexed | `Field { name, source_visibility, ty, source }` | None at field layer | Struct field source visibility is repr-native; variant fields use private source visibility. |
| `Variant` | Indexed | `Variant { name, def, fields, discriminant }` | None at variant layer | Variant payload fields are indexed; explicit discriminant becomes a `BodyId`. |
| `Generics` | Missing | None | Via owning AST refs only | Generic params and where clauses are not repr-native yet. |
| `SourceVisibility` | Partial | `SourceVisibility::{Public, Restricted, Private}` on items and fields | Restricted paths are repr-native segment lists | Semantic visibility interactions belong to name-resolution data. |

## Types

| AST surface | ProgramRepr coverage | Current representation data | AST exposure | Notes |
| --- | --- | --- | --- | --- |
| `Type::Array` | Indexed | `Type { kind: TypeKind::Array { elem, len }, scope }` | Retained source anchor only | Element type is indexed as `TypeSource::Nested`; length remains a source-expression placeholder until expression representation exists. |
| `Type::Infer` | Indexed | `Type { kind: TypeKind::Infer, scope }` | Retained source anchor only | Represents `_` without requiring later phases to inspect AST. |
| `Type::Path` | Indexed | `Type { kind: TypeKind::Path, scope }` with segments and generic argument shape | Retained source anchor only | Type and associated-type generic arguments link to nested `TypeId`s; const args, associated consts, and constraints remain placeholders until expression/bound representation exists. |
| `Type::Reference` | Indexed | `Type { kind: TypeKind::Reference { elem, is_mut }, scope }` | Retained source anchor only | Referenced type is indexed as `TypeSource::Nested`. |
| `Type::Slice` | Indexed | `Type { kind: TypeKind::Slice { elem }, scope }` | Retained source anchor only | Element type is indexed as `TypeSource::Nested`. |
| `Type::Tuple` | Indexed | `Type { kind: TypeKind::Tuple { elems }, scope }` | Retained source anchor only | Tuple element types are indexed as `TypeSource::Nested`. |

Current `TypeSource` roles are `ConstType`, `SignatureParam`, `ImplSelf`,
`StructField`, `VariantField`, `TypeAlias`, `AssocConstType`, and
`AssocTypeValue`; nested type entries use `Nested`.

## Bodies, Statements, Expressions, and Patterns

| AST surface | ProgramRepr coverage | Current representation data | AST exposure | Notes |
| --- | --- | --- | --- | --- |
| Function block bodies | Indexed | `Body { owner, scope, kind: BodyKind::Block }` | None at body layer | No statement-level or expression-level model is created yet. |
| Const initializers | Indexed | `Body { owner, kind: BodyKind::Expr }` | None at body layer | Includes free consts, impl consts, trait const defaults, and variant discriminants. |
| `Stmt::{Local, Item, Expr}` | Missing | None | None through public body data | Block-local items and locals are not repr-native. |
| `Local` and `LocalInit` | Missing | None | None through public body data | Local initializer expressions do not get separate `BodyId`s today. |
| `Pat` variants | Missing | None | None through public signature/body data | Parameter patterns are not repr-native yet. |
| `Expr` variants | Missing | None | None through public body data | Desugared body IR is intentionally not implemented in V1. |

Current `BodyOwner` variants are `Item`, `AssocItem`, and `Variant`.

## Current Test Coverage

Existing `syn-sem-pr` tests now cover the main rows in this matrix:
supported item kinds, associated item kinds, `TypeSource` roles, `BodyOwner`
roles, body kinds, inline and file-backed module shape, struct and variant
fields, variant discriminants, and simple `DefId` linking behavior.

They still do not prove full repr-native conversion for generics, statements,
locals, patterns, or expression trees because those rows remain intentionally
missing in V1. `Type` still exposes AST payloads as the current source type
boundary; `Body` now exposes only owner, scope, and source kind.
