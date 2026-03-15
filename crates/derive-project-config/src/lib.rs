//! Read `[package.metadata.<tool>]` config from `Cargo.toml` for derive macros.
//!
//! Proc macros run at compile time and can read `CARGO_MANIFEST_DIR` to find
//! the project's `Cargo.toml`. This crate provides a generic reader that:
//!
//! 1. Reads `[package.metadata.<tool>]` from the compiling crate's `Cargo.toml`
//! 2. Falls back to `[workspace.metadata.<tool>]` in the workspace root
//! 3. Caches the result for the compilation session
//!
//! ```rust,ignore
//! use derive_project_config::read_metadata;
//!
//! let table = read_metadata("flowjs-rs");
//! if let Some(val) = table.and_then(|t| t.get("opaque_newtypes")) {
//!     // use val
//! }
//! ```

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use toml::Value;

/// Read `[package.metadata.<tool>]` or `[workspace.metadata.<tool>]` from Cargo.toml.
///
/// Returns the parsed TOML table if found, `None` otherwise.
/// Result is cached — subsequent calls with the same tool name return the same value.
///
/// Reads `CARGO_MANIFEST_DIR` to locate the Cargo.toml. If not set
/// (e.g., running outside `cargo`), returns `None`.
pub fn read_metadata(tool: &str) -> Option<&'static Value> {
    // Cache per tool name — in practice, a proc macro only reads one tool's config
    static CACHE: OnceLock<Option<Value>> = OnceLock::new();
    CACHE
        .get_or_init(|| load_metadata(tool))
        .as_ref()
}

fn load_metadata(tool: &str) -> Option<Value> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let manifest_path = PathBuf::from(&manifest_dir).join("Cargo.toml");

    // Try package-level first
    if let Some(val) = read_table(&manifest_path, &["package", "metadata", tool]) {
        return Some(val);
    }

    // Walk up to workspace root
    let mut dir = PathBuf::from(&manifest_dir);
    while dir.pop() {
        let workspace_toml = dir.join("Cargo.toml");
        if workspace_toml.exists() {
            if let Some(val) = read_table(&workspace_toml, &["workspace", "metadata", tool]) {
                return Some(val);
            }
        }
    }

    None
}

fn read_table(path: &Path, keys: &[&str]) -> Option<Value> {
    let content = std::fs::read_to_string(path).ok()?;
    let doc: Value = content.parse().ok()?;

    let mut current = &doc;
    for key in keys {
        current = current.get(key)?;
    }
    Some(current.clone())
}

/// Helper: read a boolean value from metadata.
pub fn read_bool(tool: &str, key: &str) -> Option<bool> {
    read_metadata(tool)?.get(key)?.as_bool()
}

/// Helper: read a string value from metadata.
pub fn read_string(tool: &str, key: &str) -> Option<String> {
    read_metadata(tool)?
        .get(key)?
        .as_str()
        .map(|s| s.to_owned())
}
