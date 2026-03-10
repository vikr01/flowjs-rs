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

struct DerivedFlow {
    crate_rename: Path,
    flow_name: Expr,
    docs: Vec<Expr>,
    inline: TokenStream,
    is_enum: TokenStream,
    is_opaque: bool,
    opaque_bound: Option<TokenStream>,
    export: bool,
    export_to: Option<Expr>,
    bound: Option<Vec<WherePredicate>>,
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

        let decl_fn = if self.is_opaque {
            let bound = self
                .opaque_bound
                .map(|b| quote! { format!("opaque type {}: {};", #flow_name, #b) })
                .unwrap_or_else(|| quote! { format!("opaque type {};", #flow_name) });
            quote! {
                fn decl(cfg: &#crate_rename::Config) -> String {
                    #bound
                }
            }
        } else {
            quote! {
                fn decl(cfg: &#crate_rename::Config) -> String {
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
            let dummies = generics.params.iter().map(|_| {
                quote!(#crate_rename::Dummy)
            });
            quote!(#rust_ty<#(#dummies),*>)
        };

        // decl_concrete: concrete declaration using actual generic args (not placeholders)
        let decl_concrete_fn = if self.is_opaque {
            // opaque types use the same decl for concrete
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

        quote! {
            #[automatically_derived]
            impl #impl_generics #crate_rename::Flow for #rust_ty #ty_generics #where_clause {
                type WithoutGenerics = #without_generics;
                type OptionInnerType = Self;

                fn name(cfg: &#crate_rename::Config) -> String {
                    #flow_name.to_owned()
                }

                fn inline(cfg: &#crate_rename::Config) -> String {
                    #inline
                }

                #decl_fn
                #decl_concrete_fn
                #docs_fn
                #output_path_fn

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
            // Use Dummy for all generic params
            let dummies = generics.params.iter().map(|_| {
                quote!(#crate_rename::Dummy)
            });
            quote!(#rust_ty<#(#dummies),*>)
        };

        quote! {
            #[cfg(test)]
            #[test]
            fn #test_name() {
                let cfg = #crate_rename::Config::from_env();
                <#ty as #crate_rename::Flow>::export(&cfg)
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
/// - `#[flow(opaque)]` — Emit as `opaque type Name` (fully opaque)
/// - `#[flow(opaque = "string")]` — Emit as `opaque type Name: string` (bounded)
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
/// - `#[flow(inline)]` — Inline the field type
/// - `#[flow(flatten)]` — Flatten nested fields
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

    let (inline, is_enum) = match &input.data {
        Data::Struct(data) => {
            let inline = derive_struct(&crate_rename, &container, &data.fields)?;
            (inline, quote!(false))
        }
        Data::Enum(data) => {
            let inline = derive_enum(&crate_rename, &container, data)?;
            (inline, quote!(true))
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
        is_enum,
        is_opaque,
        opaque_bound,
        export: container.export,
        export_to: container.export_to.clone(),
        bound: container.bound.clone(),
    };

    Ok(derived.into_impl(ident.clone(), input.generics.clone()))
}

fn derive_struct(
    crate_rename: &Path,
    container: &ContainerAttr,
    fields: &Fields,
) -> Result<TokenStream> {
    match fields {
        Fields::Named(named) => {
            let field_defs: Vec<TokenStream> = named
                .named
                .iter()
                .filter_map(|f| {
                    let field_attr = FieldAttr::from_attrs(&f.attrs).ok()?;
                    if field_attr.skip {
                        return None;
                    }

                    let field_name = f.ident.as_ref()?;
                    let name = field_attr.rename.clone().unwrap_or_else(|| {
                        let raw = field_name.to_string();
                        container.rename_field(&raw)
                    });

                    let ty = &f.ty;
                    let type_str = if let Some(override_ty) = &field_attr.type_override {
                        quote!(#override_ty.to_owned())
                    } else {
                        quote!(<#ty as #crate_rename::Flow>::inline(cfg))
                    };

                    let optional = field_attr.optional || is_option_type(ty);
                    let readonly = true; // Flow uses +field for readonly

                    let prefix = if readonly { "+" } else { "" };
                    let opt_marker = if optional { "?" } else { "" };

                    Some(quote! {
                        format!("  {}{}{}: {},", #prefix, #name, #opt_marker, #type_str)
                    })
                })
                .collect();

            if field_defs.is_empty() {
                Ok(quote!("{||}".to_owned()))
            } else {
                Ok(quote! {{
                    let fields = vec![#(#field_defs),*];
                    format!("{{|\n{}\n|}}", fields.join("\n"))
                }})
            }
        }
        Fields::Unnamed(unnamed) => {
            if unnamed.unnamed.len() == 1 {
                // Newtype — inline the inner type
                let ty = &unnamed.unnamed[0].ty;
                Ok(quote!(<#ty as #crate_rename::Flow>::inline(cfg)))
            } else {
                // Tuple struct → Flow tuple
                let elems: Vec<TokenStream> = unnamed
                    .unnamed
                    .iter()
                    .map(|f| {
                        let ty = &f.ty;
                        quote!(<#ty as #crate_rename::Flow>::inline(cfg))
                    })
                    .collect();
                Ok(quote! {{
                    let elems: Vec<String> = vec![#(#elems),*];
                    format!("[{}]", elems.join(", "))
                }})
            }
        }
        Fields::Unit => Ok(quote!("void".to_owned())),
    }
}

fn derive_enum(
    crate_rename: &Path,
    container: &ContainerAttr,
    data: &syn::DataEnum,
) -> Result<TokenStream> {
    let is_untagged = container.untagged;
    let tag = &container.tag;
    let content = &container.content;

    let variant_defs: Vec<TokenStream> = data
        .variants
        .iter()
        .filter_map(|v| {
            let variant_attr = VariantAttr::from_attrs(&v.attrs).ok()?;
            if variant_attr.skip {
                return None;
            }

            let variant_name = variant_attr.rename.clone().unwrap_or_else(|| {
                let raw = v.ident.to_string();
                container.rename_variant(&raw)
            });

            match &v.fields {
                Fields::Unit => {
                    if is_untagged {
                        Some(quote!("void".to_owned()))
                    } else if let Some(tag_field) = tag {
                        Some(quote!(format!("{{| +{}: '{}' |}}", #tag_field, #variant_name)))
                    } else {
                        Some(quote!(format!("'{}'", #variant_name)))
                    }
                }
                Fields::Unnamed(unnamed) => {
                    if unnamed.unnamed.len() == 1 {
                        let ty = &unnamed.unnamed[0].ty;
                        let inner = quote!(<#ty as #crate_rename::Flow>::inline(cfg));
                        if is_untagged {
                            Some(inner)
                        } else if let (Some(tag_field), Some(content_field)) = (tag, content) {
                            Some(quote!(format!(
                                "{{| +{}: '{}', +{}: {} |}}",
                                #tag_field, #variant_name, #content_field, #inner
                            )))
                        } else if let Some(tag_field) = tag {
                            Some(quote!(format!(
                                "{{| +{}: '{}', ... {} |}}",
                                #tag_field, #variant_name, #inner
                            )))
                        } else {
                            Some(quote!(format!(
                                "{{| {}: {} |}}",
                                #variant_name, #inner
                            )))
                        }
                    } else {
                        // Multi-field tuple variant — treat as tuple
                        let elems: Vec<TokenStream> = unnamed
                            .unnamed
                            .iter()
                            .map(|f| {
                                let ty = &f.ty;
                                quote!(<#ty as #crate_rename::Flow>::inline(cfg))
                            })
                            .collect();
                        let tuple = quote! {{
                            let elems: Vec<String> = vec![#(#elems),*];
                            format!("[{}]", elems.join(", "))
                        }};
                        if is_untagged {
                            Some(tuple)
                        } else if let (Some(tag_field), Some(content_field)) = (tag, content) {
                            Some(quote!(format!(
                                "{{| +{}: '{}', +{}: {} |}}",
                                #tag_field, #variant_name, #content_field, #tuple
                            )))
                        } else {
                            Some(quote!(format!(
                                "{{| {}: {} |}}",
                                #variant_name, #tuple
                            )))
                        }
                    }
                }
                Fields::Named(named) => {
                    let field_defs: Vec<TokenStream> = named
                        .named
                        .iter()
                        .filter_map(|f| {
                            let field_attr = FieldAttr::from_attrs(&f.attrs).ok()?;
                            if field_attr.skip {
                                return None;
                            }
                            let field_name = f.ident.as_ref()?;
                            let name = field_attr.rename.clone().unwrap_or_else(|| {
                                container.rename_field(&field_name.to_string())
                            });
                            let ty = &f.ty;
                            let type_str = if let Some(override_ty) = &field_attr.type_override {
                                quote!(#override_ty.to_owned())
                            } else {
                                quote!(<#ty as #crate_rename::Flow>::inline(cfg))
                            };
                            Some(quote!(format!("+{}: {}", #name, #type_str)))
                        })
                        .collect();

                    let obj = quote! {{
                        let fields: Vec<String> = vec![#(#field_defs),*];
                        format!("{{| {} |}}", fields.join(", "))
                    }};

                    if is_untagged {
                        Some(obj)
                    } else if let Some(tag_field) = tag {
                        Some(quote! {{
                            let base = #obj;
                            // Inject tag field into exact object
                            let tag_part = format!("+{}: '{}'", #tag_field, #variant_name);
                            let inner = &base[3..base.len()-3]; // strip {| and |}
                            if inner.trim().is_empty() {
                                format!("{{| {} |}}", tag_part)
                            } else {
                                format!("{{| {}, {} |}}", tag_part, inner.trim())
                            }
                        }})
                    } else {
                        Some(quote!(format!(
                            "{{| {}: {} |}}",
                            #variant_name, #obj
                        )))
                    }
                }
            }
        })
        .collect();

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
