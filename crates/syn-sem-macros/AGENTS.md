# Instructions

## Role

- Own procedural macros used by extracted `syn-sem` crates.

## Boundaries

- Keep this crate limited to small compile-time helpers.
- Do not add runtime semantic-analysis logic here.
- Do not add shared data models or phase-specific behavior here.
- Generate straightforward code with inspectable expansion behavior.

## Model

- Prefer explicit compile-time checks over generated runtime behavior for crate invariants.

## Primary Public Items

- `CheckDropless`: derive macro asserting that the target type does not need drop.
