//! Attribute parsing for `#[flow(...)]` and `#[serde(...)]` (when serde-compat is enabled).
//!
//! API-compatible with ts-rs: every `#[ts(...)]` attribute has a `#[flow(...)]` equivalent
//! with identical semantics. Flow-specific additions (opaque, flow_enum) extend the surface.

use syn::{Attribute, Expr, Ident, Lit, Result, Token, WherePredicate};

// ── Optional system ─────────────────────────────────────────────────────

/// Controls how `Option<T>` fields are represented in Flow.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Optional {
    /// `Option<T>` → `field?: T` (or `field?: T | null` if nullable)
    Optional { nullable: bool },
    /// Explicitly NOT optional — standard behavior.
    NotOptional,
    /// Inherit from parent (container-level `optional_fields`).
    Inherit,
}

impl Optional {
    /// Parse from `#[flow(optional)]`, `#[flow(optional = nullable)]`, `#[flow(optional = false)]`.
    pub fn parse_from_meta(meta: &syn::meta::ParseNestedMeta<'_>) -> Result<Self> {
        if meta.input.peek(Token![=]) {
            let value: Lit = meta.value()?.parse()?;
            if let Lit::Str(s) = &value {
                match s.value().as_str() {
                    "nullable" => Ok(Self::Optional { nullable: true }),
                    other => Err(meta.error(format!(
                        "unknown optional value: `{other}`. Expected \"nullable\""
                    ))),
                }
            } else if let Lit::Bool(b) = &value {
                if b.value() {
                    Ok(Self::Optional { nullable: false })
                } else {
                    Ok(Self::NotOptional)
                }
            } else {
                Err(meta.error("expected string or bool for `optional`"))
            }
        } else {
            Ok(Self::Optional { nullable: false })
        }
    }

    /// Resolve field-level optional against container-level optional_fields.
    #[allow(dead_code)]
    pub fn resolve(&self, container: &Self) -> bool {
        match self {
            Self::Optional { .. } => true,
            Self::NotOptional => false,
            Self::Inherit => matches!(container, Self::Optional { .. }),
        }
    }

    #[allow(dead_code)]
    pub fn is_nullable(&self, container: &Self) -> bool {
        match self {
            Self::Optional { nullable } => *nullable,
            Self::Inherit => match container {
                Self::Optional { nullable } => *nullable,
                _ => false,
            },
            Self::NotOptional => false,
        }
    }
}

// ── Flow enum representation ────────────────────────────────────────────

/// Flow enum representation type.
#[derive(Clone)]
pub enum FlowEnumRepr {
    Symbol,
    String,
    Number,
    Boolean,
}

