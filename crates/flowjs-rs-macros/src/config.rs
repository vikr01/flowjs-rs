//! Project-level configuration read from `Cargo.toml` metadata.
//!
//! ```toml
//! [package.metadata.flowjs-rs]
//! opaque_newtypes = true
//! file_extension = "mjs.flow"
//! large_int = "number"
//! ```
//!
//! Workspace-level config is also supported:
//! ```toml
//! [workspace.metadata.flowjs-rs]
//! opaque_newtypes = true
//! ```
//!
//! Package-level config overrides workspace-level.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Project configuration parsed from Cargo.toml metadata.
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    /// Newtypes automatically become opaque types.
    pub opaque_newtypes: bool,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            opaque_newtypes: false,
        }
    }
}

/// Cached config — parsed once per compilation.
static CONFIG: OnceLock<ProjectConfig> = OnceLock::new();

/// Get the project config, parsing from Cargo.toml on first access.
pub fn project_config() -> &'static ProjectConfig {
    CONFIG.get_or_init(|| load_config().unwrap_or_default())
}

fn load_config() -> Option<ProjectConfig> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let manifest_path = PathBuf::from(&manifest_dir).join("Cargo.toml");

    // Try package-level first
    if let Some(cfg) = read_config_from(&manifest_path) {
        return Some(cfg);
    }

    // Walk up to find workspace root
    let mut dir = PathBuf::from(&manifest_dir);
    while dir.pop() {
        let workspace_toml = dir.join("Cargo.toml");
        if workspace_toml.exists() {
            if let Some(cfg) = read_workspace_config_from(&workspace_toml) {
                return Some(cfg);
            }
        }
    }

    None
}

fn read_config_from(path: &Path) -> Option<ProjectConfig> {
    let content = std::fs::read_to_string(path).ok()?;
    let doc: toml::Value = content.parse().ok()?;
    let meta = doc.get("package")?.get("metadata")?.get("flowjs-rs")?;
    parse_config_table(meta)
}

fn read_workspace_config_from(path: &Path) -> Option<ProjectConfig> {
    let content = std::fs::read_to_string(path).ok()?;
    let doc: toml::Value = content.parse().ok()?;
    let meta = doc.get("workspace")?.get("metadata")?.get("flowjs-rs")?;
    parse_config_table(meta)
}

fn parse_config_table(table: &toml::Value) -> Option<ProjectConfig> {
    let mut cfg = ProjectConfig::default();

    if let Some(v) = table.get("opaque_newtypes").and_then(|v| v.as_bool()) {
        cfg.opaque_newtypes = v;
    }

    Some(cfg)
}
