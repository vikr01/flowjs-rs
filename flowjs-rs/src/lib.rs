//! Generate Flow type declarations from Rust types.
//!
//! # Usage
//! ```rust
//! #[derive(flowjs_rs::Flow)]
//! struct User {
//!     user_id: i32,
//!     first_name: String,
//!     last_name: String,
//! }
//! ```
//!
//! When running `cargo test`, the following Flow type will be exported:
//! ```flow
//! type User = {|
//!   +user_id: number,
//!   +first_name: string,
//!   +last_name: string,
//! |};
//! ```

mod export;
mod impls;

pub use flowjs_rs_macros::Flow;

use std::path::{Path, PathBuf};

/// Export error.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("type `{0}` cannot be exported")]
    CannotBeExported(&'static str),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Fmt(#[from] std::fmt::Error),
}

/// Configuration for Flow type generation.
#[derive(Debug, Clone)]
pub struct Config {
    export_dir: PathBuf,
}

impl Config {
    /// Create a new config with default settings.
    pub fn new() -> Self {
        Self {
            export_dir: PathBuf::from("./bindings"),
        }
    }

    /// Read config from environment variables.
    ///
    /// | Variable | Default |
    /// |---|---|
    /// | `FLOW_RS_EXPORT_DIR` | `./bindings` |
    pub fn from_env() -> Self {
        let export_dir = std::env::var("FLOW_RS_EXPORT_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./bindings"));
        Self { export_dir }
    }

    /// Set the export directory.
    pub fn with_out_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.export_dir = dir.into();
        self
    }

    /// Return the export directory.
    pub fn out_dir(&self) -> &Path {
        &self.export_dir
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

/// The core trait. Derive it on your types to generate Flow declarations.
///
/// Mirrors the ts-rs `TS` trait interface.
pub trait Flow {
    /// Whether this is an enum type.
    const IS_ENUM: bool = false;

    /// Flow type name.
    fn name(cfg: &Config) -> String;

    /// Inline Flow type definition (the right-hand side of `type X = ...`).
    fn inline(cfg: &Config) -> String;

    /// Full Flow type declaration (e.g. `type User = {| +id: number |}`).
    fn decl(cfg: &Config) -> String {
        format!("type {} = {};", Self::name(cfg), Self::inline(cfg))
    }

    /// JSDoc/Flow comment for this type.
    fn docs() -> Option<String> {
        None
    }

    /// Output file path relative to the export directory.
    fn output_path() -> Option<PathBuf> {
        None
    }

    /// Export this type to disk.
    fn export(cfg: &Config) -> Result<(), ExportError>
    where
        Self: 'static,
    {
        let relative = Self::output_path()
            .ok_or(ExportError::CannotBeExported(std::any::type_name::<Self>()))?;
        let path = cfg.export_dir.join(relative);
        export::export_to::<Self>(cfg, &path)
    }
}

/// Dummy type used as a placeholder for generic parameters during codegen.
pub struct Dummy;

impl Flow for Dummy {
    fn name(_: &Config) -> String {
        "any".to_owned()
    }
    fn inline(_: &Config) -> String {
        "any".to_owned()
    }
}