// ── Container attributes ────────────────────────────────────────────────

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
    /// Flow enum declaration. Also set by `repr(enum)` / `repr(enum = name)`.
    pub flow_enum: Option<FlowEnumRepr>,
    /// `#[flow(type = "...")]` — raw Flow type override for the whole container.
    pub type_override: Option<String>,
    /// `#[flow(as = "...")]` — delegate to another Rust type's Flow representation.
    pub type_as: Option<syn::Type>,
    /// `#[flow(concrete(T = i32, ...))]` — replace generic params with concrete types.
    pub concrete: std::collections::HashMap<Ident, syn::Type>,
    /// `#[flow(optional_fields)]` — make all `Option<T>` fields optional.
    pub optional_fields: Optional,
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
            flow_enum: None,
            type_override: None,
            type_as: None,
            concrete: std::collections::HashMap::new(),
            optional_fields: Optional::Inherit,
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
                            this.export_to = Some(syn::parse_quote!(#val));
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
                        if meta.input.peek(Token![=]) {
                            let value: Lit = meta.value()?.parse()?;
                            if let Lit::Str(s) = value {
                                this.opaque = Some(Some(s.value()));
                            }
                        } else {
                            this.opaque = Some(None);
                        }
                    } else if meta.path.is_ident("flow_enum") {
                        if meta.input.peek(Token![=]) {
                            let value: Lit = meta.value()?.parse()?;
                            if let Lit::Str(s) = value {
                                match s.value().as_str() {
                                    "string" => this.flow_enum = Some(FlowEnumRepr::String),
                                    "symbol" => this.flow_enum = Some(FlowEnumRepr::Symbol),
                                    "number" => this.flow_enum = Some(FlowEnumRepr::Number),
                                    "boolean" => this.flow_enum = Some(FlowEnumRepr::Boolean),
                                    other => return Err(meta.error(format!(
                                        "unknown flow_enum representation: `{other}`. Expected \"string\", \"symbol\", \"number\", or \"boolean\""
                                    ))),
                                }
                            }
                        } else {
                            this.flow_enum = Some(FlowEnumRepr::Symbol);
                        }
                    } else if meta.path.is_ident("type") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.type_override = Some(s.value());
                        }
                    } else if meta.path.is_ident("as") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.type_as = Some(s.parse()?);
                        }
                    } else if meta.path.is_ident("concrete") {
                        // #[flow(concrete(T = i32, U = String))]
                        let content;
                        syn::parenthesized!(content in meta.input);
                        while !content.is_empty() {
                            let ident: Ident = content.parse()?;
                            let _: Token![=] = content.parse()?;
                            let ty: syn::Type = content.parse()?;
                            this.concrete.insert(ident, ty);
                            if content.peek(Token![,]) {
                                let _: Token![,] = content.parse()?;
                            }
                        }
                    } else if meta.path.is_ident("optional_fields") {
                        this.optional_fields = Optional::parse_from_meta(&meta)?;
                    } else if meta.path.is_ident("repr") {
                        // ts-rs compat: #[flow(repr(enum))] or #[flow(repr(enum = name))]
                        let content;
                        syn::parenthesized!(content in meta.input);
                        let ident: Ident = content.parse()?;
                        if ident != "enum" {
                            return Err(content.error("expected `enum`"));
                        }
                        if content.peek(Token![=]) {
                            let _: Token![=] = content.parse()?;
                            let value: Ident = content.parse()?;
                            if value == "name" {
                                this.flow_enum = Some(FlowEnumRepr::String);
                            } else {
                                return Err(content.error("expected `name`"));
                            }
                        } else {
                            this.flow_enum = Some(FlowEnumRepr::Number);
                        }
                    } else if meta.path.is_ident("bound") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            let where_clause: syn::WhereClause =
                                syn::parse_str(&format!("where {}", s.value()))?;
                            let preds: Vec<_> = where_clause.predicates.into_iter().collect();
                            match &mut this.bound {
                                Some(existing) => existing.extend(preds),
                                None => this.bound = Some(preds),
                            }
                        }
                    } else {
                        let path = meta
                            .path
                            .get_ident()
                            .map(|i| i.to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        return Err(meta.error(format!("unknown #[flow(...)] attribute: `{path}`")));
                    }
                    Ok(())
                })?;
            }

            #[cfg(feature = "serde-compat")]
            if attr.path().is_ident("serde") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        let value: Lit = meta.value()?.parse()?;
                        if this.rename.is_none() {
                            if let Lit::Str(s) = value {
                                this.rename = Some(s.value());
                            }
                        }
                    } else if meta.path.is_ident("rename_all") {
                        let value: Lit = meta.value()?.parse()?;
                        if this.rename_all.is_none() {
                            if let Lit::Str(s) = value {
                                this.rename_all = Some(Inflection::from_str(&s.value()));
                            }
                        }
                    } else if meta.path.is_ident("rename_all_fields") {
                        let value: Lit = meta.value()?.parse()?;
                        if this.rename_all_fields.is_none() {
                            if let Lit::Str(s) = value {
                                this.rename_all_fields = Some(Inflection::from_str(&s.value()));
                            }
                        }
                    } else if meta.path.is_ident("tag") {
                        let value: Lit = meta.value()?.parse()?;
                        if this.tag.is_none() {
                            if let Lit::Str(s) = value {
                                this.tag = Some(s.value());
                            }
                        }
                    } else if meta.path.is_ident("content") {
                        let value: Lit = meta.value()?.parse()?;
                        if this.content.is_none() {
                            if let Lit::Str(s) = value {
                                this.content = Some(s.value());
                            }
                        }
                    } else if meta.path.is_ident("untagged") {
                        if !this.untagged {
                            this.untagged = true;
                        }
                    } else {
                        // Skip other serde attributes — consume = value if present
                        let _ = meta.value().and_then(|v| v.parse::<Lit>()).ok();
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

// ── Field attributes ────────────────────────────────────────────────────

/// Field-level attributes (`#[flow(...)]` on struct fields).
pub struct FieldAttr {
    pub rename: Option<String>,
    pub type_override: Option<String>,
    pub type_as: Option<syn::Type>,
    pub skip: bool,
    pub optional: Optional,
    pub inline: bool,
    pub flatten: bool,
    /// Add the `+` (covariant/readonly) prefix on this field.
    /// Default: false (matches ts-rs behavior — fields are mutable by default).
    pub readonly: bool,
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
            type_as: None,
            skip: false,
            optional: Optional::Inherit,
            inline: false,
            flatten: false,
            readonly: false,
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
                    } else if meta.path.is_ident("as") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.type_as = Some(s.parse()?);
                        }
                    } else if meta.path.is_ident("skip") {
                        this.skip = true;
                    } else if meta.path.is_ident("optional") {
                        this.optional = Optional::parse_from_meta(&meta)?;
                    } else if meta.path.is_ident("inline") {
                        this.inline = true;
                    } else if meta.path.is_ident("flatten") {
                        this.flatten = true;
                    } else if meta.path.is_ident("readonly") {
                        this.readonly = true;
                    } else {
                        let path = meta
                            .path
                            .get_ident()
                            .map(|i| i.to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        return Err(
                            meta.error(format!("unknown #[flow(...)] field attribute: `{path}`"))
                        );
                    }
                    Ok(())
                })?;
            }

            #[cfg(feature = "serde-compat")]
            if attr.path().is_ident("serde") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        let value: Lit = meta.value()?.parse()?;
                        if this.rename.is_none() {
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
                        let _ = meta.value().and_then(|v| v.parse::<Lit>()).ok();
                        this.maybe_omitted = true;
                    } else if meta.path.is_ident("default") {
                        let _ = meta.value().and_then(|v| v.parse::<Lit>()).ok();
                        this.has_default = true;
                    } else if meta.path.is_ident("flatten") {
                        if !this.flatten {
                            this.flatten = true;
                        }
                    } else {
                        // Skip other serde attributes — consume = value if present
                        let _ = meta.value().and_then(|v| v.parse::<Lit>()).ok();
                    }
                    Ok(())
                })?;
            }
        }

        Ok(this)
    }

    /// Whether this field should be key-optional (`field?:`).
    pub fn is_optional(&self, container_optional: &Optional) -> bool {
        match &self.optional {
            Optional::Optional { .. } => true,
            Optional::NotOptional => false,
            Optional::Inherit => {
                // Inherit from container, or fall back to serde heuristic
                if matches!(container_optional, Optional::Optional { .. }) {
                    true
                } else {
                    self.is_serde_optional()
                }
            }
        }
    }

    /// Whether this field can be absent in serialized output based on serde attributes.
    pub fn is_serde_optional(&self) -> bool {
        self.maybe_omitted
    }
}

