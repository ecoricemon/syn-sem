# Instructions

## Role

- Own shared infrastructure for extracted `syn-sem` crates.
- Own common context state, lifetime-bearing string interning, source identifiers,
  and abstract source files.

## Boundaries

- Keep this crate independent from syntax, AST, name-resolution, semantic,
  inference, evaluation, and backend crates.
- Do not add domain-specific semantic concepts here.
- Add reusable infrastructure here only when it is genuinely shared and lower
  level than phase crates.

## Model

- Use lifetime-bearing interned string aliases from this crate.
- Prefer `InternedStr<'cx>`, `FilePath<'cx>`, and `SourceText<'cx>`.
- Do not expose or store `RawInterned<str>` in crate APIs.
- Tie interned source paths and source text to the `CommonCx` or `StringInterner`
  that produced them.

## Primary Public Items

- `CommonCx`: shared infrastructure context.
- `StringInterner`: string-only interner used by `CommonCx`.
- `AbstractFiles`: interner-independent source-file table keyed by owned paths.
- `InternedStr`, `FilePath`, `SourceText`, `LibraryName`: lifetime-bearing
  interned string aliases.
- `Result`, `Error`, `Map`: shared utility aliases for internal crates.
