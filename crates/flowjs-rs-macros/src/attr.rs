//! Attribute parsing for `#[flow(...)]` and `#[serde(...)]` (when serde-compat is enabled).

use syn::{Attribute, Expr, Lit, Result, WherePredicate};

/// Container-level attributes (`#[flow(...)]` on struct/enum).
pub struct ContainerAttr {
    pub rename: Option<String>,
    pub rename_all: Option<Inflection>,
    pub rename_all_fields: Option<Inflection>,
    pub export: bool,
    pub export_to: Option<Expr>,
    pub tag: Option<String>,
    pub content: Option<String>,
    pub untagged: bool,
    pub crate_rename: Option<syn::Path>,
    pub bound: Option<Vec<WherePredicate>>,
    /// `Some(None)` = fully opaque, `Some(Some("string"))` = bounded opaque.
    pub opaque: Option<Option<String>>,
}

impl ContainerAttr {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut this = Self {
            rename: None,
            rename_all: None,
            rename_all_fields: None,
            export: false,
            export_to: None,
            tag: None,
            content: None,
            untagged: false,
            crate_rename: None,
            bound: None,
            opaque: None,
        };

        for attr in attrs {
            if attr.path().is_ident("flow") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.rename = Some(s.value());
                        }
                    } else if meta.path.is_ident("rename_all") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.rename_all = Some(Inflection::from_str(&s.value()));
                        }
                    } else if meta.path.is_ident("rename_all_fields") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.rename_all_fields = Some(Inflection::from_str(&s.value()));
                        }
                    } else if meta.path.is_ident("export") {
                        this.export = true;
                    } else if meta.path.is_ident("export_to") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            let val = s.value();
                            this.export_to =
                                Some(syn::parse_quote!(#val));
                        }
                    } else if meta.path.is_ident("tag") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.tag = Some(s.value());
                        }
                    } else if meta.path.is_ident("content") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.content = Some(s.value());
                        }
                    } else if meta.path.is_ident("untagged") {
                        this.untagged = true;
                    } else if meta.path.is_ident("crate") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.crate_rename = Some(s.parse()?);
                        }
                    } else if meta.path.is_ident("opaque") {
                        // #[flow(opaque)] or #[flow(opaque = "string")]
                        if meta.input.peek(syn::Token![=]) {
                            let value: Lit = meta.value()?.parse()?;
                            if let Lit::Str(s) = value {
                                this.opaque = Some(Some(s.value()));
                            }
                        } else {
                            this.opaque = Some(None);
                        }
                    } else if meta.path.is_ident("bound") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            let where_clause: syn::WhereClause =
                                syn::parse_str(&format!("where {}", s.value()))?;
                            this.bound =
                                Some(where_clause.predicates.into_iter().collect());
                        }
                    } else {
                        let path = meta.path.get_ident()
                            .map(|i| i.to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        return Err(meta.error(format!(
                            "unknown #[flow(...)] attribute: `{path}`"
                        )));
                    }
                    Ok(())
                })?;
            }

            #[cfg(feature = "serde-compat")]
            if attr.path().is_ident("serde") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        if this.rename.is_none() {
                            let value: Lit = meta.value()?.parse()?;
                            if let Lit::Str(s) = value {
                                this.rename = Some(s.value());
                            }
                        }
                    } else if meta.path.is_ident("rename_all") {
                        if this.rename_all.is_none() {
                            let value: Lit = meta.value()?.parse()?;
                            if let Lit::Str(s) = value {
                                this.rename_all = Some(Inflection::from_str(&s.value()));
                            }
                        }
                    } else if meta.path.is_ident("tag") {
                        if this.tag.is_none() {
                            let value: Lit = meta.value()?.parse()?;
                            if let Lit::Str(s) = value {
                                this.tag = Some(s.value());
                            }
                        }
                    } else if meta.path.is_ident("content") {
                        if this.content.is_none() {
                            let value: Lit = meta.value()?.parse()?;
                            if let Lit::Str(s) = value {
                                this.content = Some(s.value());
                            }
                        }
                    } else if meta.path.is_ident("untagged") {
                        if !this.untagged {
                            this.untagged = true;
                        }
                    } else {
                        // Skip other serde attributes
                        let _ = meta.value().ok();
                    }
                    Ok(())
                })?;
            }
        }

        Ok(this)
    }

    /// Apply `rename_all` inflection to a field name.
    pub fn rename_field(&self, raw: &str) -> String {
        match &self.rename_all {
            Some(inflection) => inflection.apply(raw),
            None => raw.to_owned(),
        }
    }

    /// Apply `rename_all` inflection to a variant name.
    pub fn rename_variant(&self, raw: &str) -> String {
        match &self.rename_all {
            Some(inflection) => inflection.apply(raw),
            None => raw.to_owned(),
        }
    }

    /// Apply `rename_all_fields` inflection to a field inside an enum variant.
    /// Falls back to `rename_all` if `rename_all_fields` is not set.
    pub fn rename_variant_field(&self, raw: &str) -> String {
        match &self.rename_all_fields {
            Some(inflection) => inflection.apply(raw),
            None => self.rename_field(raw),
        }
    }
}

/// Field-level attributes (`#[flow(...)]` on struct fields).
pub struct FieldAttr {
    pub rename: Option<String>,
    pub type_override: Option<String>,
    pub skip: bool,
    pub optional: bool,
    pub inline: bool,
    pub flatten: bool,
    /// serde: `skip_serializing_if` or `skip_serializing` seen
    pub maybe_omitted: bool,
    /// serde: `default` seen
    pub has_default: bool,
}

