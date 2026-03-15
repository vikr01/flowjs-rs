//! Project-level configuration, delegating to `derive-project-config`.

use std::sync::OnceLock;

const TOOL_NAME: &str = "flowjs-rs";

#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub opaque_newtypes: bool,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            opaque_newtypes: false,
        }
    }
}

static CONFIG: OnceLock<ProjectConfig> = OnceLock::new();

pub fn project_config() -> &'static ProjectConfig {
    CONFIG.get_or_init(|| {
        let mut cfg = ProjectConfig::default();
        if let Some(v) = derive_project_config::read_bool(TOOL_NAME, "opaque_newtypes") {
            cfg.opaque_newtypes = v;
        }
        cfg
    })
}
