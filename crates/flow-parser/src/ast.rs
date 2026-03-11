//! Typed representation of the Flow parser AST.
//!
//! Coverage: primitives, literals, nullable, object, union, intersection,
//! generic, tuple, array, typeof, opaque. Unrecognized nodes deserialize as
//! `Other` via `#[serde(other)]`.
//!
//! Future: complete the AST to remove all `Other` fallbacks. Missing variants:
//!   - TypeAnnotation: FunctionTypeAnnotation (params, return, rest, type params),
//!     InterfaceTypeAnnotation, ExistsTypeAnnotation (*), IndexedAccessType,
//!     OptionalIndexedAccessType, KeyofTypeAnnotation, ConditionalTypeAnnotation,
//!     InferTypeAnnotation, TypeOperator, ComponentTypeAnnotation
//!   - Declaration: InterfaceDeclaration, DeclareTypeAlias, DeclareClass,
//!     DeclareFunction, DeclareVariable, DeclareModule
//!   - Statement: ImportDeclaration, DeclareModuleExports, TypeCastExpression
//!   - PropertyKey: NumberLiteral (numeric keys)
//!   - ObjectMember: ObjectTypeInternalSlot, ObjectTypeCallProperty
//!   - Positional info: loc (SourceLocation), range ([start, end]) on all nodes
//!
//! Reference: https://github.com/facebook/flow/blob/main/src/parser/estree_translator.ml

use serde::Deserialize;

// ── Program ─────────────────────────────────────────────────────────────

/// Root AST node returned by the Flow parser.
#[derive(Debug, Clone, Deserialize)]
pub struct Program {
    pub body: Vec<Statement>,
    #[serde(default)]
    pub errors: Vec<ParseError>,
}

/// A parse error reported by the Flow parser.
#[derive(Debug, Clone, Deserialize)]
pub struct ParseError {
    pub message: String,
}

// ── Statements ──────────────────────────────────────────────────────────

/// Top-level statement in a Flow program.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Statement {
    ExportNamedDeclaration {
        declaration: Option<Declaration>,
    },
    DeclareExportDeclaration {
        declaration: Option<Declaration>,
    },
    /// Catch-all for statement types we don't inspect.
    #[serde(other)]
    Other,
}

// ── Declarations ────────────────────────────────────────────────────────

/// A type-level declaration inside an export statement.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Declaration {
    TypeAlias {
        id: Identifier,
        right: TypeAnnotation,
    },
    OpaqueType {
        id: Identifier,
        supertype: Option<TypeAnnotation>,
    },
    DeclareOpaqueType {
        id: Identifier,
        supertype: Option<TypeAnnotation>,
    },
    #[serde(other)]
    Other,
}

// ── Type annotations ────────────────────────────────────────────────────

/// A Flow type annotation node.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum TypeAnnotation {
    // Primitives
    StringTypeAnnotation,
    NumberTypeAnnotation,
    BooleanTypeAnnotation,
    VoidTypeAnnotation,
    MixedTypeAnnotation,
    AnyTypeAnnotation,
    EmptyTypeAnnotation,
    NullLiteralTypeAnnotation,
    BigIntTypeAnnotation,
    SymbolTypeAnnotation,

    // Literals
    StringLiteralTypeAnnotation {
        value: String,
    },
    NumberLiteralTypeAnnotation {
        value: f64,
    },
    BooleanLiteralTypeAnnotation {
        value: bool,
    },

    // Nullable (?T)
    NullableTypeAnnotation {
        #[serde(rename = "typeAnnotation")]
        type_annotation: Box<TypeAnnotation>,
    },

    // Object ({| +key: T |})
    ObjectTypeAnnotation {
        #[serde(default)]
        properties: Vec<ObjectMember>,
        #[serde(default)]
        indexers: Vec<ObjectTypeIndexer>,
        #[serde(default)]
        exact: bool,
    },

    // Union (A | B)
    UnionTypeAnnotation {
        types: Vec<TypeAnnotation>,
    },

    // Intersection (A & B)
    IntersectionTypeAnnotation {
        types: Vec<TypeAnnotation>,
    },

    // Named type reference (Foo, $ReadOnlyArray<T>)
    GenericTypeAnnotation {
        id: Identifier,
        #[serde(rename = "typeParameters")]
        type_parameters: Option<TypeParameterInstantiation>,
    },

    // Tuple ([A, B, C])
    TupleTypeAnnotation {
        #[serde(alias = "types", default)]
        #[serde(rename = "elementTypes")]
        element_types: Vec<TypeAnnotation>,
    },

    // Array (T[])
    ArrayTypeAnnotation {
        #[serde(rename = "elementType")]
        element_type: Box<TypeAnnotation>,
    },

    // typeof T
    TypeofTypeAnnotation {
        argument: Box<TypeAnnotation>,
    },

    #[serde(other)]
    Other,
}

