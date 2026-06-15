# Instructions

## Role

- Own the reusable name-resolution model.
- Model definitions, scopes, namespaces, bindings, imports, lookup results, and
  semantic visibility.
- Provide the name facts that upper semantic phases query from HIR ids.

## Boundaries

- Keep the core model AST-agnostic.
- Keep AST-aware collection isolated in a dedicated module.
- Do not make collection read files or depend on `TopCx`.
- Let higher layers attach AST-specific identities through opaque origins.
- Expose owned facts through focused `NameDb` query APIs instead of making
  callers reconstruct them from raw storage.
- Do not add type inference, evaluation, monomorphization, or backend lowering here.

## Model

- Keep Rust namespaces separate: type, value, macro, and lifetime.
- Represent generic parameters as definitions.
- Do not recover generic parameters through ad hoc syntax ancestry.
- Keep definition-attached scope roles separate.
- Use `DefScopes::path` for path-reachable children such as enum variants.
- Use `DefScopes::generic` for lexical generic parameters.
- Use `DefScopes::body` for value-body bindings.
- Make name resolution use-site based, scope-aware, and namespace-aware.
- Do not force struct fields into `DefScopes::path`.
- Add a dedicated field/member concept if field modeling becomes necessary.

## Primary Public Items

- `NameDb`: scopes, definitions, and imports.
- `DefId`, `ScopeId`, `ImportId`: stable database ids.
- `Def`, `DefKind`, `Visibility`, `Origin`: definition metadata.
- `Scope`, `ScopeKind`, `Bindings`, `Binding`: lexical scope and binding model.
- `Namespace`: Rust namespace selector.
- `Import`, `ImportKind`, `ImportStatus`: import model and resolution state.
- `ResolveResult`: result of resolving a name from a use site.
- `Name`: interned name text tied to shared common infrastructure.
- `collect`: AST-aware collection from already parsed file inputs.
- `collect::ModulePath`: module path helper for callers that prepare those
  file inputs without making `syn-sem-name` read files.
