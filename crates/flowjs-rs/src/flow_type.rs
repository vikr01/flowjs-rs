//! Flow built-in type names.
//!
//! Every primitive and special type in Flow's type system is defined here as a
//! constant. No code in this crate should use raw string literals for Flow
//! type names — always reference these constants instead.

/// `string` — text values.
pub const STRING: &str = "string";

/// `number` — numeric values (integer and floating-point).
pub const NUMBER: &str = "number";

/// `boolean` — true/false.
pub const BOOLEAN: &str = "boolean";

/// `void` — absence of a value (unit type).
pub const VOID: &str = "void";

/// `null` — explicit null.
pub const NULL: &str = "null";

/// `mixed` — the top type (any value, unknown shape).
pub const MIXED: &str = "mixed";

/// `any` — opt-out of type checking.
pub const ANY: &str = "any";

/// `empty` — the bottom type (no value inhabits this type).
pub const EMPTY: &str = "empty";

/// `bigint` — arbitrary-precision integers.
pub const BIGINT: &str = "bigint";

/// `symbol` — unique symbols.
pub const SYMBOL: &str = "symbol";

/// `$ReadOnlyArray` — immutable array generic.
pub const READ_ONLY_ARRAY: &str = "$ReadOnlyArray";
