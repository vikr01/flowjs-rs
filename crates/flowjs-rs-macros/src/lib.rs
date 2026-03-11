//! Derive macro for flowjs-rs.
//!
//! Generates `Flow` trait implementations from Rust struct and enum definitions,
//! producing Flow type declarations.

#![deny(unused)]

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, Data, DeriveInput, Expr, Fields, GenericParam, Generics, Lit,
    Meta, Path, Result, Type, WherePredicate,
};

mod attr;
mod utils;

use attr::{ContainerAttr, FieldAttr, VariantAttr};

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
    export: bool,
    export_to: Option<Expr>,
    bound: Option<Vec<WherePredicate>>,
    deps: Dependencies,
}

impl DerivedFlow {
    fn into_impl(self, rust_ty: Ident, generics: Generics) -> TokenStream {
        let export_test = self.export.then(|| self.generate_export_test(&rust_ty, &generics));

        let output_path_fn = {
            let flow_name = &self.flow_name;
            let path_string = match &self.export_to {
                Some(dir_or_file) => quote! {{
                    let dir_or_file = format!("{}", #dir_or_file);
                    if dir_or_file.ends_with('/') {
                        format!("{dir_or_file}{}.js.flow", #flow_name)
                    } else {
                        format!("{dir_or_file}")
                    }
                }},
                None => quote![format!("{}.js.flow", #flow_name)],
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

        // Generate name() with generic parameters
        let name_fn = {
            let generic_names: Vec<_> = generics
                .type_params()
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
        let decl_fn = if self.is_opaque {
            let bound = self
                .opaque_bound
                .map(|b| quote! { format!("declare export opaque type {}: {};", #flow_name, #b) })
                .unwrap_or_else(|| quote! { format!("declare export opaque type {};", #flow_name) });
            quote! {
                fn decl(cfg: &#crate_rename::Config) -> String {
                    #bound
                }
            }
        } else {
            let has_generics = generics.type_params().next().is_some();
            if has_generics {
                // For generic types: create dummy types, get inline from WithoutGenerics
                let generic_idents: Vec<_> = generics
                    .type_params()
                    .map(|tp| &tp.ident)
                    .collect();

                // Generate dummy type declarations for each generic param
                let dummy_decls: Vec<_> = generic_idents.iter().map(|ident| {
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
                }).collect();

                let generics_str: Vec<_> = generic_idents.iter().map(|ident| {
                    quote!(stringify!(#ident))
                }).collect();

                // Build full generic args for instantiation (named dummies for type params)
                let full_generic_args: Vec<_> = generics.params.iter().map(|p| match p {
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
                }).collect();

                quote! {
                    fn decl(cfg: &#crate_rename::Config) -> String {
                        // Named dummies output the type param name (e.g. "T") instead of "any".
                        // If the struct has non-Flow trait bounds, use #[flow(bound = "")] to override.
                        #(#dummy_decls)*
                        let inline = <#rust_ty<#(#full_generic_args),*> as #crate_rename::Flow>::inline(cfg);
                        let generics = format!("<{}>", vec![#(#generics_str.to_owned()),*].join(", "));
                        format!("type {}{generics} = {inline};", #flow_name)
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
        let decl_concrete_fn = if self.is_opaque {
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
        // Add Flow bound for all type params
        for param in &generics.params {
            if let GenericParam::Type(tp) = param {
                let ident = &tp.ident;
                let where_clause = bounds.make_where_clause();
                where_clause
                    .predicates
                    .push(parse_quote!(#ident: #crate_rename::Flow));
            }
        }
        let (impl_generics, ty_generics, where_clause) = bounds.split_for_impl();

        // WithoutGenerics: if no generics, Self; otherwise replace all type params with Dummy
        let without_generics = if generics.params.is_empty() {
            quote!(Self)
        } else {
            let params = generics.params.iter().map(|p| match p {
                GenericParam::Type(_) => quote!(#crate_rename::Dummy),
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

        // visit_generics: iterate type params
        let visit_generics_fn = {
            let generic_visits: Vec<_> = generics
                .type_params()
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
                GenericParam::Type(_) => quote!(#crate_rename::Dummy),
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

    let (inline, inline_flattened, is_enum) = match &input.data {
        Data::Struct(data) => {
            let (inline, flattened) =
                derive_struct(&crate_rename, &container, &data.fields, &mut deps)?;
            (inline, flattened, quote!(false))
        }
        Data::Enum(data) => {
            let inline = derive_enum(&crate_rename, &container, data, &mut deps)?;
            let flattened = quote! {
                format!("({})", Self::inline(cfg))
            };
            (inline, flattened, quote!(true))
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
        None => (false, None),
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
        export: container.export,
        export_to: container.export_to.clone(),
        bound: container.bound.clone(),
        deps,
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
                    flattened_fields.push(
                        quote!(<#ty as #crate_rename::Flow>::inline_flattened(cfg)),
                    );
                    continue;
                }

                let name = utils::quote_property_name(
                    &field_attr.rename.clone().unwrap_or_else(|| {
                        let raw = field_name.to_string();
                        container.rename_field(&raw)
                    }),
                );

                // Track dependencies
                if field_attr.type_override.is_none() {
                    if field_attr.inline {
                        deps.append_from(ty);
                    } else {
                        deps.push(ty);
                    }
                }

                let type_str = if let Some(override_ty) = &field_attr.type_override {
                    quote!(#override_ty.to_owned())
                } else if field_attr.inline {
                    quote!(<#ty as #crate_rename::Flow>::inline(cfg))
                } else {
                    quote!(<#ty as #crate_rename::Flow>::name(cfg))
                };

                let is_optional = field_attr.optional || field_attr.is_serde_optional() || is_option_type(ty);
                let opt_marker = if is_optional { "?" } else { "" };

                formatted_fields.push(quote! {
                    format!("  +{}{}: {},", #name, #opt_marker, #type_str)
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
            let inline = quote!(#crate_rename::flow_type::VOID.to_owned());
            let flattened = quote!(#crate_rename::flow_type::VOID.to_owned());
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
        // Escaped version for use inside string literal values: 'VariantName'
        let variant_name = utils::escape_string_literal(&variant_name_raw);
        // Quoted version for use as an object key (externally-tagged enum)
        let variant_key = utils::quote_property_name(&variant_name_raw);

        let def = match &v.fields {
            Fields::Unit => {
                if is_untagged {
                    quote!(#crate_rename::flow_type::VOID.to_owned())
                } else if let Some(tag_field) = tag {
                    quote!(format!("{{| +{}: '{}' |}}", #tag_field, #variant_name))
                } else {
                    quote!(format!("'{}'", #variant_name))
                }
            }
            Fields::Unnamed(unnamed) => {
                if unnamed.unnamed.len() == 1 {
                    let ty = &unnamed.unnamed[0].ty;
                    deps.push(ty);
                    let inner = quote!(<#ty as #crate_rename::Flow>::inline(cfg));
                    if is_untagged {
                        inner
                    } else if let (Some(tag_field), Some(content_field)) = (tag, content) {
                        quote!(format!(
                            "{{| +{}: '{}', +{}: {} |}}",
                            #tag_field, #variant_name, #content_field, #inner
                        ))
                    } else if let Some(tag_field) = tag {
                        quote!(format!(
                            "{{| +{}: '{}' |}} & {}",
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
                    if is_untagged {
                        tuple
                    } else if let (Some(tag_field), Some(content_field)) = (tag, content) {
                        quote!(format!(
                            "{{| +{}: '{}', +{}: {} |}}",
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
                        flattened_defs.push(
                            quote!(<#ty as #crate_rename::Flow>::inline_flattened(cfg)),
                        );
                        continue;
                    }

                    let field_name = f.ident.as_ref().unwrap();
                    let name = utils::quote_property_name(
                        &field_attr.rename.clone().unwrap_or_else(|| {
                            container.rename_variant_field(&field_name.to_string())
                        }),
                    );
                    if field_attr.type_override.is_none() {
                        if field_attr.inline {
                            deps.append_from(ty);
                        } else {
                            deps.push(ty);
                        }
                    }
                    let type_str = if let Some(override_ty) = &field_attr.type_override {
                        quote!(#override_ty.to_owned())
                    } else if field_attr.inline {
                        quote!(<#ty as #crate_rename::Flow>::inline(cfg))
                    } else {
                        quote!(<#ty as #crate_rename::Flow>::name(cfg))
                    };
                    let is_optional = field_attr.optional || field_attr.is_serde_optional() || is_option_type(ty);
                    let opt_marker = if is_optional { "?" } else { "" };
                    field_defs.push(quote!(format!("+{}{}: {}", #name, #opt_marker, #type_str)));
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

                if is_untagged {
                    obj
                } else if let (Some(tag_field), Some(content_field)) = (tag, content) {
                    quote!(format!(
                        "{{| +{}: '{}', +{}: {} |}}",
                        #tag_field, #variant_name, #content_field, #obj
                    ))
                } else if let Some(tag_field) = tag {
                    // Internally tagged: inject tag field into the object
                    // Build the tagged object by prepending the tag to the field list
                    let tag_field_def = quote!(format!("+{}: '{}'", #tag_field, #variant_name));

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

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(path) = ty {
        if let Some(seg) = path.path.segments.last() {
            return seg.ident == "Option";
        }
    }
    false
}
