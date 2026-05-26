# AGENTS.md

## Crate Focus

`syn-sem-common` owns shared infrastructure used by extracted `syn-sem` crates:
common context state, lifetime-bearing string interning, interned source
identifiers, and abstract source files.

## Boundaries

Keep this crate independent from syntax, AST, name-resolution, semantic,
inference, evaluation, and backend crates.

Do not add domain-specific semantic concepts here. Reusable infrastructure may
live here only when it is genuinely shared and lower-level than the extracted
phase crates.

## Model Rules

Use lifetime-bearing interned string aliases from this crate:

```rust
InternedStr<'cx>
FilePath<'cx>
SourceCode<'cx>
```

Do not expose or store `RawInterned<str>` in crate APIs.

Interned source paths and source text should be tied to the `CommonCx` or
`StringInterner` that produced them.

## Primary Public Items

- `CommonCx`: root context for shared infrastructure, especially string
  interning.
- `StringInterner`: string-only interner used by `CommonCx`.
- `AbstractFiles`: abstract source-file table keyed by interned file paths.
- `Source`: source metadata for physical or virtual files.
- `InternedStr`, `FilePath`, `SourceCode`, and `LibraryName`: lifetime-bearing
  interned string aliases.
- `Result`, `Error`, and `Map`: shared utility aliases for internal crates.
