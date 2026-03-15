# flowjs-rs

Generate Flow type declarations from Rust types.

### Why?

If you're building a web application with a Rust backend and a Flow-typed JavaScript frontend, your data structures need to stay in sync on both sides. flowjs-rs lets you derive Flow type declarations directly from your Rust structs and enums, keeping your types in one place.

It works for REST APIs, WebAssembly, or anywhere Rust needs to talk to Flow-typed JavaScript.

Every generated declaration is validated against Facebook's actual Flow parser — not string matching, not heuristics. If the output isn't valid Flow, the tests fail.

### How?

flowjs-rs provides a single trait, `Flow`. Derive it on your types, then export the bindings during `cargo test`.

### Get started

```toml
[dependencies]
flowjs-rs = "0.0.0-alpha.1"
```

```rust
#[derive(flowjs_rs::Flow)]
#[flow(export)]
struct User {
    user_id: i32,
    first_name: String,
    last_name: String,
}
```

Running `cargo test` exports the following to `bindings/User.js.flow`:

```js
// @flow
export type User = {| user_id: number, first_name: string, last_name: string |};
```

### Features

- Derive Flow types from Rust structs and enums
- Tagged, untagged, and adjacently tagged enum unions
- Generic type support
- Serde attribute compatibility out of the box
- Automatic import generation across files
- Override any type with `#[flow(type = "..")]` or `#[flow(as = "..")]`
- Native Flow features: opaque types, Flow enums, exact objects
- Parser-validated output — declarations checked by the real Flow parser in tests
- 209 tests, 68% code coverage, CI with Flow type-checking

### Works alongside ts-rs

flowjs-rs shares the same attribute API as [ts-rs](https://github.com/Aleph-Alpha/ts-rs). If you already derive `TS`, add `Flow` next to it — same serde attributes, same behavior, Flow output instead of TypeScript.

```rust
use flowjs_rs::Flow;
use ts_rs::TS;

#[derive(Flow, TS)]
#[serde(rename_all = "camelCase")]
struct UserProfile {
    user_id: i32,
    display_name: String,
}
// Generates both UserProfile.ts and UserProfile.js.flow
```

### Configuration

Project-level settings go in your `Cargo.toml`:

```toml
[package.metadata.flowjs-rs]
opaque_newtypes = true  # newtypes automatically become opaque types
```

Runtime configuration uses environment variables:

| Variable | Description | Default |
|---|---|---|
| `FLOW_RS_EXPORT_DIR` | Directory for exported files | `./bindings` |
| `FLOW_RS_FILE_EXTENSION` | File extension for exports | `js.flow` |
| `FLOW_RS_LARGE_INT` | Flow type for `i64`, `u64`, `i128`, `u128` | `bigint` |

Persistent env config via `.cargo/config.toml`:
```toml
[env]
FLOW_RS_EXPORT_DIR = { value = "flow-bindings", relative = true }
FLOW_RS_FILE_EXTENSION = "mjs.flow"
```

### Flow Enums

Unit-variant Rust enums can produce native Flow `enum` declarations:

```rust
#[derive(Flow)]
#[flow(flow_enum = "string", rename_all = "lowercase")]
enum Status {
    Active,
    Paused,
    Disabled,
}
```

```js
export enum Status of string {
  Active = 'active',
  Paused = 'paused',
  Disabled = 'disabled',
}
```

Four representations: `symbol`, `string`, `number`, `boolean`. The ts-rs `repr(enum)` syntax also works.

### Opaque Types

Newtypes can generate Flow opaque types — consumers see the type name but can't construct or destructure values:

```rust
#[derive(Flow)]
#[flow(opaque)]
struct SessionToken(String);

#[derive(Flow)]
#[flow(opaque)]
struct Wrapper<T: Flow> {
    thing: T,
}
```

```js
export opaque type SessionToken = string;
export opaque type Wrapper<T> = {| thing: T |};
```

The defining module can access the body. Consumers cannot. Set `opaque_newtypes = true` in `Cargo.toml` metadata to make all newtypes opaque by default.

### Serde Compatibility

With the `serde-compat` feature (on by default), serde attributes are parsed automatically.

Supported: `rename`, `rename_all`, `rename_all_fields`, `tag`, `content`, `untagged`, `skip`, `skip_serializing`, `skip_serializing_if`, `flatten`, `default`

### Cargo Features

| Feature | Description |
|---|---|
| serde-compat | **On by default.** Parse serde attributes. |
| no-serde-warnings | Silence warnings for unsupported serde attributes. |
| serde-json-impl | `Flow` impl for types from *serde_json* |
| chrono-impl | `Flow` impl for types from *chrono* |
| uuid-impl | `Flow` impl for types from *uuid* |
| url-impl | `Flow` impl for types from *url* |

### MSRV

Minimum Supported Rust Version: 1.78.0

### Contributing

Contributions welcome. Open an issue or PR.

License: MIT