// ── Variant attributes ──────────────────────────────────────────────────

/// Variant-level attributes (`#[flow(...)]` on enum variants).
pub struct VariantAttr {
    pub rename: Option<String>,
    pub skip: bool,
    /// `#[flow(type = "...")]` — override variant type.
    pub type_override: Option<String>,
    /// `#[flow(as = "...")]` — delegate to another type.
    pub type_as: Option<syn::Type>,
    /// `#[flow(rename_all = "...")]` — per-variant field renaming.
    pub rename_all: Option<Inflection>,
    /// `#[flow(inline)]` — inline variant type definition.
    pub inline: bool,
    /// `#[flow(untagged)]` — make this specific variant untagged.
    pub untagged: bool,
    /// `#[flow(optional_fields)]` — per-variant optional field behavior.
    pub optional_fields: Optional,
}

impl VariantAttr {
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut this = Self {
            rename: None,
            skip: false,
            type_override: None,
            type_as: None,
            rename_all: None,
            inline: false,
            untagged: false,
            optional_fields: Optional::Inherit,
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
                    } else if meta.path.is_ident("type") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.type_override = Some(s.value());
                        }
                    } else if meta.path.is_ident("as") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.type_as = Some(s.parse()?);
                        }
                    } else if meta.path.is_ident("rename_all") {
                        let value: Lit = meta.value()?.parse()?;
                        if let Lit::Str(s) = value {
                            this.rename_all = Some(Inflection::from_str(&s.value()));
                        }
                    } else if meta.path.is_ident("inline") {
                        this.inline = true;
                    } else if meta.path.is_ident("untagged") {
                        this.untagged = true;
                    } else if meta.path.is_ident("optional_fields") {
                        this.optional_fields = Optional::parse_from_meta(&meta)?;
                    } else {
                        let path = meta
                            .path
                            .get_ident()
                            .map(|i| i.to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        return Err(
                            meta.error(format!("unknown #[flow(...)] variant attribute: `{path}`"))
                        );
                    }
                    Ok(())
                })?;
            }

            #[cfg(feature = "serde-compat")]
            if attr.path().is_ident("serde") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("rename") {
                        let value: Lit = meta.value()?.parse()?;
                        if this.rename.is_none() {
                            if let Lit::Str(s) = value {
                                this.rename = Some(s.value());
                            }
                        }
                    } else if meta.path.is_ident("skip") {
                        if !this.skip {
                            this.skip = true;
                        }
                    } else if meta.path.is_ident("untagged") {
                        if !this.untagged {
                            this.untagged = true;
                        }
                    } else if meta.path.is_ident("rename_all") {
                        let value: Lit = meta.value()?.parse()?;
                        if this.rename_all.is_none() {
                            if let Lit::Str(s) = value {
                                this.rename_all = Some(Inflection::from_str(&s.value()));
                            }
                        }
                    } else {
                        // Skip other serde attributes — consume = value if present
                        let _ = meta.value().and_then(|v| v.parse::<Lit>()).ok();
                    }
                    Ok(())
                })?;
            }
        }

        Ok(this)
    }
}

// ── Inflection ──────────────────────────────────────────────────────────

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
            // This panic is acceptable in a proc macro context — the error points to the attribute.
            // A syn::Error would be better but requires threading spans through all call sites.
            other => panic!("unknown rename_all value: \"{other}\". Expected one of: lowercase, UPPERCASE, camelCase, snake_case, PascalCase, SCREAMING_SNAKE_CASE, kebab-case, SCREAMING-KEBAB-CASE"),
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
