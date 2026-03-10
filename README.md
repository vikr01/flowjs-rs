# flowjs-rs

Generate [Flow](https://flow.org/) type declarations from Rust types.

Same interface as [ts-rs](https://github.com/Aleph-Alpha/ts-rs), different output. If you know ts-rs, you know flowjs-rs.

## Usage

```toml
[dependencies]
flowjs-rs = "0.1"
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

Running `cargo test` exports `bindings/User.js.flow`:

```js
// @flow
export type User = {|
  +user_id: number,
  +first_name: string,
  +last_name: string,
|};
```

## Features

- Derive macro generates Flow types from Rust structs and enums
- Exact objects (`{| |}`) with covariant (`+`) readonly fields
- `opaque type` support for newtypes and sealed return types
- Serde compatibility (reads `#[serde(...)]` attributes by default)
- Same attribute API as ts-rs: `rename`, `rename_all`, `skip`, `tag`, `content`, `untagged`, etc.
- Built-in impls for all Rust primitives, `Vec`, `Option`, `HashMap`, tuples, etc.

## Opaque types

Flow's `opaque type` prevents external construction — the type can only be obtained from your API:

```rust
#[derive(flowjs_rs::Flow)]
#[flow(opaque)]
struct SessionToken(String);

// → opaque type SessionToken;
```

With a supertype bound (readable as string, not constructable from string):

```rust
#[derive(flowjs_rs::Flow)]
#[flow(opaque = "string")]
struct TagName(String);

// → opaque type TagName: string;
```

## Flow ↔ TypeScript differences

| Concept | TypeScript | Flow |
|---|---|---|
| Exact objects | `{ ... }` (structural) | `{| ... |}` (exact) |
| Readonly fields | `readonly field` | `+field` |
| Readonly arrays | `readonly T[]` | `$ReadOnlyArray<T>` |
| Nullable | `T \| null` | `?T` |
| Opaque types | N/A | `opaque type T` |
| Any | `any` | `mixed` |

## Configuration

| Environment variable | Default | Description |
|---|---|---|
| `FLOW_RS_EXPORT_DIR` | `./bindings` | Export directory for generated `.js.flow` files |

## Attributes

### Container (`#[flow(...)]`)

| Attribute | Description |
|---|---|
| `rename = "..."` | Override Flow type name |
| `rename_all = "..."` | Rename all fields (camelCase, snake_case, etc.) |
| `export` | Generate export test |
| `export_to = "..."` | Custom export path |
| `opaque` | Fully opaque type |
| `opaque = "bound"` | Opaque with supertype bound |
| `tag = "..."` | Tagged enum discriminant field |
| `content = "..."` | Adjacent tag content field |
| `untagged` | Untagged enum (union) |
| `crate = "..."` | Override crate path |

### Field (`#[flow(...)]`)

| Attribute | Description |
|---|---|
| `rename = "..."` | Rename field |
| `type = "..."` | Override Flow type |
| `skip` | Omit field |
| `optional` | Mark as optional |
| `inline` | Inline type definition |
| `flatten` | Flatten nested fields |

## License

MIT
