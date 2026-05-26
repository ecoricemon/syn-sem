# AGENTS.md

## Crate Focus

`syn-sem-forest` owns the raw `syn` syntax forest: pinned parsed files, cloned
syntax fragments, syntax-node identity, source-location lookup, and parent/child
navigation over raw syntax nodes.

## Boundaries

Keep this crate below the semantic AST layer. It may depend on `syn` and shared
common infrastructure, but it should not add semantic AST wrappers,
name-resolution, type inference, evaluation, monomorphization policy, or backend
lowering.

Do not move higher-level semantic meaning into syntax identity or parent lookup.

## Model Rules

Stored raw syntax roots should remain pinned so pointer-based syntax identity
stays valid.

`SynId` identifies a specific raw `syn` node instance. It should remain an
identity/source-navigation mechanism, not a semantic definition identifier.

Parent lookup should describe raw syntax containment only. Semantic ownership,
scope, and resolution relationships belong in higher-level crates.

## Primary Public Items

- `SyntaxForest`: collection of pinned raw syntax roots plus lookup metadata.
- `File`: pinned parsed `syn::File` with locator state and interned file path.
- `ClonedImpl`: cloned `syn::ItemImpl` with independent locator state.
- `SynId` and `IdentifySyn`: syntax-node identity over raw `syn` nodes.
- `ParentFinder` and `InsertRelation`: parent-relation construction and lookup.
- `FindChild` and `AttributeHelper`: convenience helpers for raw syntax
  inspection.
