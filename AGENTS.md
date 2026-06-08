# Instructions

## Project

- Extract focused sub-crates from `crates/syn-sem` incrementally.
- Keep `syn-sem` as the facade/orchestrator while internals migrate.
- Implement new migration work in `syn-sem-top` instead of reshaping `syn-sem`.
- Treat current crate boundaries, phase ordering, and ownership choices as
  migration guidance unless this file or the active user task makes them durable.
- Prefer the nearest crate-local `AGENTS.md` for crate-specific rules.

## Instruction Files

- Keep `AGENTS.md` files short, stable, and action-oriented.
- Do not use `AGENTS.md` as a design log.
- Follow the user's latest explicit task instruction over stale guidance here.
- Update `AGENTS.md` only for decisions expected to last across many tasks.

## Contexts

- Use a flattened context hierarchy.
- Keep `CommonCx` as bottom/shared infrastructure.
- Keep phase contexts as top-level siblings that borrow the contexts they need.
- Put self-referential ownership and lifetime wiring only in the top-level root.
- Do not make lower-level contexts own parent contexts or build deep context chains.
- Treat `TopCx` as the self-referential root like the old `GlobalCx`.

## Names

- Bind `CommonCx` as `ccx`.
- Bind `SyntaxCx` as `scx`.
- Bind `TopCx` as `tcx`.
- Write the top-level root as `TopCx<'tcx>`.
- Write non-top-level context types with `'cx`, for example `FooCx<'cx>`.
- Use referent-specific lifetimes elsewhere, for example `&'ccx CommonCx`.

## Crate Boundaries

- Keep extracted crates focused.
- Respect crate-local boundary rules before adding dependencies or moving public items.
- Expose lower-phase owned facts through focused query APIs.
- Make higher phases ask owning crates for definition, source, scope, and resolution facts.
- Extract reusable infrastructure before wiring it deeply into `syn-sem`.
- Let `PathTree` remain temporarily, but move new name-resolution work toward `syn-sem-name`.

## Tests

- Run `cargo fmt` before finalizing changes.
- Check all sub-crates with `cargo check`, `cargo clippy`, and `cargo test`.
- For extracted crates, prefer `cargo rustdoc -p <crate> -- -D missing_docs`.
- Add rustdoc comments for new public items in extracted crates.

## Style

- Keep `lib.rs` clean.
- Prefer incremental refactors.
- Make each step compile and pass focused tests before continuing.
- Avoid broad rewrites unless they directly support the current extraction step.
