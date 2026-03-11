# flow-parser

> Typed Rust bindings for Facebook's [Flow](https://flow.org/) parser

Embeds the official Flow parser via [QuickJS](https://crates.io/crates/quick-js) and deserializes the AST into typed Rust structs. No Node.js or npm required — the parser JS is fetched from the npm registry at build time.

See the [repository](https://github.com/unstabler/flowjs-rs) for documentation or the [issues](https://github.com/unstabler/flowjs-rs/issues) tracker.

## Install

```sh
cargo add flow-parser
```
