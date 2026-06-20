# Instructions

## Role

- Own procedural macros used by extracted `syn-sem` crates.

## Boundaries

- Keep this crate limited to small compile-time helpers.
- Do not add runtime semantic analysis, shared models, or phase behavior here.
- Generate straightforward code with inspectable expansion behavior.

## Model

- Prefer explicit compile-time checks over generated runtime behavior for crate invariants.

## Primary Public Items

- `CheckDropless`.
