# syn-sem

`syn-sem` is an experimental semantic analyzer for a subset of Rust.

It starts from Rust source code parsed by [`syn`](https://github.com/dtolnay/syn)
and builds semantic information you can inspect from your own code: module
paths, resolved items, inferred types, evaluated constants, trait logic, and
monomorphized generic code.

This crate is early-stage (`0.0.1`). The API is useful for experiments, but it
is still unstable and may change.

## What This Crate Is For

Use `syn-sem` when you want to ask questions about Rust code that are deeper
than syntax:

- Where is this item in the module tree?
- Which item does this `use` path resolve to?
- What type did this local variable or field get?
- What is the value of this compile-time constant?
- What does an impl block contribute to a type?

The supported language is intentionally smaller than full Rust. It is best for
small analysis tools, experiments, and learning how semantic analysis can be
layered on top of syntax.

## Install

Add the crate to `Cargo.toml`:

```toml
[dependencies]
syn-sem = "0.0.1"
```

When working inside this repository, run examples with Cargo's package flag:

```sh
cargo run -p syn-sem --example basic
```

## Run The Examples

The examples under `examples/` are meant to be read in order.

```sh
cargo run -p syn-sem --example basic
cargo run -p syn-sem --example type_inference
cargo run -p syn-sem --example constant_evaluation
```

- `basic` shows the smallest in-memory analysis flow and item lookup.
- `type_inference` shows how to inspect inferred local types with `GetOwned`.
- `constant_evaluation` shows evaluated constants and fixed array lengths.

## Reading Analysis Results

`AnalysisSession::run` returns an `Analyzed` value. The semantic results live in
`analyzed.sem`.

### Path Tree

`sem.ptree` is the path tree. It stores the crate/module hierarchy and the items
found at each path.

```rust,ignore
let ptree = &analyzed.sem.ptree;
let crate_name = ptree.crate_name();
let item = pitem!(ptree, "{crate_name}::model::User");
```

Useful path tree helpers:

| Helper | Use |
|--------|-----|
| `pitem!(ptree, path)` | Look up an item by fully-qualified path |
| `pid!(ptree, path)` | Get the `PathId` for a path |
| `pnode!(ptree, path)` | Get the node for a path |
| `ptree.get_owned(type_id)` | Convert an internal `TypeId` into an owned, printable type |

### Types

Many items expose a `TypeId`. Import `GetOwned` to turn that id into a value you
can print or compare.

```rust,ignore
use syn_sem::{pitem, GetOwned};

let local = pitem!(ptree, "{crate_name}::demo::use_values::{{0}}::output")
    .as_local()
    .unwrap();
let ty = ptree.get_owned(local.type_id());

assert_eq!(ty.to_string(), "i32");
```

### Constants

Evaluated compile-time constants are available through `sem.evaluated`.

```rust,ignore
use syn_sem::{pid, value::{Scalar, Value}};

let const_id = pid!(ptree, "{crate_name}::demo::A");
let value = analyzed
    .sem
    .evaluated
    .get_mapped_value_by_path_id(const_id)
    .unwrap();

assert!(matches!(value, Value::Scalar(Scalar::I32(5))));
```

## Physical Files

You can also analyze real files. The entry path is resolved relative to the
current working directory, and `mod` declarations are followed automatically.

```rust,no_run
use syn_sem::{pitem, AnalysisSession};

let analyzed = AnalysisSession::default()
    .run(|analyzer| analyzer.analyze("src/lib.rs"))
    .unwrap();

let ptree = &analyzed.sem.ptree;
let crate_name = ptree.crate_name();
let item = pitem!(ptree, "{crate_name}::my_module::MyStruct");

assert!(item.as_struct().is_some());
```

## Supported Rust Subset

### Items

- Modules (`mod`, inline and file-based)
- Functions (`fn`) with generics and trait bounds
- Structs with fields and generics
- Traits with associated types and const generics
- Type aliases
- Constants and associated constants
- Enums with variants
- `use` statements
- `impl` blocks

### Types

- Scalar primitives
- Named path types
- Tuples
- Arrays with fixed or generic lengths
- References (`&T`, `&mut T`)
- Unit type `()`

## API Reference

### Entry Points

| Type | Description |
|------|-------------|
| `AnalysisSession` | Configures and runs semantic analysis |
| `Analyzer` | Builder passed to the session closure; use it to register files and trigger analysis |
| `Semantics` | Analysis results |
| `Config` / `ConfigLoad` | Controls whether built-in `core` and `std` sources are loaded |

### Analysis Results

| Field | Type | Description |
|-------|------|-------------|
| `sem.ptree` | `PathTree` | Hierarchical namespace tree of analyzed items |
| `sem.stree` | `SyntaxTree` | Parsed syntax tree |
| `sem.evaluated` | `Evaluated` | Evaluated compile-time constants |
| `sem.logic` | `Logic` | Trait resolution logic engine |

### Re-exported Item Types

Available under `syn_sem::item`:

`Block`, `Const`, `Field`, `Fn`, `Local`, `Mod`, `Param`, `Struct`, `Trait`,
`TypeAlias`, `Use`

### Re-exported Value Types

Available under `syn_sem::value`:

`ConstGeneric`, `Enum`, `Field`, `Fn`, `Scalar`, `Value`

### Type System

| Type | Description |
|------|-------------|
| `Type` | Enum of all type forms |
| `TypeScalar` | Scalar primitive types |
| `TypePath` | Named path types |
| `TypeArray` | Array types with `ArrayLen` |
| `TypeRef` / `TypeMut` | Reference and mutable reference types |
| `TypeTuple` | Tuple types |
| `TypeId` | Unique identifier for a type |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE.txt) or
[MIT License](LICENSE-MIT.txt) at your option.
