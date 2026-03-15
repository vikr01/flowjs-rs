//! Derive macro for flowjs-rs.
//!
//! Generates `Flow` trait implementations from Rust struct and enum definitions,
//! producing Flow type declarations.

#![deny(unused)]

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, spanned::Spanned, Data, DeriveInput, Expr, Fields,
    GenericParam, Generics, Lit, Meta, Path, Result, Type, WherePredicate,
};

mod attr;
mod config;
mod utils;

use attr::{ContainerAttr, FieldAttr, FlowEnumRepr, VariantAttr};

/// Collected dependency types for visit_dependencies generation.
struct Dependencies {
    crate_rename: Path,
    /// Types to visit directly: `v.visit::<T>()`
    types: Vec<Type>,
    /// Types whose dependencies to visit transitively: `<T as Flow>::visit_dependencies(v)`
    transitive: Vec<Type>,
    /// Types whose generics to visit: `<T as Flow>::visit_generics(v)`
    generics: Vec<Type>,
}

impl Dependencies {
    fn new(crate_rename: Path) -> Self {
        Self {
            crate_rename,
            types: Vec::new(),
            transitive: Vec::new(),
            generics: Vec::new(),
        }
    }

    /// Add a field type as a dependency (visit the type + its generics).
    fn push(&mut self, ty: &Type) {
        self.types.push(ty.clone());
        self.generics.push(ty.clone());
    }

    /// Add a field type's transitive deps only (for inline/flatten).
    fn append_from(&mut self, ty: &Type) {
        self.transitive.push(ty.clone());
    }

