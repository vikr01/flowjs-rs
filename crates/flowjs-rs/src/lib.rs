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
pub mod flow_type;
mod impls;

pub use flowjs_rs_macros::Flow;

pub use crate::export::ExportError;

use std::any::TypeId;
use std::path::{Path, PathBuf};

/// Configuration for Flow type generation.
#[derive(Debug, Clone)]
pub struct Config {
    export_dir: PathBuf,
    array_tuple_limit: usize,
}

impl Config {
    /// Create a new config with default settings.
    pub fn new() -> Self {
        Self {
            export_dir: PathBuf::from("./bindings"),
            array_tuple_limit: 64,
        }
    }

    /// Read config from environment variables.
    ///
    /// | Variable | Default |
    /// |---|---|
    /// | `FLOW_RS_EXPORT_DIR` | `./bindings` |
    pub fn from_env() -> Self {
        let mut cfg = Self::new();

        if let Ok(dir) = std::env::var("FLOW_RS_EXPORT_DIR") {
            cfg = cfg.with_out_dir(dir);
        }

        cfg
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

    /// Set the maximum size of arrays up to which they are treated as Flow tuples.
    /// Arrays beyond this size will instead result in a `$ReadOnlyArray<T>`.
    ///
    /// Default: `64`
    pub fn with_array_tuple_limit(mut self, limit: usize) -> Self {
        self.array_tuple_limit = limit;
        self
    }

    /// Return the maximum size of arrays treated as tuples.
    pub fn array_tuple_limit(&self) -> usize {
        self.array_tuple_limit
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

/// A visitor used to iterate over all dependencies or generics of a type.
/// When an instance of [`TypeVisitor`] is passed to [`Flow::visit_dependencies`] or
/// [`Flow::visit_generics`], the [`TypeVisitor::visit`] method will be invoked for every
/// dependency or generic parameter respectively.
pub trait TypeVisitor: Sized {
    fn visit<T: Flow + 'static + ?Sized>(&mut self);
}

/// A Flow type which is depended upon by other types.
/// This information is required for generating the correct import statements.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Dependency {
    /// Type ID of the rust type.
    pub type_id: TypeId,
    /// Name of the type in Flow.
    pub flow_name: String,
    /// Path to where the type would be exported. By default, a filename is derived from the
    /// type name, which can be customized with `#[flow(export_to = "..")]`.
    /// This path does _not_ include a base directory.
    pub output_path: PathBuf,
}

impl Dependency {
    /// Construct a [`Dependency`] from the given type `T`.
    /// If `T` is not exportable (meaning `T::output_path()` returns `None`), this function
    /// will return `None`.
    pub fn from_ty<T: Flow + 'static + ?Sized>(cfg: &Config) -> Option<Self> {
        let output_path = <T as crate::Flow>::output_path()?;
        Some(Dependency {
            type_id: TypeId::of::<T>(),
            flow_name: <T as crate::Flow>::ident(cfg),
            output_path,
        })
    }
}

/// The core trait. Derive it on your types to generate Flow declarations.
///
/// Mirrors the ts-rs `TS` trait interface.
pub trait Flow {
    /// If this type does not have generic parameters, then `WithoutGenerics` should be `Self`.
    /// If the type does have generic parameters, then all generic parameters must be replaced
    /// with a dummy type, e.g `flowjs_rs::Dummy` or `()`.
    /// The only requirement for these dummy types is that `output_path()` must return `None`.
    type WithoutGenerics: Flow + ?Sized;

    /// If the implementing type is `std::option::Option<T>`, then this associated type is set
    /// to `T`. All other implementations of `Flow` should set this type to `Self` instead.
    type OptionInnerType: ?Sized;

    #[doc(hidden)]
    const IS_OPTION: bool = false;

    /// Whether this is an enum type.
    const IS_ENUM: bool = false;

    /// JSDoc/Flow comment to describe this type -- when `Flow` is derived, docs are
    /// automatically read from your doc comments or `#[doc = ".."]` attributes.
    fn docs() -> Option<String> {
        None
    }

    /// Identifier of this type, excluding generic parameters.
    fn ident(cfg: &Config) -> String {
        let name = <Self as crate::Flow>::name(cfg);
        match name.find('<') {
            Some(i) => name[..i].to_owned(),
            None => name,
        }
    }

    /// Declaration of this type, e.g. `type User = {| +user_id: number |};`.
    /// This function will panic if the type has no declaration.
    ///
    /// If this type is generic, then all provided generic parameters will be swapped for
    /// placeholders, resulting in a generic Flow definition.
    fn decl(cfg: &Config) -> String {
        panic!("{} cannot be declared", Self::name(cfg))
    }

    /// Declaration of this type using the supplied generic arguments.
    /// The resulting Flow definition will not be generic. For that, see `Flow::decl()`.
    /// If this type is not generic, then this function is equivalent to `Flow::decl()`.
    fn decl_concrete(cfg: &Config) -> String {
        panic!("{} cannot be declared", Self::name(cfg))
    }

    /// Flow type name, including generic parameters.
    fn name(cfg: &Config) -> String;

    /// Inline Flow type definition (the right-hand side of `type X = ...`).
    fn inline(cfg: &Config) -> String;

    /// Flatten a type declaration.
    /// This function will panic if the type cannot be flattened.
    fn inline_flattened(cfg: &Config) -> String {
        panic!("{} cannot be flattened", Self::name(cfg))
    }

    /// Iterate over all dependencies of this type.
    fn visit_dependencies(_: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
    }

    /// Iterate over all type parameters of this type.
    fn visit_generics(_: &mut impl TypeVisitor)
    where
        Self: 'static,
    {
    }

    /// Resolve all dependencies of this type recursively.
    fn dependencies(cfg: &Config) -> Vec<Dependency>
    where
        Self: 'static,
    {
        struct Visit<'a>(&'a Config, &'a mut Vec<Dependency>);
        impl TypeVisitor for Visit<'_> {
            fn visit<T: Flow + 'static + ?Sized>(&mut self) {
                let Visit(cfg, deps) = self;
                if let Some(dep) = Dependency::from_ty::<T>(cfg) {
                    deps.push(dep);
                }
            }
        }

        let mut deps: Vec<Dependency> = vec![];
        Self::visit_dependencies(&mut Visit(cfg, &mut deps));
        deps
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

    /// Export this type to disk, together with all of its dependencies.
    fn export_all(cfg: &Config) -> Result<(), ExportError>
    where
        Self: 'static,
    {
        export::export_all_into::<Self>(cfg)
    }

    /// Render this type as a string, returning the full file content.
    fn export_to_string(cfg: &Config) -> Result<String, ExportError>
    where
        Self: 'static,
    {
        export::export_to_string::<Self>(cfg)
    }
}

/// Dummy type used as a placeholder for generic parameters during codegen.
pub struct Dummy;

impl Flow for Dummy {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name(_: &Config) -> String {
        flow_type::ANY.to_owned()
    }
    fn inline(_: &Config) -> String {
        flow_type::ANY.to_owned()
    }
}