impl TypeAnnotation {
    /// Short type tag for assertions (e.g. `"StringTypeAnnotation"`).
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::StringTypeAnnotation => "StringTypeAnnotation",
            Self::NumberTypeAnnotation => "NumberTypeAnnotation",
            Self::BooleanTypeAnnotation => "BooleanTypeAnnotation",
            Self::VoidTypeAnnotation => "VoidTypeAnnotation",
            Self::MixedTypeAnnotation => "MixedTypeAnnotation",
            Self::AnyTypeAnnotation => "AnyTypeAnnotation",
            Self::EmptyTypeAnnotation => "EmptyTypeAnnotation",
            Self::NullLiteralTypeAnnotation => "NullLiteralTypeAnnotation",
            Self::BigIntTypeAnnotation => "BigIntTypeAnnotation",
            Self::SymbolTypeAnnotation => "SymbolTypeAnnotation",
            Self::StringLiteralTypeAnnotation { .. } => "StringLiteralTypeAnnotation",
            Self::NumberLiteralTypeAnnotation { .. } => "NumberLiteralTypeAnnotation",
            Self::BooleanLiteralTypeAnnotation { .. } => "BooleanLiteralTypeAnnotation",
            Self::NullableTypeAnnotation { .. } => "NullableTypeAnnotation",
            Self::ObjectTypeAnnotation { .. } => "ObjectTypeAnnotation",
            Self::UnionTypeAnnotation { .. } => "UnionTypeAnnotation",
            Self::IntersectionTypeAnnotation { .. } => "IntersectionTypeAnnotation",
            Self::GenericTypeAnnotation { .. } => "GenericTypeAnnotation",
            Self::TupleTypeAnnotation { .. } => "TupleTypeAnnotation",
            Self::ArrayTypeAnnotation { .. } => "ArrayTypeAnnotation",
            Self::TypeofTypeAnnotation { .. } => "TypeofTypeAnnotation",
            Self::Other => "Other",
        }
    }
}

// ── Object type members ─────────────────────────────────────────────────

/// A member of an object type annotation.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ObjectMember {
    ObjectTypeProperty {
        key: PropertyKey,
        value: TypeAnnotation,
        variance: Option<Variance>,
        #[serde(default)]
        optional: bool,
    },
    ObjectTypeSpreadProperty {
        argument: TypeAnnotation,
    },
    #[serde(other)]
    Other,
}

/// An indexer on an object type (`[key: K]: V`).
#[derive(Debug, Clone, Deserialize)]
pub struct ObjectTypeIndexer {
    pub key: TypeAnnotation,
    pub value: TypeAnnotation,
}

// ── Shared nodes ────────────────────────────────────────────────────────

/// An identifier node.
#[derive(Debug, Clone, Deserialize)]
pub struct Identifier {
    pub name: String,
}

/// Property key — either an identifier or a string literal (for quoted keys).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum PropertyKey {
    Identifier { name: String },
    /// Quoted key — Flow parser emits `Literal` (not `StringLiteral`) for property keys.
    Literal { value: serde_json::Value },
    #[serde(other)]
    Other,
}

impl PropertyKey {
    /// The key's text regardless of representation.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Identifier { name } => Some(name),
            Self::Literal { value } => value.as_str(),
            Self::Other => None,
        }
    }

    /// Whether this key is a quoted literal (non-identifier).
    pub fn is_quoted(&self) -> bool {
        matches!(self, Self::Literal { .. })
    }
}

/// Variance annotation (`+` = plus, `-` = minus).
#[derive(Debug, Clone, Deserialize)]
pub struct Variance {
    pub kind: VarianceKind,
}

/// Variance direction.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VarianceKind {
    Plus,
    Minus,
}

/// Type parameter instantiation (`<T, U>`).
#[derive(Debug, Clone, Deserialize)]
pub struct TypeParameterInstantiation {
    pub params: Vec<TypeAnnotation>,
}
