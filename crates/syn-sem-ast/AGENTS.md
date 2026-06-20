# Instructions

## Role

- Own the lifetime-bearing semantic AST wrapper over `syn` syntax trees.
- Parse and store source files through `SyntaxCx`.
- Convert raw `syn` nodes into construction-layer AST nodes.

## Boundaries

- Keep this crate focused on syntax-shaped AST data and source mapping.
- Do not make upper semantic phases depend on this crate directly.
- Do not add name resolution, type inference, evaluation, monomorphization, or
  backend lowering here.

## Model

- Keep AST nodes dropless.
- Allocate AST nodes from `SyntaxCx` when they need context-owned storage.
- Get interned strings, file paths, and source text through `SyntaxCx`.
- Prefer semantic wrappers over raw `syn` nodes in production APIs.
- Keep raw `syn` mainly at conversion boundaries.

## Primary Public Items

- `SyntaxCx`, `FromSyn`, `InputDesc`.
- `Source`, `SourceKind`, `Ident`, `Span`.
- `File`, `Item`, `Expr`, `Type`, `Pat`, `Path`, `Generics`.
