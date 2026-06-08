# Instructions

## Role

- Own the lifetime-bearing semantic AST wrapper over `syn` syntax trees.
- Parse and store source files through `SyntaxCx`.
- Convert raw `syn` nodes into construction-layer AST nodes for `syn-sem-top`
  and extracted representation builders.

## Boundaries

- Keep this crate focused on syntax-shaped AST data and source mapping.
- Do not make upper semantic phases depend on this crate directly.
- Do not add name resolution, type inference, evaluation, monomorphization, or
  backend lowering here.
- Keep phase logic in sibling phase crates or top-level orchestration.

## Model

- Keep AST nodes dropless.
- Allocate AST nodes from `SyntaxCx` when they need context-owned storage.
- Get interned strings, file paths, and source text through `syn-sem-common`.
- Borrow the shared `CommonCx` through `SyntaxCx`.
- Prefer semantic wrappers over raw `syn` nodes in production APIs.
- Keep raw `syn` mainly at conversion boundaries.

## Primary Public Items

- `SyntaxCx`: allocation, interning, parsing, and source-storage context.
- `FromSyn`: conversion trait from raw `syn` nodes.
- `InputDesc`: source-file and input-node descriptor for conversion.
- `Source`, `SourceKind`: parsed source metadata and source classification.
- `File`, `Item`, `Expr`, `Type`, `Pat`, `Path`, `Generics`: semantic AST node families.
- `Ident`, `Span`: source-aware identifiers and locations.
