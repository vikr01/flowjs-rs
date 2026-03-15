//! Shared export infrastructure for type declaration generators.
//!
//! Provides the core traits and utilities that both flowjs-rs and ts-rs
//! (and any future type-declaration-from-Rust project) need:
//!
//! - `TypeVisitor` trait for dependency graph walking
//! - `Dependency` struct for import generation
//! - `ExportConfig` for configuring output paths and extensions
//! - `ExportError` for error handling
//! - File export with thread-safe locking and idempotent writes
//! - Relative import path calculation

use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};

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

/// A visitor used to iterate over all dependencies or generics of a type.
pub trait TypeVisitor: Sized {
    fn visit<T: ExportableType + 'static + ?Sized>(&mut self);
}

/// Core trait that all exportable types implement.
///
/// This is the type-system-agnostic base. Language-specific traits
/// (Flow's `Flow`, ts-rs's `TS`) extend this with syntax-specific methods.
pub trait ExportableType {
    /// Type name (may include generic parameters).
    fn type_name(cfg: &ExportConfig) -> String;

    /// Identifier without generic parameters.
    fn type_ident(cfg: &ExportConfig) -> String {
        let name = Self::type_name(cfg);
        match name.find('<') {
            Some(i) => name[..i].to_owned(),
            None => name,
        }
    }

    /// Output file path (relative to export dir), without extension.
    fn output_path() -> Option<PathBuf> {
        None
    }

    /// Visit all direct dependencies.
    fn visit_dependencies(_: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
    }

    /// Visit all generic type parameters.
    fn visit_generics(_: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
    }
}

/// A type dependency for import generation.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Dependency {
    pub type_id: TypeId,
    pub name: String,
    pub output_path: PathBuf,
}

/// Configuration for type export.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    pub export_dir: PathBuf,
    pub file_extension: String,
    pub array_tuple_limit: usize,
    pub large_int_type: String,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            export_dir: PathBuf::from("./bindings"),
            file_extension: "js.flow".to_owned(),
            array_tuple_limit: 64,
            large_int_type: "bigint".to_owned(),
        }
    }
}

impl ExportConfig {
    /// Resolve a base path (without extension) to a full path with extension.
    pub fn resolve_path(&self, base: &Path) -> PathBuf {
        if base.extension().is_some() {
            base.to_owned()
        } else {
            let name = base.to_str().unwrap_or("unknown");
            PathBuf::from(format!("{name}.{}", self.file_extension))
        }
    }
}

/// Compute a relative path from `base` to `target`.
pub fn diff_paths(target: &Path, base: &Path) -> PathBuf {
    let target_components: Vec<Component<'_>> = target.components().collect();
    let base_components: Vec<Component<'_>> = base.components().collect();

    let common = target_components
        .iter()
        .zip(base_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let mut result = PathBuf::new();
    for _ in common..base_components.len() {
        result.push("..");
    }
    for component in &target_components[common..] {
        result.push(component);
    }

    if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    }
}

/// Normalize Windows backslashes to forward slashes for JS import paths.
pub fn normalize_separators(s: &str) -> String {
    s.replace('\\', "/")
}

/// Compute a relative import path from one file to another.
pub fn relative_import_path(from: &Path, to: &Path) -> String {
    let from_dir = from.parent().unwrap_or(Path::new(""));
    let rel = diff_paths(to, from_dir);
    let s = normalize_separators(rel.to_str().unwrap_or("./unknown"));

    if s.starts_with("../") || s.starts_with("./") {
        s
    } else {
        format!("./{s}")
    }
}

/// Per-file mutex for thread-safe exports.
pub fn file_lock(path: &Path) -> &'static Mutex<()> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, &'static Mutex<()>>>> = OnceLock::new();
    let locks = LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = locks.lock().unwrap_or_else(|e| e.into_inner());
    let canonical = path.to_path_buf();
    map.entry(canonical)
        .or_insert_with(|| Box::leak(Box::new(Mutex::new(()))))
}

/// Write content to a file, with idempotent marker checking.
///
/// If the file already contains a marker string, the write is skipped.
/// Thread-safe via per-file locking.
pub fn write_with_lock(
    path: &Path,
    marker: &str,
    content_fn: impl FnOnce() -> String,
    append_fn: Option<impl FnOnce(&str) -> String>,
) -> Result<(), ExportError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let _guard = file_lock(path)
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    if path.exists() {
        let existing = std::fs::read_to_string(path)?;
        if existing.contains(marker) {
            return Ok(());
        }
        if let Some(append) = append_fn {
            std::fs::write(path, append(&existing))?;
        }
    } else {
        std::fs::write(path, content_fn())?;
    }

    Ok(())
}

/// Recursively export a type and all its dependencies.
pub fn export_recursive<T: ExportableType + 'static + ?Sized>(
    cfg: &ExportConfig,
    seen: &mut HashSet<TypeId>,
    export_one: &dyn Fn(&ExportConfig, &Path) -> Result<(), ExportError>,
) -> Result<(), ExportError> {
    if !seen.insert(TypeId::of::<T>()) {
        return Ok(());
    }

    struct Visit<'a> {
        cfg: &'a ExportConfig,
        seen: &'a mut HashSet<TypeId>,
        export_one: &'a dyn Fn(&ExportConfig, &Path) -> Result<(), ExportError>,
        error: Option<ExportError>,
    }

    impl TypeVisitor for Visit<'_> {
        fn visit<U: ExportableType + 'static + ?Sized>(&mut self) {
            if self.error.is_some() || U::output_path().is_none() {
                return;
            }
            self.error =
                export_recursive::<U>(self.cfg, self.seen, self.export_one).err();
        }
    }

    let mut visitor = Visit {
        cfg,
        seen,
        export_one,
        error: None,
    };
    T::visit_dependencies(&mut visitor);

    if let Some(e) = visitor.error {
        return Err(e);
    }

    let base = T::output_path()
        .ok_or(ExportError::CannotBeExported(std::any::type_name::<T>()))?;
    let relative = cfg.resolve_path(&base);
    let path = cfg.export_dir.join(relative);
    export_one(cfg, &path)
}
