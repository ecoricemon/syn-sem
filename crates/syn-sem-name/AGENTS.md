# Instructions

## Role

- Own the reusable name-resolution model.
- Model definitions, scopes, namespaces, bindings, imports, lookup results, and visibility.
- Provide name facts queried by upper semantic phases.

## Boundaries

- Keep the core model AST-agnostic.
- Keep AST-aware collection isolated in a dedicated module.
- Do not make collection read files or depend on `TopCx`.
- Let higher layers attach AST-specific identities through opaque origins.
- Expose owned facts through focused `NameDb` query APIs.
- Do not add type inference, evaluation, monomorphization, or backend lowering here.

## Model

- Keep Rust namespaces separate: type, value, macro, and lifetime.
- Represent generic parameters as definitions.
- Do not recover generic parameters through ad hoc syntax ancestry.
- Keep definition-attached scope roles separate.
- Use `DefScopes::{path,generic,body}` for path children, generics, and body bindings.
- Make name resolution use-site based, scope-aware, and namespace-aware.
- Do not force struct fields into `DefScopes::path`.
- Add a dedicated field/member concept if field modeling becomes necessary.

## Entry Points

- Start from `NameDb` for name facts and `collect::NameCollector` for AST-aware collection.