impl FieldAttr {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut this = Self {
            rename: None,
            type_override: None,
            skip: false,
            optional: false,
            inline: false,
            flatten: false,
            maybe_omitted: false,
            has_default: false,
        };

        for attr in attrs {
            if attr.path().is_ident("flow") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.rename = Some(s.value());
                        }
                    } else if meta.path.is_ident("type") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.type_override = Some(s.value());
                        }
                    } else if meta.path.is_ident("skip") {
                        this.skip = true;
                    } else if meta.path.is_ident("optional") {
                        this.optional = true;
                    } else if meta.path.is_ident("inline") {
                        this.inline = true;
                    } else if meta.path.is_ident("flatten") {
                        this.flatten = true;
                    } else {
                        let path = meta.path.get_ident()
                            .map(|i| i.to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        return Err(meta.error(format!(
                            "unknown #[flow(...)] field attribute: `{path}`"
                        )));
                    }
                    Ok(())
                })?;
            }

            #[cfg(feature = "serde-compat")]
            if attr.path().is_ident("serde") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        if this.rename.is_none() {
                            let value: Lit = meta.value()?.parse()?;
                            if let Lit::Str(s) = value {
                                this.rename = Some(s.value());
                            }
                        }
                    } else if meta.path.is_ident("skip") {
                        if !this.skip {
                            this.skip = true;
                        }
                    } else if meta.path.is_ident("skip_serializing") {
                        this.maybe_omitted = true;
                    } else if meta.path.is_ident("skip_serializing_if") {
                        let _ = meta.value().ok(); // consume the function name
                        this.maybe_omitted = true;
                    } else if meta.path.is_ident("default") {
                        let _ = meta.value().ok(); // consume optional default fn
                        this.has_default = true;
                    } else if meta.path.is_ident("flatten") {
                        if !this.flatten {
                            this.flatten = true;
                        }
                    } else {
                        let _ = meta.value().ok();
                    }
                    Ok(())
                })?;
            }
        }

        Ok(this)
    }

    /// Whether this field should be optional based on serde attributes.
    /// A field is optional if it has both `skip_serializing_if`/`skip_serializing` AND `default`.
    pub fn is_serde_optional(&self) -> bool {
        self.maybe_omitted && self.has_default
    }
}

/// Variant-level attributes (`#[flow(...)]` on enum variants).
pub struct VariantAttr {
    pub rename: Option<String>,
    pub skip: bool,
}

impl VariantAttr {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut this = Self {
            rename: None,
            skip: false,
        };

        for attr in attrs {
            if attr.path().is_ident("flow") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.rename = Some(s.value());
                        }
                    } else if meta.path.is_ident("skip") {
                        this.skip = true;
                    } else {
                        let path = meta.path.get_ident()
                            .map(|i| i.to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        return Err(meta.error(format!(
                            "unknown #[flow(...)] variant attribute: `{path}`"
                        )));
                    }
                    Ok(())
                })?;
            }

            #[cfg(feature = "serde-compat")]
            if attr.path().is_ident("serde") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        if this.rename.is_none() {
                            let value: Lit = meta.value()?.parse()?;
                            if let Lit::Str(s) = value {
                                this.rename = Some(s.value());
                            }
                        }
                    } else if meta.path.is_ident("skip") {
                        if !this.skip {
                            this.skip = true;
                        }
                    } else {
                        let _ = meta.value().ok();
                    }
                    Ok(())
                })?;
            }
        }

        Ok(this)
    }
}

/// Field/variant name inflection.
#[derive(Clone)]
pub enum Inflection {
    Lower,
    Upper,
    Camel,
    Snake,
    Pascal,
    ScreamingSnake,
    Kebab,
    ScreamingKebab,
}

impl Inflection {
    pub fn from_str(s: &str) -> Self {
        match s {
            "lowercase" => Self::Lower,
            "UPPERCASE" => Self::Upper,
            "camelCase" => Self::Camel,
            "snake_case" => Self::Snake,
            "PascalCase" => Self::Pascal,
            "SCREAMING_SNAKE_CASE" => Self::ScreamingSnake,
            "kebab-case" => Self::Kebab,
            "SCREAMING-KEBAB-CASE" => Self::ScreamingKebab,
            other => panic!("unknown rename_all inflection: \"{other}\". Expected one of: lowercase, UPPERCASE, camelCase, snake_case, PascalCase, SCREAMING_SNAKE_CASE, kebab-case, SCREAMING-KEBAB-CASE"),
        }
    }

    pub fn apply(&self, s: &str) -> String {
        match self {
            Self::Lower => s.to_lowercase(),
            Self::Upper => s.to_uppercase(),
            Self::Snake => to_snake_case(s),
            Self::ScreamingSnake => to_snake_case(s).to_uppercase(),
            Self::Camel => to_camel_case(s),
            Self::Pascal => to_pascal_case(s),
            Self::Kebab => to_snake_case(s).replace('_', "-"),
            Self::ScreamingKebab => to_snake_case(s).to_uppercase().replace('_', "-"),
        }
    }
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }
    result
}

fn to_camel_case(s: &str) -> String {
    let pascal = to_pascal_case(s);
    let mut chars = pascal.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().to_string() + chars.as_str(),
    }
}

fn to_pascal_case(s: &str) -> String {
    // Handle both snake_case and PascalCase input
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect()
}
