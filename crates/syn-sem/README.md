# syn-sem

An experimental semantic analyzer for a subset of Rust.

`syn-sem` builds on the [`syn`](https://github.com/dtolnay/syn) crate to
perform deep semantic analysis of Rust source code, including type inference,
constant evaluation, trait resolution, and monomorphization of generic code.

![syn-sem overview](docs/syn-sem-diagram.svg)

## Status

This is an early-stage experimental library (`0.0.1`). The API is unstable and
subject to significant change.

## Features

- **Path tree construction** — builds a hierarchical namespace tree
  representing the full module structure of a crate
- **Type inference** — infers types of expressions across items
- **Constant evaluation** — evaluates compile-time constants and const generics
- **Trait resolution** — resolves trait implementations using logic-based
  reasoning
- **Monomorphization** — instantiates generic code with concrete type arguments
- **Module resolution** — resolves `mod` and `use` statements across files
- **Virtual files** — analyze in-memory source code without writing to disk

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
- `impl` blocks (with monomorphization support)

### Types
- Scalar primitives
- Named path types
- Tuples
- Arrays (fixed and generic length)
- References (`&T`, `&mut T`)
- Unit type `()`

## Usage

Add `syn-sem` to your `Cargo.toml`:

```toml
[dependencies]
syn-sem = "0.0.1"
```

### Analyzing physical files

Physical-file analysis reads `.rs` files from disk starting at an entry path
relative to the current working directory. All `mod` declarations are followed
automatically and their corresponding files are loaded.

```rust,no_run
use syn_sem::{pitem, AnalysisSession};

// "src/lib.rs" is resolved relative to the current working directory.
// The crate name is derived from the entry file name:
// e.g. "src/lib.rs" → "lib", "src/main.rs" → "main".
let analyzed = AnalysisSession::default()
    .run(|analyzer| analyzer.analyze("src/lib.rs"))
    .unwrap();

let ptree = &analyzed.sem.ptree;
let crate_name = ptree.crate_name();

// Items are looked up by their fully-qualified path.
let item = pitem!(ptree, "{crate_name}::my_module::MyStruct");
let _struct_item = item.as_struct().unwrap();
```

### Analyzing virtual (in-memory) files

Virtual files let you analyze Rust code without writing to disk. Pass source
code as strings — the registered paths must match how the entry module resolves
`mod` declarations (e.g. `mod utils` in `"lib.rs"` resolves to `"utils.rs"`).

```rust
use syn_sem::{pitem, AnalysisSession};

let analyzed = AnalysisSession::default()
    .run(|mut analyzer| {
        // Entry module declares a submodule.
        analyzer.add_virtual_file("lib.rs", "pub mod utils;");

        // Submodule file — path matches the `mod utils` declaration in lib.rs.
        analyzer.add_virtual_file("utils.rs", "
            pub struct Config {
                pub value: u32,
            }

            pub fn default_config() -> Config {
                Config { value: 42 }
            }
        ");

        analyzer.analyze("lib.rs")
    })
    .unwrap();

let ptree = &analyzed.sem.ptree;
let crate_name = ptree.crate_name();

// Struct item
let item = pitem!(ptree, "{crate_name}::utils::Config");
assert!(item.as_struct().is_some());

// Function item
let item = pitem!(ptree, "{crate_name}::utils::default_config");
assert!(item.as_fn().is_some());
```

## API Overview

### Entry point

| Type | Description |
|------|-------------|
| `AnalysisSession` | Configures and runs semantic analysis |
| `Analyzer` | Builder passed to the session closure; use it to register files and trigger analysis |
| `Semantics` | Analysis results |
| `Config` / `ConfigLoad` | Controls which standard libraries are loaded (`core`, `std`) |

### Analysis results

| Field | Type | Description |
|-------|------|-------------|
| `sem.ptree` | `PathTree` | Hierarchical namespace tree of all items |
| `sem.stree` | `SyntaxTree` | Original parsed syntax tree |
| `sem.evaluated` | `Evaluated` | Evaluated compile-time constants |
| `sem.logic` | `Logic` | Trait resolution logic engine |

### Path tree macros

| Macro | Description |
|-------|-------------|
| `pitem!(ptree, path)` | Look up an item by path, panic if not found |
| `pid!(ptree, path)` | Get the `PathId` for a path |
| `pnode!(ptree, path)` | Get the `NodeIndex` for a path |

### Item types (`syn_sem::item`)

`Block`, `Const`, `Field`, `Fn`, `Local`, `Mod`, `Param`, `Struct`, `Trait`, `TypeAlias`, `Use`

### Value types (`syn_sem::value`)

`ConstGeneric`, `Enum`, `Field`, `Fn`, `Scalar`, `Value`

### Type system

| Type | Description |
|------|-------------|
| `Type` | Enum of all type forms |
| `TypeScalar` | Scalar primitive types |
| `TypePath` | Named path types |
| `TypeArray` | Array types with `ArrayLen` (fixed or generic) |
| `TypeRef` / `TypeMut` | Reference types |
| `TypeTuple` | Tuple types |
| `TypeId` | Unique identifier for a type |

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT License](LICENSE-MIT) at your option.
