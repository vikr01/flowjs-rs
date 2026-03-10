# Contributing to flowjs-rs

## Getting started

```sh
git clone https://github.com/unstabler/flowjs-rs
cd flowjs-rs
cargo test
```

## Development

- Rust stable (MSRV 1.78.0)
- `cargo fmt --all` before committing
- `cargo clippy --all-targets` must pass with zero warnings
- `cargo test` must pass

## Structure

```
flowjs-rs/          workspace root
├── flowjs-rs/      main crate (Flow trait, built-in impls, export)
│   ├── src/
│   │   ├── lib.rs      Flow trait + Config + Dummy
│   │   ├── impls.rs    Built-in impls for primitives, Vec, Option, etc.
│   │   └── export.rs   File export logic
│   └── tests/
│       └── derive.rs   Integration tests
└── flowjs-rs-macros/   proc-macro crate
    └── src/
        ├── lib.rs      #[derive(Flow)] entry point
        └── attr.rs     Attribute parsing (#[flow(...)], #[serde(...)])
```

## Adding a built-in impl

1. Add `impl Flow for YourType` in `flowjs-rs/src/impls.rs`
2. If behind a feature gate, use `#[cfg(feature = "your-feature")]`
3. Add the feature to `flowjs-rs/Cargo.toml`
4. Add a test

## Adding a new attribute

1. Parse it in `flowjs-rs-macros/src/attr.rs`
2. Handle it in the derive logic in `flowjs-rs-macros/src/lib.rs`
3. Add integration tests in `flowjs-rs/tests/derive.rs`

## Flow syntax reference

- Exact objects: `{| +field: type |}`
- Covariant (readonly) fields: `+field`
- Opaque types: `opaque type Name` or `opaque type Name: Bound`
- Nullable: `?Type`
- Read-only arrays: `$ReadOnlyArray<T>`
- Mixed (any): `mixed`
- Void (unit): `void`

## Pull requests

- One feature/fix per PR
- Include tests
- Run `cargo fmt && cargo clippy && cargo test` before pushing