    fn to_tokens(&self) -> TokenStream {
        let crate_rename = &self.crate_rename;
        let visit_types = self.types.iter().map(|ty| {
            quote! { v.visit::<#ty>(); }
        });
        let visit_transitive = self.transitive.iter().map(|ty| {
            quote! { <#ty as #crate_rename::Flow>::visit_dependencies(v); }
        });
        let visit_generics = self.generics.iter().map(|ty| {
            quote! { <#ty as #crate_rename::Flow>::visit_generics(v); }
        });

        quote! {
            #(#visit_types)*
            #(#visit_generics)*
            #(#visit_transitive)*
        }
    }
}

struct DerivedFlow {
    crate_rename: Path,
    flow_name: Expr,
    docs: Vec<Expr>,
    inline: TokenStream,
    inline_flattened: TokenStream,
    is_enum: TokenStream,
    is_opaque: bool,
    opaque_bound: Option<TokenStream>,
    /// Override decl() for Flow enum declarations.
    enum_decl_override: Option<TokenStream>,
    export: bool,
    export_to: Option<Expr>,
    bound: Option<Vec<WherePredicate>>,
    deps: Dependencies,
    /// `#[flow(concrete(T = i32, ...))]` — replace generic params with concrete types.
    concrete: std::collections::HashMap<Ident, Type>,
}

impl DerivedFlow {
    fn into_impl(self, rust_ty: Ident, generics: Generics) -> TokenStream {
        let export_test = self
            .export
            .then(|| self.generate_export_test(&rust_ty, &generics));

        let output_path_fn = {
            let flow_name = &self.flow_name;
            // output_path returns a base path WITHOUT extension.
            // The extension is added at export time from Config::file_extension().
            // If export_to specifies a full file path (with extension), it's used as-is.
            let path_string = match &self.export_to {
                Some(dir_or_file) => quote! {{
                    let dir_or_file = format!("{}", #dir_or_file);
                    if dir_or_file.ends_with('/') {
                        // Directory — append type name (no extension; Config adds it)
                        format!("{dir_or_file}{}", #flow_name)
                    } else {
                        // Full file path — keep as-is (has extension)
                        format!("{dir_or_file}")
                    }
                }},
                // Default — just the type name (no extension; Config adds it)
                None => quote![format!("{}", #flow_name)],
            };
            quote! {
                fn output_path() -> Option<std::path::PathBuf> {
                    Some(std::path::PathBuf::from(#path_string))
                }
            }
        };

        let crate_rename = &self.crate_rename;
        let flow_name = &self.flow_name;
        let inline = &self.inline;
        let inline_flattened = &self.inline_flattened;
        let is_enum = &self.is_enum;

        let docs_fn = if self.docs.is_empty() {
            quote! { fn docs() -> Option<String> { None } }
        } else {
            let docs = &self.docs;
            quote! {
                fn docs() -> Option<String> {
                    Some([#(#docs),*].join("\n"))
                }
            }
        };

        // Generate name() with generic parameters (excluding concrete ones)
        let name_fn = {
            let generic_names: Vec<_> = generics
                .type_params()
                .filter(|tp| !self.concrete.contains_key(&tp.ident))
                .map(|tp| {
                    let ident = &tp.ident;
                    quote!(<#ident as #crate_rename::Flow>::name(cfg))
                })
                .collect();

            if generic_names.is_empty() {
                quote! {
                    fn name(cfg: &#crate_rename::Config) -> String {
                        #flow_name.to_owned()
                    }
                }
            } else {
                quote! {
                    fn name(cfg: &#crate_rename::Config) -> String {
                        format!("{}<{}>", #flow_name, vec![#(#generic_names),*].join(", "))
                    }
                }
            }
        };

        // Generate decl() — for generic types, create placeholder dummy types
        let decl_fn = if let Some(enum_decl) = &self.enum_decl_override {
            quote! {
                fn decl(cfg: &#crate_rename::Config) -> String {
                    #enum_decl
                }
            }
        } else if self.is_opaque {
            // Opaque types emit the full body (visible in the defining module).
            // `opaque type Foo = body` or `opaque type Foo: bound = body`
            // The `export` prefix is added by format_decl in export.rs.
            let has_generics = generics.type_params().next().is_some();

            if has_generics {
                // Generic opaque: opaque type Foo<T> = {| thing: T |}
                let generic_idents: Vec<_> = generics.type_params().map(|tp| &tp.ident).collect();
                let dummy_decls: Vec<_> = generic_idents
                    .iter()
                    .map(|ident| {
                        let dummy_name = format_ident!("{}Dummy", ident);
                        quote! {
                            struct #dummy_name;
                            impl #crate_rename::Flow for #dummy_name {
                                type WithoutGenerics = Self;
                                type OptionInnerType = Self;
                                fn name(_: &#crate_rename::Config) -> String {
                                    stringify!(#ident).to_owned()
                                }
                                fn inline(cfg: &#crate_rename::Config) -> String {
                                    Self::name(cfg)
                                }
                            }
                        }
                    })
                    .collect();
                let generics_str: Vec<_> = generic_idents
                    .iter()
                    .map(|ident| quote!(stringify!(#ident)))
                    .collect();
                let full_generic_args: Vec<_> = generics
                    .params
                    .iter()
                    .map(|p| match p {
                        GenericParam::Type(tp) => {
                            let dummy_name = format_ident!("{}Dummy", tp.ident);
                            quote!(#dummy_name)
                        }
                        GenericParam::Lifetime(lt) => {
                            let lt = &lt.lifetime;
                            quote!(#lt)
                        }
                        GenericParam::Const(c) => {
                            let ident = &c.ident;
                            quote!(#ident)
                        }
                    })
                    .collect();

                let bound_part = self.opaque_bound
                    .map(|b| quote! { format!(": {}", #b) })
                    .unwrap_or_else(|| quote! { String::new() });

                quote! {
                    fn decl(cfg: &#crate_rename::Config) -> String {
                        #(#dummy_decls)*
                        let inline = <#rust_ty<#(#full_generic_args),*> as #crate_rename::Flow>::inline(cfg);
                        let generics = format!("<{}>", vec![#(#generics_str.to_owned()),*].join(", "));
                        let bound = #bound_part;
                        format!("opaque type {}{generics}{bound} = {inline};", #flow_name)
                    }
                }
            } else {
                let bound_part = self.opaque_bound
                    .map(|b| quote! { format!(": {}", #b) })
                    .unwrap_or_else(|| quote! { String::new() });

                quote! {
                    fn decl(cfg: &#crate_rename::Config) -> String {
                        let inline = Self::inline(cfg);
                        let bound = #bound_part;
                        format!("opaque type {}{bound} = {inline};", #flow_name)
                    }
                }
            }
        } else {
            let has_generics = generics.type_params().next().is_some();
            if has_generics {
                // For generic types: create dummy types, get inline from WithoutGenerics
                // Non-concrete params get dummy structs; concrete params use their concrete type.

                // Generate dummy type declarations only for NON-concrete generic params
                let dummy_decls: Vec<_> = generics
                    .type_params()
                    .filter(|tp| !self.concrete.contains_key(&tp.ident))
                    .map(|tp| {
                        let ident = &tp.ident;
                        let dummy_name = format_ident!("{}Dummy", ident);
                        quote! {
                            struct #dummy_name;
                            impl #crate_rename::Flow for #dummy_name {
                                type WithoutGenerics = Self;
                                type OptionInnerType = Self;
                                fn name(_: &#crate_rename::Config) -> String {
                                    stringify!(#ident).to_owned()
                                }
                                fn inline(cfg: &#crate_rename::Config) -> String {
                                    Self::name(cfg)
                                }
                            }
                        }
                    })
                    .collect();

                // Generics string only includes non-concrete params
                let generics_str: Vec<_> = generics
                    .type_params()
                    .filter(|tp| !self.concrete.contains_key(&tp.ident))
                    .map(|tp| {
                        let ident = &tp.ident;
                        quote!(stringify!(#ident))
                    })
                    .collect();

                // Build full generic args for instantiation:
                // - non-concrete type params → named dummies
                // - concrete type params → the concrete type
                // - lifetime/const params → pass through
                let full_generic_args: Vec<_> = generics
                    .params
                    .iter()
                    .map(|p| match p {
                        GenericParam::Type(tp) => {
                            if let Some(concrete_ty) = self.concrete.get(&tp.ident) {
                                quote!(#concrete_ty)
                            } else {
                                let dummy_name = format_ident!("{}Dummy", tp.ident);
                                quote!(#dummy_name)
                            }
                        }
                        GenericParam::Lifetime(lt) => {
                            let lt = &lt.lifetime;
                            quote!(#lt)
                        }
                        GenericParam::Const(c) => {
                            let ident = &c.ident;
                            quote!(#ident)
                        }
                    })
                    .collect();

                // If all type params are concrete, no generics suffix in the declaration
                let decl_format = if generics_str.is_empty() {
                    quote! {
                        format!("type {} = {inline};", #flow_name)
                    }
                } else {
                    quote! {
                        let generics = format!("<{}>", vec![#(#generics_str.to_owned()),*].join(", "));
                        format!("type {}{generics} = {inline};", #flow_name)
                    }
                };

                quote! {
                    fn decl(cfg: &#crate_rename::Config) -> String {
                        // Named dummies output the type param name (e.g. "T") instead of "any".
                        // Concrete params use the specified concrete type directly.
                        // If the struct has non-Flow trait bounds, use #[flow(bound = "")] to override.
                        #(#dummy_decls)*
                        let inline = <#rust_ty<#(#full_generic_args),*> as #crate_rename::Flow>::inline(cfg);
                        #decl_format
                    }
                }
            } else {
                quote! {
                    fn decl(cfg: &#crate_rename::Config) -> String {
                        format!("type {} = {};", Self::name(cfg), Self::inline(cfg))
                    }
                }
            }
        };

        // decl_concrete: always uses concrete types
        let decl_concrete_fn = if self.is_opaque || self.enum_decl_override.is_some() {
            quote! {
                fn decl_concrete(cfg: &#crate_rename::Config) -> String {
                    Self::decl(cfg)
                }
            }
        } else {
            quote! {
                fn decl_concrete(cfg: &#crate_rename::Config) -> String {
                    format!("type {} = {};", Self::name(cfg), Self::inline(cfg))
                }
            }
        };

        // Build where clause
        let mut bounds = generics.clone();
        if let Some(extra) = &self.bound {
            let where_clause = bounds.make_where_clause();
            for pred in extra {
                where_clause.predicates.push(pred.clone());
            }
        }
        // Add Flow bound for type params that are NOT concrete
        for param in &generics.params {
            if let GenericParam::Type(tp) = param {
                if self.concrete.contains_key(&tp.ident) {
                    continue;
                }
                let ident = &tp.ident;
                let where_clause = bounds.make_where_clause();
                where_clause
                    .predicates
                    .push(parse_quote!(#ident: #crate_rename::Flow));
            }
        }
        let (impl_generics, ty_generics, where_clause) = bounds.split_for_impl();

        // WithoutGenerics: if no generics, Self; otherwise replace type params with Dummy
        // (concrete params use the concrete type instead of Dummy)
        let without_generics = if generics.params.is_empty() {
            quote!(Self)
        } else {
            let params = generics.params.iter().map(|p| match p {
                GenericParam::Type(tp) => {
                    if let Some(concrete_ty) = self.concrete.get(&tp.ident) {
                        quote!(#concrete_ty)
                    } else {
                        quote!(#crate_rename::Dummy)
                    }
                }
                GenericParam::Lifetime(lt) => {
                    let lt = &lt.lifetime;
                    quote!(#lt)
                }
                GenericParam::Const(c) => {
                    let ident = &c.ident;
                    quote!(#ident)
                }
            });
            quote!(#rust_ty<#(#params),*>)
        };

        // visit_dependencies
        let dep_tokens = self.deps.to_tokens();
        let visit_deps_fn = quote! {
            fn visit_dependencies(v: &mut impl #crate_rename::TypeVisitor)
            where
                Self: 'static,
            {
                #dep_tokens
            }
        };

        // visit_generics: iterate type params (skip concrete ones)
        let visit_generics_fn = {
            let generic_visits: Vec<_> = generics
                .type_params()
                .filter(|tp| !self.concrete.contains_key(&tp.ident))
                .map(|tp| {
                    let ident = &tp.ident;
                    quote! {
                        v.visit::<#ident>();
                        <#ident as #crate_rename::Flow>::visit_generics(v);
                    }
                })
                .collect();

            quote! {
                fn visit_generics(v: &mut impl #crate_rename::TypeVisitor)
                where
                    Self: 'static,
                {
                    #(#generic_visits)*
                }
            }
        };

        // inline_flattened
        let inline_flattened_fn = quote! {
            fn inline_flattened(cfg: &#crate_rename::Config) -> String {
                #inline_flattened
            }
        };

        quote! {
            #[automatically_derived]
            impl #impl_generics #crate_rename::Flow for #rust_ty #ty_generics #where_clause {
                type WithoutGenerics = #without_generics;
                type OptionInnerType = Self;

                #name_fn

                fn inline(cfg: &#crate_rename::Config) -> String {
                    #inline
                }

                #inline_flattened_fn
                #decl_fn
                #decl_concrete_fn
                #docs_fn
                #output_path_fn
                #visit_deps_fn
                #visit_generics_fn

                const IS_ENUM: bool = #is_enum;
            }

            #export_test
        }
    }

    fn generate_export_test(&self, rust_ty: &Ident, generics: &Generics) -> TokenStream {
        let crate_rename = &self.crate_rename;
        let test_name = format_ident!("export_flow_bindings_{}", rust_ty);
        let ty = if generics.params.is_empty() {
            quote!(#rust_ty)
        } else {
            let dummies = generics.params.iter().map(|p| match p {
                GenericParam::Type(tp) => {
                    if let Some(concrete_ty) = self.concrete.get(&tp.ident) {
                        quote!(#concrete_ty)
                    } else {
                        quote!(#crate_rename::Dummy)
                    }
                }
                GenericParam::Lifetime(lt) => {
                    let lt = &lt.lifetime;
                    quote!(#lt)
                }
                GenericParam::Const(c) => {
                    let ident = &c.ident;
                    quote!(#ident)
                }
            });
            quote!(#rust_ty<#(#dummies),*>)
        };

        quote! {
            #[cfg(test)]
            #[test]
            #[allow(non_snake_case)]
            fn #test_name() {
                let cfg = #crate_rename::Config::from_env();
                <#ty as #crate_rename::Flow>::export_all(&cfg)
                    .expect("could not export type");
            }
        }
    }
}

/// Derive the `Flow` trait for a struct or enum.
///
/// # Container attributes
/// - `#[flow(rename = "..")]` — Override the Flow type name
/// - `#[flow(rename_all = "..")]` — Rename all fields (camelCase, snake_case, etc.)
/// - `#[flow(export)]` — Generate a test that exports this type to disk
/// - `#[flow(export_to = "..")]` — Custom export path
/// - `#[flow(opaque)]` — Emit as `declare export opaque type Name` (fully opaque)
/// - `#[flow(opaque = "string")]` — Emit as `declare export opaque type Name: string` (bounded)
/// - `#[flow(tag = "..")]` — Tagged enum representation
/// - `#[flow(content = "..")]` — Content field for adjacently tagged enums
/// - `#[flow(untagged)]` — Untagged enum
/// - `#[flow(bound = "..")]` — Additional where clause bounds
///
/// # Field attributes
/// - `#[flow(rename = "..")]` — Rename this field
/// - `#[flow(type = "..")]` — Override field type
/// - `#[flow(skip)]` — Skip this field
/// - `#[flow(optional)]` — Mark as optional
/// - `#[flow(inline)]` — Inline the field type definition
/// - `#[flow(flatten)]` — Flatten nested fields into parent
#[proc_macro_derive(Flow, attributes(flow))]
pub fn derive_flow(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_flow_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_flow_impl(input: DeriveInput) -> Result<TokenStream> {
    let container = ContainerAttr::from_attrs(&input.attrs)?;
    let ident = &input.ident;

    let crate_rename = container
        .crate_rename
        .clone()
        .unwrap_or_else(|| parse_quote!(::flowjs_rs));

    let flow_name: Expr = match &container.rename {
        Some(name) => parse_quote!(#name),
        None => {
            let name = ident.to_string();
            parse_quote!(#name)
        }
    };

    let docs: Vec<Expr> = input
        .attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            if let Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(lit) = &nv.value {
                    if let Lit::Str(s) = &lit.lit {
                        let val = s.value();
                        let trimmed = val.trim();
                        return Some(parse_quote!(#trimmed));
                    }
                }
            }
            None
        })
        .collect();

    let mut deps = Dependencies::new(crate_rename.clone());

    // Container-level type/as override — short-circuits all other codegen
    if let Some(type_str) = &container.type_override {
        let inline = quote!(#type_str.to_owned());
        let derived = DerivedFlow {
            crate_rename: crate_rename.clone(),
            flow_name,
            docs,
            inline: inline.clone(),
            inline_flattened: quote!(format!("({})", #type_str)),
            is_enum: quote!(false),
            is_opaque: false,
            opaque_bound: None,
            enum_decl_override: None,
            export: container.export,
            export_to: container.export_to.clone(),
            bound: container.bound.clone(),
            deps,
            concrete: container.concrete.clone(),
        };
        return Ok(derived.into_impl(ident.clone(), input.generics.clone()));
    }

    if let Some(as_ty) = &container.type_as {
        let cr = &crate_rename;
        let inline = quote!(<#as_ty as #cr::Flow>::inline(cfg));
        deps.push(&syn::parse_quote!(#as_ty));
        let derived = DerivedFlow {
            crate_rename: crate_rename.clone(),
            flow_name,
            docs,
            inline: inline.clone(),
            inline_flattened: quote!(format!("({})", <#as_ty as #cr::Flow>::inline(cfg))),
            is_enum: quote!(false),
            is_opaque: false,
            opaque_bound: None,
            enum_decl_override: None,
            export: container.export,
            export_to: container.export_to.clone(),
            bound: container.bound.clone(),
            deps,
            concrete: container.concrete.clone(),
        };
        return Ok(derived.into_impl(ident.clone(), input.generics.clone()));
    }

    let (inline, inline_flattened, is_enum, enum_decl_override) = match &input.data {
        Data::Struct(data) => {
            let (inline, flattened) =
                derive_struct(&crate_rename, &container, &data.fields, &mut deps)?;
            (inline, flattened, quote!(false), None)
        }
        Data::Enum(data) => {
            if let Some(repr) = &container.flow_enum {
                // Validate: flow_enum is incompatible with generics
                if input.generics.type_params().next().is_some() {
                    return Err(syn::Error::new(
                        ident.span(),
                        "#[flow(flow_enum)] cannot be used on generic enums",
                    ));
                }
                // Validate: flow_enum is incompatible with tagging/opaque
                if container.tag.is_some() {
                    return Err(syn::Error::new(
                        ident.span(),
                        "#[flow(flow_enum)] cannot be combined with #[flow(tag = \"...\")]",
                    ));
                }
                if container.content.is_some() {
                    return Err(syn::Error::new(
                        ident.span(),
                        "#[flow(flow_enum)] cannot be combined with #[flow(content = \"...\")]",
                    ));
                }
                if container.untagged {
                    return Err(syn::Error::new(
                        ident.span(),
                        "#[flow(flow_enum)] cannot be combined with #[flow(untagged)]",
                    ));
                }
                if container.opaque.is_some() {
                    return Err(syn::Error::new(
                        ident.span(),
                        "#[flow(flow_enum)] cannot be combined with #[flow(opaque)]",
                    ));
                }
                let (inline, flattened, enum_decl) =
                    derive_flow_enum(&container, data, repr)?;
                (inline, flattened, quote!(true), Some(enum_decl))
            } else {
                let inline = derive_enum(&crate_rename, &container, data, &mut deps)?;
                let flattened = quote! {
                    format!("({})", Self::inline(cfg))
                };
                (inline, flattened, quote!(true), None)
            }
        }
        Data::Union(_) => {
            return Err(syn::Error::new(
                ident.span(),
                "Flow cannot be derived for unions",
            ));
        }
    };

    let (is_opaque, opaque_bound) = match &container.opaque {
        Some(Some(bound)) => (true, Some(quote!(#bound))),
        Some(None) => (true, None),
        None => {
            // Auto-opaque newtypes when configured in Cargo.toml:
            // [package.metadata.flowjs-rs]
            // opaque_newtypes = true
            let is_newtype = matches!(&input.data, Data::Struct(s) if matches!(&s.fields, Fields::Unnamed(u) if u.unnamed.len() == 1));
            if is_newtype && config::project_config().opaque_newtypes {
                // Use the inner type as the opaque bound
                if let Data::Struct(s) = &input.data {
                    if let Fields::Unnamed(u) = &s.fields {
                        let inner_ty = &u.unnamed[0].ty;
                        let bound_str = quote!(<#inner_ty as #crate_rename::Flow>::name(cfg));
                        (true, Some(bound_str))
                    } else {
                        (false, None)
                    }
                } else {
                    (false, None)
                }
            } else {
                (false, None)
            }
        }
    };

    let derived = DerivedFlow {
        crate_rename,
        flow_name,
        docs,
        inline,
        inline_flattened,
        is_enum,
        is_opaque,
        opaque_bound,
        enum_decl_override,
        export: container.export,
        export_to: container.export_to.clone(),
        bound: container.bound.clone(),
        deps,
        concrete: container.concrete.clone(),
    };

    Ok(derived.into_impl(ident.clone(), input.generics.clone()))
}

fn derive_struct(
    crate_rename: &Path,
    container: &ContainerAttr,
    fields: &Fields,
    deps: &mut Dependencies,
) -> Result<(TokenStream, TokenStream)> {
    match fields {
        Fields::Named(named) => {
            let mut formatted_fields: Vec<TokenStream> = Vec::new();
            let mut flattened_fields: Vec<TokenStream> = Vec::new();

            for f in &named.named {
                let field_attr = FieldAttr::from_attrs(&f.attrs)?;
                if field_attr.skip {
                    continue;
                }

                let field_name = f.ident.as_ref().unwrap();
                let ty = &f.ty;

                if field_attr.flatten {
                    // Flatten: add transitive deps, push inline_flattened
                    if field_attr.type_override.is_none() {
                        deps.append_from(ty);
                    }
                    flattened_fields
                        .push(quote!(<#ty as #crate_rename::Flow>::inline_flattened(cfg)));
                    continue;
                }

                let name =
                    utils::quote_property_name(&field_attr.rename.clone().unwrap_or_else(|| {
                        let raw = field_name.to_string();
                        container.rename_field(&raw)
                    }));

                // Resolve the effective type: `as` overrides the Rust type,
                // `type` overrides with a literal string.
                let effective_ty = field_attr.type_as.as_ref().unwrap_or(ty);

                // Track dependencies
                if field_attr.type_override.is_none() {
                    if field_attr.inline {
                        deps.append_from(effective_ty);
                    } else {
                        deps.push(effective_ty);
                    }
                }

                let type_str = if let Some(override_ty) = &field_attr.type_override {
                    quote!(#override_ty.to_owned())
                } else if field_attr.inline {
                    quote!(<#effective_ty as #crate_rename::Flow>::inline(cfg))
                } else {
                    quote!(<#effective_ty as #crate_rename::Flow>::name(cfg))
                };

                // Key-optional (`field?:`) only when explicitly marked or serde says omittable.
                // `Option<T>` without skip_serializing_if is always-present-but-nullable (`+field: ?T`),
                // NOT omittable (`+field?: ?T`). The `?T` nullability comes from Flow::name() for Option.
                let is_omittable = field_attr.is_optional(&container.optional_fields);
                let opt_marker = if is_omittable { "?" } else { "" };
                let variance = if field_attr.readonly { "+" } else { "" };

                formatted_fields.push(quote! {
                    format!("  {}{}{}: {},", #variance, #name, #opt_marker, #type_str)
                });
            }

            // Combine normal fields and flattened fields
            let inline = match (formatted_fields.len(), flattened_fields.len()) {
                (0, 0) => quote!("{||}".to_owned()),
                (_, 0) => quote! {{
                    let fields = vec![#(#formatted_fields),*];
                    format!("{{|\n{}\n|}}", fields.join("\n"))
                }},
                (0, 1) => {
                    let flat = &flattened_fields[0];
                    quote! {{
                        let f = #flat;
                        if f.starts_with('(') && f.ends_with(')') {
                            f[1..f.len() - 1].trim().to_owned()
                        } else {
                            f.trim().to_owned()
                        }
                    }}
                }
                (0, _) => quote! {{
                    let parts: Vec<String> = vec![#(#flattened_fields),*];
                    parts.join(" & ")
                }},
                (_, _) => quote! {{
                    let fields = vec![#(#formatted_fields),*];
                    let base = format!("{{|\n{}\n|}}", fields.join("\n"));
                    let flattened: Vec<String> = vec![#(#flattened_fields),*];
                    format!("{} & {}", base, flattened.join(" & "))
                }},
            };

            // inline_flattened always wraps in exact object (for use by parent flatten)
            let inline_flattened = match (formatted_fields.len(), flattened_fields.len()) {
                (_, 0) => quote! {{
                    let fields = vec![#(#formatted_fields),*];
                    format!("{{|\n{}\n|}}", fields.join("\n"))
                }},
                (0, _) => quote! {{
                    let parts: Vec<String> = vec![#(#flattened_fields),*];
                    parts.join(" & ")
                }},
                (_, _) => quote! {{
                    let fields = vec![#(#formatted_fields),*];
                    let base = format!("{{|\n{}\n|}}", fields.join("\n"));
                    let flattened: Vec<String> = vec![#(#flattened_fields),*];
                    format!("{} & {}", base, flattened.join(" & "))
                }},
            };

            Ok((inline, inline_flattened))
        }
        Fields::Unnamed(unnamed) => {
            if unnamed.unnamed.len() == 1 {
                // Newtype — inline the inner type
                let ty = &unnamed.unnamed[0].ty;
                deps.push(ty);
                let inline = quote!(<#ty as #crate_rename::Flow>::inline(cfg));
                let flattened = quote! {
                    format!("({})", <#ty as #crate_rename::Flow>::inline(cfg))
                };
                Ok((inline, flattened))
            } else {
                // Tuple struct → Flow tuple
                let elems: Vec<TokenStream> = unnamed
                    .unnamed
                    .iter()
                    .map(|f| {
                        let ty = &f.ty;
                        deps.push(ty);
                        quote!(<#ty as #crate_rename::Flow>::inline(cfg))
                    })
                    .collect();
                let inline = quote! {{
                    let elems: Vec<String> = vec![#(#elems),*];
                    format!("[{}]", elems.join(", "))
                }};
                let flattened = quote! {
                    format!("({})", Self::inline(cfg))
                };
                Ok((inline, flattened))
            }
        }
        Fields::Unit => {
            let inline = quote!(#crate_rename::flow_type::NULL.to_owned());
            let flattened = quote!(#crate_rename::flow_type::NULL.to_owned());
            Ok((inline, flattened))
        }
    }
}

fn derive_enum(
    crate_rename: &Path,
    container: &ContainerAttr,
    data: &syn::DataEnum,
    deps: &mut Dependencies,
) -> Result<TokenStream> {
    if data.variants.is_empty() {
        return Ok(quote!(#crate_rename::flow_type::EMPTY.to_owned()));
    }

    let is_untagged = container.untagged;
    let tag = &container.tag.as_deref().map(utils::quote_property_name);
    let content = &container.content.as_deref().map(utils::quote_property_name);

    let mut variant_defs: Vec<TokenStream> = Vec::new();
    for v in &data.variants {
        let variant_attr = VariantAttr::from_attrs(&v.attrs)?;
        if variant_attr.skip {
            continue;
        }

        let variant_name_raw = variant_attr.rename.clone().unwrap_or_else(|| {
            let raw = v.ident.to_string();
            container.rename_variant(&raw)
        });
        let variant_name = utils::escape_string_literal(&variant_name_raw);
        let variant_key = utils::quote_property_name(&variant_name_raw);

        // Per-variant untagged: bypass tagging for this specific variant
        let is_variant_untagged = is_untagged || variant_attr.untagged;

        // Per-variant type/as override: short-circuit all field processing
        if let Some(ref type_override) = variant_attr.type_override {
            let type_str = quote!(#type_override.to_owned());
            let def = if is_variant_untagged {
                type_str
            } else if let Some(tag_field) = tag {
                quote!(format!(
                    "{{| {}: '{}' |}} & {}",
                    #tag_field, #variant_name, #type_str
                ))
            } else {
                quote!(format!("{{| {}: {} |}}", #variant_key, #type_str))
            };
            variant_defs.push(def);
            continue;
        }
        if let Some(ref type_as) = variant_attr.type_as {
            deps.push(&syn::parse_quote!(#type_as));
            let type_str = quote!(<#type_as as #crate_rename::Flow>::inline(cfg));
            let def = if is_variant_untagged {
                type_str
            } else if let Some(tag_field) = tag {
                quote!(format!(
                    "{{| {}: '{}' |}} & {}",
                    #tag_field, #variant_name, #type_str
                ))
            } else {
                quote!(format!("{{| {}: {} |}}", #variant_key, #type_str))
            };
            variant_defs.push(def);
            continue;
        }

        let def = match &v.fields {
            Fields::Unit => {
                if is_variant_untagged {
                    quote!(#crate_rename::flow_type::NULL.to_owned())
                } else if let Some(tag_field) = tag {
                    quote!(format!("{{| {}: '{}' |}}", #tag_field, #variant_name))
                } else {
                    quote!(format!("'{}'", #variant_name))
                }
            }
            Fields::Unnamed(unnamed) => {
                if unnamed.unnamed.len() == 1 {
                    let ty = &unnamed.unnamed[0].ty;
                    deps.push(ty);
                    let inner = quote!(<#ty as #crate_rename::Flow>::inline(cfg));
                    if is_variant_untagged {
                        inner
                    } else if let (Some(tag_field), Some(content_field)) = (tag, content) {
                        quote!(format!(
                            "{{| {}: '{}', {}: {} |}}",
                            #tag_field, #variant_name, #content_field, #inner
                        ))
                    } else if let Some(tag_field) = tag {
                        quote!(format!(
                            "{{| {}: '{}' |}} & {}",
                            #tag_field, #variant_name, #inner
                        ))
                    } else {
                        quote!(format!(
                            "{{| {}: {} |}}",
                            #variant_key, #inner
                        ))
                    }
                } else {
                    // Multi-field tuple variant
                    let elems: Vec<TokenStream> = unnamed
                        .unnamed
                        .iter()
                        .map(|f| {
                            let ty = &f.ty;
                            deps.push(ty);
                            quote!(<#ty as #crate_rename::Flow>::inline(cfg))
                        })
                        .collect();
                    let tuple = quote! {{
                        let elems: Vec<String> = vec![#(#elems),*];
                        format!("[{}]", elems.join(", "))
                    }};
                    if is_variant_untagged {
                        tuple
                    } else if let (Some(tag_field), Some(content_field)) = (tag, content) {
                        quote!(format!(
                            "{{| {}: '{}', {}: {} |}}",
                            #tag_field, #variant_name, #content_field, #tuple
                        ))
                    } else {
                        quote!(format!(
                            "{{| {}: {} |}}",
                            #variant_key, #tuple
                        ))
                    }
                }
            }
            Fields::Named(named) => {
                let mut field_defs: Vec<TokenStream> = Vec::new();
                let mut flattened_defs: Vec<TokenStream> = Vec::new();
                for f in &named.named {
                    let field_attr = FieldAttr::from_attrs(&f.attrs)?;
                    if field_attr.skip {
                        continue;
                    }
                    let ty = &f.ty;

                    if field_attr.flatten {
                        if field_attr.type_override.is_none() {
                            deps.append_from(ty);
                        }
                        flattened_defs
                            .push(quote!(<#ty as #crate_rename::Flow>::inline_flattened(cfg)));
                        continue;
                    }

                    let field_name = f.ident.as_ref().unwrap();
                    // Per-variant rename_all takes priority over container
                    let name =
                        utils::quote_property_name(&field_attr.rename.clone().unwrap_or_else(
                            || {
                                let raw = field_name.to_string();
                                if let Some(ref inflection) = variant_attr.rename_all {
                                    inflection.apply(&raw)
                                } else {
                                    container.rename_variant_field(&raw)
                                }
                            },
                        ));
                    // Per-variant inline propagates to field dep tracking
                    let effective_inline = field_attr.inline || variant_attr.inline;
                    if field_attr.type_override.is_none() {
                        if effective_inline {
                            deps.append_from(ty);
                        } else {
                            deps.push(ty);
                        }
                    }
                    let effective_ty = field_attr.type_as.as_ref().unwrap_or(ty);
                    let type_str = if let Some(override_ty) = &field_attr.type_override {
                        quote!(#override_ty.to_owned())
                    } else if effective_inline {
                        quote!(<#effective_ty as #crate_rename::Flow>::inline(cfg))
                    } else {
                        quote!(<#effective_ty as #crate_rename::Flow>::name(cfg))
                    };
                    // Per-variant optional_fields overrides container
                    let variant_optional = match &variant_attr.optional_fields {
                        attr::Optional::Inherit => &container.optional_fields,
                        explicit => explicit,
                    };
                    let is_omittable = field_attr.is_optional(variant_optional);
                    let opt_marker = if is_omittable { "?" } else { "" };
                    let variance = if field_attr.readonly { "+" } else { "" };
                    field_defs.push(quote!(format!("{}{}{}: {}", #variance, #name, #opt_marker, #type_str)));
                }

                let obj = if flattened_defs.is_empty() {
                    quote! {{
                        let fields: Vec<String> = vec![#(#field_defs),*];
                        format!("{{| {} |}}", fields.join(", "))
                    }}
                } else if field_defs.is_empty() {
                    quote! {{
                        let parts: Vec<String> = vec![#(#flattened_defs),*];
                        parts.join(" & ")
                    }}
                } else {
                    quote! {{
                        let fields: Vec<String> = vec![#(#field_defs),*];
                        let base = format!("{{| {} |}}", fields.join(", "));
                        let flattened: Vec<String> = vec![#(#flattened_defs),*];
                        format!("{} & {}", base, flattened.join(" & "))
                    }}
                };

                if is_variant_untagged {
                    obj
                } else if let (Some(tag_field), Some(content_field)) = (tag, content) {
                    quote!(format!(
                        "{{| {}: '{}', {}: {} |}}",
                        #tag_field, #variant_name, #content_field, #obj
                    ))
                } else if let Some(tag_field) = tag {
                    // Internally tagged: inject tag field into the object
                    // Build the tagged object by prepending the tag to the field list
                    let tag_field_def = quote!(format!("{}: '{}'", #tag_field, #variant_name));

                    if flattened_defs.is_empty() {
                        // Simple case: all fields are regular, build a single exact object
                        let all_fields: Vec<_> = std::iter::once(tag_field_def.clone())
                            .chain(field_defs.iter().cloned())
                            .collect();
                        quote! {{
                            let fields: Vec<String> = vec![#(#all_fields),*];
                            format!("{{| {} |}}", fields.join(", "))
                        }}
                    } else {
                        // Has flattened fields: tag goes in base object, then intersect
                        let base_fields: Vec<_> = std::iter::once(tag_field_def.clone())
                            .chain(field_defs.iter().cloned())
                            .collect();
                        quote! {{
                            let fields: Vec<String> = vec![#(#base_fields),*];
                            let base = format!("{{| {} |}}", fields.join(", "));
                            let flattened: Vec<String> = vec![#(#flattened_defs),*];
                            format!("{} & {}", base, flattened.join(" & "))
                        }}
                    }
                } else {
                    quote!(format!(
                        "{{| {}: {} |}}",
                        #variant_key, #obj
                    ))
                }
            }
        };
        variant_defs.push(def);
    }

    if variant_defs.is_empty() {
        return Ok(quote!(#crate_rename::flow_type::EMPTY.to_owned()));
    }

    Ok(quote! {{
        let variants: Vec<String> = vec![#(#variant_defs),*];
        variants.join(" | ")
    }})
}

/// Derive a Flow `enum` declaration for Rust enums with only unit variants.
///
/// Produces Flow enum syntax instead of union type aliases:
/// - `#[flow(flow_enum)]` → `enum Name { A, B, }` (symbol enum)
/// - `#[flow(flow_enum = "string")]` → `enum Name of string { A = 'a', B = 'b', }` (string enum)
/// - `#[flow(flow_enum = "number")]` → `enum Name of number { A = 0, B = 1, }` (number enum)
/// - `#[flow(flow_enum = "boolean")]` → `enum Name of boolean { A = true, B = false, }` (boolean enum)
fn derive_flow_enum(
    container: &ContainerAttr,
    data: &syn::DataEnum,
    repr: &FlowEnumRepr,
) -> Result<(TokenStream, TokenStream, TokenStream)> {
    // Validate: all variants must be unit
    for v in &data.variants {
        if !matches!(v.fields, Fields::Unit) {
            return Err(syn::Error::new(
                v.ident.span(),
                "#[flow(flow_enum)] requires all variants to be unit (no fields)",
            ));
        }
    }

    // For boolean enums, validate exactly 2 non-skipped variants
    if matches!(repr, FlowEnumRepr::Boolean) {
        let active_count = data
            .variants
            .iter()
            .filter(|v| !VariantAttr::from_attrs(&v.attrs).map(|a| a.skip).unwrap_or(false))
            .count();
        if active_count != 2 {
            return Err(syn::Error::new(
                data.variants.first().map(|v| v.ident.span()).unwrap_or_else(proc_macro2::Span::call_site),
                "#[flow(flow_enum = \"boolean\")] requires exactly 2 non-skipped variants",
            ));
        }
    }

    let mut member_defs: Vec<TokenStream> = Vec::new();
    let mut next_discriminant: i64 = 0;
    let mut bool_index: usize = 0;

    for v in &data.variants {
        let variant_attr = VariantAttr::from_attrs(&v.attrs)?;
        if variant_attr.skip {
            continue;
        }

        let member_name = v.ident.to_string();

        match repr {
            FlowEnumRepr::Symbol => {
                member_defs.push(quote!(format!("  {},", #member_name)));
            }
            FlowEnumRepr::String => {
                let value = variant_attr.rename.clone().unwrap_or_else(|| {
                    container.rename_variant(&member_name)
                });
                let escaped = utils::escape_string_literal(&value);
                member_defs.push(quote!(format!("  {} = '{}',", #member_name, #escaped)));
            }
            FlowEnumRepr::Number => {
                // Extract discriminant value from `Variant = expr`, or auto-increment
                let disc_value = if let Some((_, expr)) = &v.discriminant {
                    match expr {
                        Expr::Lit(lit) => match &lit.lit {
                            Lit::Int(i) => {
                                let val = i.base10_parse::<i64>().map_err(|e| {
                                    syn::Error::new(i.span(), format!("invalid discriminant: {e}"))
                                })?;
                                next_discriminant = val + 1;
                                val
                            }
                            _ => {
                                return Err(syn::Error::new(
                                    expr.span(),
                                    "flow_enum = \"number\" requires integer literal discriminants",
                                ));
                            }
                        },
                        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Neg(_)) => {
                            if let Expr::Lit(lit) = &*unary.expr {
                                if let Lit::Int(i) = &lit.lit {
                                    let val = -(i.base10_parse::<i64>().map_err(|e| {
                                        syn::Error::new(i.span(), format!("invalid discriminant: {e}"))
                                    })?);
                                    next_discriminant = val + 1;
                                    val
                                } else {
                                    return Err(syn::Error::new(
                                        expr.span(),
                                        "flow_enum = \"number\" requires integer literal discriminants",
                                    ));
                                }
                            } else {
                                return Err(syn::Error::new(
                                    expr.span(),
                                    "flow_enum = \"number\" requires integer literal discriminants",
                                ));
                            }
                        }
                        _ => {
                            return Err(syn::Error::new(
                                expr.span(),
                                "flow_enum = \"number\" requires integer literal discriminants",
                            ));
                        }
                    }
                } else {
                    let val = next_discriminant;
                    next_discriminant = val + 1;
                    val
                };
                member_defs.push(quote!(format!("  {} = {},", #member_name, #disc_value)));
            }
            FlowEnumRepr::Boolean => {
                let bool_val = if bool_index == 0 { "true" } else { "false" };
                bool_index += 1;
                member_defs.push(quote!(format!("  {} = {},", #member_name, #bool_val)));
            }
        }
    }

    // If all variants were skipped (or enum is empty), fall back to empty type
    if member_defs.is_empty() {
        let inline = quote!("empty".to_owned());
        let inline_flattened = quote!("empty".to_owned());
        let enum_decl = quote! {
            format!("type {} = empty;", Self::ident(cfg))
        };
        return Ok((inline, inline_flattened, enum_decl));
    }

    let repr_suffix = match repr {
        FlowEnumRepr::Symbol => "",
        FlowEnumRepr::String => " of string",
        FlowEnumRepr::Number => " of number",
        FlowEnumRepr::Boolean => " of boolean",
    };

    let enum_decl = quote! {{
        let members: Vec<String> = vec![#(#member_defs),*];
        format!("enum {}{} {{\n{}\n}}", Self::ident(cfg), #repr_suffix, members.join("\n"))
    }};

    // inline: flow enums are referenced by name, not expanded
    let inline = quote!(Self::name(cfg));
    let inline_flattened = quote! {
        format!("({})", Self::name(cfg))
    };

    Ok((inline, inline_flattened, enum_decl))
}
