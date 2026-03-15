//! Parse `#[serde(...)]` attributes for derive macros.
//!
//! Used by derive macros (ts-rs, flowjs-rs, etc.) that need to read serde attributes
//! to produce type definitions matching serde's serialization behavior.
//!
//! Handles the "always consume the value" pattern to avoid parse buffer issues
//! when serde attributes coexist with other derive attributes.

use derive_inflection::Inflection;
use syn::{Attribute, Lit, Result};

/// Serde tagging model for enums.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tagged {
    /// Default: `{ "VariantName": data }`
    Externally,
    /// `#[serde(tag = "type")]`: `{ "type": "VariantName", ...data }`
    Internally { tag: String },
    /// `#[serde(tag = "t", content = "c")]`: `{ "t": "VariantName", "c": data }`
    Adjacently { tag: String, content: String },
    /// `#[serde(untagged)]`: just `data`
    Untagged,
}

/// Serde attributes parsed from a container (struct or enum).
#[derive(Debug, Clone, Default)]
pub struct SerdeContainer {
    pub rename: Option<String>,
    pub rename_all: Option<Inflection>,
    pub rename_all_fields: Option<Inflection>,
    pub tag: Option<String>,
    pub content: Option<String>,
    pub untagged: bool,
    pub transparent: bool,
}

impl SerdeContainer {
    /// Parse serde container attributes from a list of `syn::Attribute`.
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut this = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                // Always consume values to avoid leaving the parse buffer dirty.
                if meta.path.is_ident("rename") {
                    let value: Lit = meta.value()?.parse()?;
                    if let Lit::Str(s) = value {
                        this.rename = Some(s.value());
                    }
                } else if meta.path.is_ident("rename_all") {
                    let value: Lit = meta.value()?.parse()?;
                    if let Lit::Str(s) = value {
                        this.rename_all = Inflection::parse(&s.value());
                    }
                } else if meta.path.is_ident("rename_all_fields") {
                    let value: Lit = meta.value()?.parse()?;
                    if let Lit::Str(s) = value {
                        this.rename_all_fields = Inflection::parse(&s.value());
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
                } else if meta.path.is_ident("transparent") {
                    this.transparent = true;
                } else {
                    // Consume unknown attributes to keep the parse buffer clean
                    let _ = meta.value().and_then(|v| v.parse::<Lit>()).ok();
                }
                Ok(())
            })?;
        }

        Ok(this)
    }

    /// Resolve the tagging model from the parsed attributes.
    pub fn tagged(&self) -> Tagged {
        if self.untagged {
            Tagged::Untagged
        } else if let (Some(tag), Some(content)) = (&self.tag, &self.content) {
            Tagged::Adjacently {
                tag: tag.clone(),
                content: content.clone(),
            }
        } else if let Some(tag) = &self.tag {
            Tagged::Internally { tag: tag.clone() }
        } else {
            Tagged::Externally
        }
    }
}

/// Serde attributes parsed from a field.
#[derive(Debug, Clone, Default)]
pub struct SerdeField {
    pub rename: Option<String>,
    pub skip: bool,
    pub skip_serializing: bool,
    pub skip_serializing_if: bool,
    pub skip_deserializing: bool,
    pub flatten: bool,
    pub has_default: bool,
    pub with: bool,
}

impl SerdeField {
    /// Parse serde field attributes.
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut this = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    let value: Lit = meta.value()?.parse()?;
                    if let Lit::Str(s) = value {
                        this.rename = Some(s.value());
                    }
                } else if meta.path.is_ident("skip") {
                    this.skip = true;
                } else if meta.path.is_ident("skip_serializing") {
                    this.skip_serializing = true;
                } else if meta.path.is_ident("skip_serializing_if") {
                    let _ = meta.value().and_then(|v| v.parse::<Lit>()).ok();
                    this.skip_serializing_if = true;
                } else if meta.path.is_ident("skip_deserializing") {
                    this.skip_deserializing = true;
                } else if meta.path.is_ident("default") {
                    let _ = meta.value().and_then(|v| v.parse::<Lit>()).ok();
                    this.has_default = true;
                } else if meta.path.is_ident("flatten") {
                    this.flatten = true;
                } else if meta.path.is_ident("with") || meta.path.is_ident("serialize_with") {
                    let _ = meta.value().and_then(|v| v.parse::<Lit>()).ok();
                    this.with = true;
                } else {
                    let _ = meta.value().and_then(|v| v.parse::<Lit>()).ok();
                }
                Ok(())
            })?;
        }

        Ok(this)
    }

    /// Whether this field may be omitted from serialized output.
    pub fn maybe_omitted(&self) -> bool {
        self.skip_serializing || self.skip_serializing_if
    }
}

/// Serde attributes parsed from an enum variant.
#[derive(Debug, Clone, Default)]
pub struct SerdeVariant {
    pub rename: Option<String>,
    pub rename_all: Option<Inflection>,
    pub skip: bool,
    pub untagged: bool,
}

impl SerdeVariant {
    /// Parse serde variant attributes.
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut this = Self::default();

        for attr in attrs {
            if !attr.path().is_ident("serde") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    let value: Lit = meta.value()?.parse()?;
                    if let Lit::Str(s) = value {
                        this.rename = Some(s.value());
                    }
                } else if meta.path.is_ident("rename_all") {
                    let value: Lit = meta.value()?.parse()?;
                    if let Lit::Str(s) = value {
                        this.rename_all = Inflection::parse(&s.value());
                    }
                } else if meta.path.is_ident("skip") {
                    this.skip = true;
                } else if meta.path.is_ident("untagged") {
                    this.untagged = true;
                } else {
                    let _ = meta.value().and_then(|v| v.parse::<Lit>()).ok();
                }
                Ok(())
            })?;
        }

        Ok(this)
    }
}
