# Agents

Instructions for LLM-based coding agents working on this repository.

## Project

- flowjs-rs: derive macro that generates Flow type declarations from Rust types
- Same interface as ts-rs but outputs Flow syntax
- Two crates: `flowjs-rs` (library) and `flowjs-rs-macros` (proc-macro)

## Architecture

- `flowjs-rs-macros` — proc-macro crate, `#[derive(Flow)]`
  - `attr.rs` — parses `#[flow(...)]` and `#[serde(...)]` attributes
  - `lib.rs` — derive logic for structs and enums
- `flowjs-rs` — main library crate
  - `lib.rs` — `Flow` trait, `Config`, `Dummy`, `ExportError`
  - `impls.rs` — built-in `impl Flow for T` (primitives, collections, etc.)
  - `export.rs` — writes `.js.flow` files to disk

## Key differences from ts-rs

- Output is Flow, not TypeScript
- Objects use exact syntax: `{| +field: type |}`
- Fields are covariant (`+` prefix) by default
- `$ReadOnlyArray<T>` instead of `readonly T[]`
- `?T` instead of `T | null`
- `mixed` instead of `any`
- `void` instead of `null` for unit types
- `opaque type` support (no TypeScript equivalent)
- Export files are `.js.flow`, not `.ts`

## Rules

- No `unsafe`
- No nightly features
- `cargo fmt && cargo clippy && cargo test` must pass
- Integration tests go in `flowjs-rs/tests/derive.rs`
- New attributes: parse in `attr.rs`, handle in `lib.rs`, test in `derive.rs`
