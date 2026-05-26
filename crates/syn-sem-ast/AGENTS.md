# AGENTS.md

## Crate Focus

`syn-sem-ast` provides a lifetime-bearing semantic AST wrapper over `syn`
syntax trees. It parses and stores source files through `SyntaxCx`, then
converts raw `syn` nodes into AST nodes that other extracted crates can inspect
without depending directly on `syn`.

## Boundaries

Keep this crate focused on syntax-shaped AST data and source mapping.

Do not add name resolution, type inference, evaluation, monomorphization, or
backend lowering responsibilities here. Those belong in sibling phase crates or
top-level orchestration.

## Model Rules

AST nodes should be dropless and allocated from `SyntaxCx` when they need
context-owned storage.

Interned strings, file paths, and source text should come through
`syn-sem-common` aliases and the shared `CommonCx` borrowed by `SyntaxCx`.

Prefer semantic wrappers over exposing raw `syn` nodes through production APIs.
Raw `syn` should mainly appear at conversion boundaries.

## Primary Public Items

- `SyntaxCx`: allocation, interning, parsing, and source-storage context for the
  semantic AST.
- `FromSyn`: conversion trait from raw `syn` nodes into semantic AST nodes.
- `InputDesc`: source-file and input-node descriptor passed during conversion.
- `Source` and `SourceKind`: parsed source metadata and physical/virtual source
  classification.
- `File`, `Item`, `Expr`, `Type`, `Pat`, `Path`, `Generics`, and related node
  families: semantic AST views over Rust syntax.
- `Ident` and `Span`: source-aware identifiers and locations used throughout
  AST nodes.
