//! Typed representation of the Flow parser AST.
//!
//! Comprehensive coverage of all Flow type system nodes. Unrecognized nodes
//! deserialize as `Other` via `#[serde(other)]` for forward compatibility.
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
    pub loc: Option<SourceLocation>,
}

/// Source location attached to a parse error.
#[derive(Debug, Clone, Deserialize)]
pub struct SourceLocation {
    pub start: Position,
    pub end: Position,
}

/// Line and column (both 1-based for line, 0-based for column, matching Flow convention).
#[derive(Debug, Clone, Deserialize)]
pub struct Position {
    pub line: u32,
    pub column: u32,
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
    ExportDefaultDeclaration {
        #[serde(default)]
        declaration: Option<serde_json::Value>,
    },
    EnumDeclaration {
        id: Identifier,
        body: EnumBody,
    },
    ImportDeclaration {
        #[serde(default)]
        source: Option<serde_json::Value>,
        #[serde(default)]
        specifiers: Vec<serde_json::Value>,
        #[serde(rename = "importKind", default)]
        import_kind: Option<String>,
    },
    DeclareModuleExports {
        #[serde(rename = "typeAnnotation")]
        type_annotation: Box<TypeAnnotation>,
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
    EnumDeclaration {
        id: Identifier,
        body: EnumBody,
    },
    InterfaceDeclaration {
        id: Identifier,
        #[serde(default)]
        extends: Vec<InterfaceExtends>,
        body: TypeAnnotation,
    },
    DeclareTypeAlias {
        id: Identifier,
        right: TypeAnnotation,
    },
    DeclareClass {
        id: Identifier,
    },
    DeclareFunction {
        id: Identifier,
    },
    DeclareVariable {
        id: Identifier,
    },
    DeclareModule {
        #[serde(default)]
        id: serde_json::Value,
        #[serde(default)]
        body: serde_json::Value,
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
    BigIntLiteralTypeAnnotation {
        #[serde(default)]
        value: Option<serde_json::Value>,
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
        #[serde(rename = "callProperties", default)]
        call_properties: Vec<ObjectTypeCallProperty>,
        #[serde(rename = "internalSlots", default)]
        internal_slots: Vec<ObjectTypeInternalSlot>,
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

    // Named type reference (Foo, $ReadOnlyArray<T>, React.Node)
    GenericTypeAnnotation {
        id: TypeIdentifier,
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

    // Function type ((x: A) => B)
    FunctionTypeAnnotation {
        #[serde(default)]
        params: Vec<FunctionTypeParam>,
        #[serde(rename = "returnType")]
        return_type: Box<TypeAnnotation>,
        #[serde(default)]
        rest: Option<Box<FunctionTypeParam>>,
        #[serde(rename = "typeParameters", default)]
        type_parameters: Option<TypeParameterDeclaration>,
        #[serde(rename = "this", default)]
        this_constraint: Option<Box<FunctionTypeParam>>,
    },

    // Interface type (inline anonymous interface)
    InterfaceTypeAnnotation {
        #[serde(default)]
        extends: Vec<InterfaceExtends>,
        body: Box<TypeAnnotation>,
    },

    // Existential type (*)
    ExistsTypeAnnotation,

    // Indexed access (Obj['key'])
    IndexedAccessType {
        #[serde(rename = "objectType")]
        object_type: Box<TypeAnnotation>,
        #[serde(rename = "indexType")]
        index_type: Box<TypeAnnotation>,
    },

    // Optional indexed access (Obj?.['key'])
    OptionalIndexedAccessType {
        #[serde(rename = "objectType")]
        object_type: Box<TypeAnnotation>,
        #[serde(rename = "indexType")]
        index_type: Box<TypeAnnotation>,
        #[serde(default)]
        optional: bool,
    },

    // keyof T
    KeyofTypeAnnotation {
        argument: Box<TypeAnnotation>,
    },

    // Conditional type (A extends B ? C : D)
    ConditionalTypeAnnotation {
        #[serde(rename = "checkType")]
        check_type: Box<TypeAnnotation>,
        #[serde(rename = "extendsType")]
        extends_type: Box<TypeAnnotation>,
        #[serde(rename = "trueType")]
        true_type: Box<TypeAnnotation>,
        #[serde(rename = "falseType")]
        false_type: Box<TypeAnnotation>,
    },

    // infer T
    InferTypeAnnotation {
        #[serde(rename = "typeParameter", default)]
        type_parameter: Option<serde_json::Value>,
    },

    // Type operator (renders, renders?, renders*)
    TypeOperator {
        #[serde(default)]
        operator: Option<String>,
        #[serde(rename = "typeAnnotation")]
        type_annotation: Box<TypeAnnotation>,
    },

    // Component type
    ComponentTypeAnnotation {
        #[serde(default)]
        params: Vec<serde_json::Value>,
        #[serde(default)]
        rest: Option<serde_json::Value>,
        #[serde(rename = "typeParameters", default)]
        type_parameters: Option<TypeParameterDeclaration>,
        #[serde(rename = "rendersType", default)]
        renders_type: Option<Box<TypeAnnotation>>,
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
            Self::BigIntLiteralTypeAnnotation { .. } => "BigIntLiteralTypeAnnotation",
            Self::NullableTypeAnnotation { .. } => "NullableTypeAnnotation",
            Self::ObjectTypeAnnotation { .. } => "ObjectTypeAnnotation",
            Self::UnionTypeAnnotation { .. } => "UnionTypeAnnotation",
            Self::IntersectionTypeAnnotation { .. } => "IntersectionTypeAnnotation",
            Self::GenericTypeAnnotation { .. } => "GenericTypeAnnotation",
            Self::TupleTypeAnnotation { .. } => "TupleTypeAnnotation",
            Self::ArrayTypeAnnotation { .. } => "ArrayTypeAnnotation",
            Self::TypeofTypeAnnotation { .. } => "TypeofTypeAnnotation",
            Self::FunctionTypeAnnotation { .. } => "FunctionTypeAnnotation",
            Self::InterfaceTypeAnnotation { .. } => "InterfaceTypeAnnotation",
            Self::ExistsTypeAnnotation => "ExistsTypeAnnotation",
            Self::IndexedAccessType { .. } => "IndexedAccessType",
            Self::OptionalIndexedAccessType { .. } => "OptionalIndexedAccessType",
            Self::KeyofTypeAnnotation { .. } => "KeyofTypeAnnotation",
            Self::ConditionalTypeAnnotation { .. } => "ConditionalTypeAnnotation",
            Self::InferTypeAnnotation { .. } => "InferTypeAnnotation",
            Self::TypeOperator { .. } => "TypeOperator",
            Self::ComponentTypeAnnotation { .. } => "ComponentTypeAnnotation",
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

/// A call property on an object type (`{ (x: number): string }`).
#[derive(Debug, Clone, Deserialize)]
pub struct ObjectTypeCallProperty {
    pub value: TypeAnnotation,
    #[serde(rename = "static", default)]
    pub is_static: bool,
}

/// An internal slot on an object type (`{ [[call]](x: number): string }`).
#[derive(Debug, Clone, Deserialize)]
pub struct ObjectTypeInternalSlot {
    pub id: Identifier,
    pub value: TypeAnnotation,
    #[serde(default)]
    pub optional: bool,
    #[serde(rename = "static", default)]
    pub is_static: bool,
    #[serde(default)]
    pub method: bool,
}

// ── Function type param ─────────────────────────────────────────────────

/// A parameter in a function type annotation.
#[derive(Debug, Clone, Deserialize)]
pub struct FunctionTypeParam {
    #[serde(default)]
    pub name: Option<Identifier>,
    #[serde(rename = "typeAnnotation")]
    pub type_annotation: TypeAnnotation,
    #[serde(default)]
    pub optional: bool,
}

// ── Interface extends ───────────────────────────────────────────────────

/// An `extends` clause in an interface declaration or type.
#[derive(Debug, Clone, Deserialize)]
pub struct InterfaceExtends {
    pub id: Identifier,
    #[serde(rename = "typeParameters", default)]
    pub type_parameters: Option<TypeParameterInstantiation>,
}

// ── Enum types ──────────────────────────────────────────────────────────

/// Body of a Flow enum declaration.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum EnumBody {
    EnumStringBody {
        members: Vec<EnumMember>,
        #[serde(rename = "explicitType", default)]
        explicit_type: bool,
        #[serde(rename = "hasUnknownMembers", default)]
        has_unknown_members: bool,
    },
    EnumNumberBody {
        members: Vec<EnumMember>,
        #[serde(rename = "explicitType", default)]
        explicit_type: bool,
        #[serde(rename = "hasUnknownMembers", default)]
        has_unknown_members: bool,
    },
    EnumBooleanBody {
        members: Vec<EnumMember>,
        #[serde(rename = "explicitType", default)]
        explicit_type: bool,
        #[serde(rename = "hasUnknownMembers", default)]
        has_unknown_members: bool,
    },
    EnumSymbolBody {
        members: Vec<EnumMember>,
        #[serde(rename = "hasUnknownMembers", default)]
        has_unknown_members: bool,
    },
    EnumBigIntBody {
        members: Vec<EnumMember>,
        #[serde(rename = "explicitType", default)]
        explicit_type: bool,
        #[serde(rename = "hasUnknownMembers", default)]
        has_unknown_members: bool,
    },
    #[serde(other)]
    Other,
}

impl EnumBody {
    /// Short type tag for assertions.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::EnumStringBody { .. } => "EnumStringBody",
            Self::EnumNumberBody { .. } => "EnumNumberBody",
            Self::EnumBooleanBody { .. } => "EnumBooleanBody",
            Self::EnumSymbolBody { .. } => "EnumSymbolBody",
            Self::EnumBigIntBody { .. } => "EnumBigIntBody",
            Self::Other => "Other",
        }
    }

    /// Members of the enum body.
    pub fn members(&self) -> &[EnumMember] {
        match self {
            Self::EnumStringBody { members, .. }
            | Self::EnumNumberBody { members, .. }
            | Self::EnumBooleanBody { members, .. }
            | Self::EnumSymbolBody { members, .. }
            | Self::EnumBigIntBody { members, .. } => members,
            Self::Other => &[],
        }
    }
}

/// A member of a Flow enum body.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum EnumMember {
    /// Defaulted member (symbol enum, or auto-initialized).
    EnumDefaultedMember {
        id: Identifier,
    },
    /// String-initialized member.
    EnumStringMember {
        id: Identifier,
        init: EnumStringInit,
    },
    /// Number-initialized member.
    EnumNumberMember {
        id: Identifier,
        init: EnumNumberInit,
    },
    /// Boolean-initialized member.
    EnumBooleanMember {
        id: Identifier,
        init: EnumBooleanInit,
    },
    #[serde(other)]
    Other,
}

impl EnumMember {
    /// Name of the enum member, if known.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::EnumDefaultedMember { id }
            | Self::EnumStringMember { id, .. }
            | Self::EnumNumberMember { id, .. }
            | Self::EnumBooleanMember { id, .. } => Some(&id.name),
            Self::Other => None,
        }
    }
}

/// String literal init value in a Flow enum member.
#[derive(Debug, Clone, Deserialize)]
pub struct EnumStringInit {
    pub value: String,
}

/// Number literal init value in a Flow enum member.
#[derive(Debug, Clone, Deserialize)]
pub struct EnumNumberInit {
    pub value: f64,
}

/// Boolean literal init value in a Flow enum member.
#[derive(Debug, Clone, Deserialize)]
pub struct EnumBooleanInit {
    pub value: bool,
}

// ── Shared nodes ────────────────────────────────────────────────────────

/// An identifier node.
#[derive(Debug, Clone, Deserialize)]
pub struct Identifier {
    pub name: String,
}

/// A type identifier — either a simple `Identifier` or a qualified `A.B` reference.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum TypeIdentifier {
    Identifier {
        name: String,
    },
    QualifiedTypeIdentifier {
        qualification: Box<TypeIdentifier>,
        id: Identifier,
    },
    #[serde(other)]
    Other,
}

impl TypeIdentifier {
    /// The leaf name of this identifier (e.g. `"Node"` for `React.Node`).
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Identifier { name } => Some(name),
            Self::QualifiedTypeIdentifier { id, .. } => Some(&id.name),
            Self::Other => None,
        }
    }

    /// Full dotted name (e.g. `"React.Node"`).
    pub fn full_name(&self) -> String {
        match self {
            Self::Identifier { name } => name.clone(),
            Self::QualifiedTypeIdentifier { qualification, id } => {
                format!("{}.{}", qualification.full_name(), id.name)
            }
            Self::Other => "?".to_owned(),
        }
    }
}

/// Property key — either an identifier or a string literal (for quoted keys).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum PropertyKey {
    Identifier {
        name: String,
    },
    /// Quoted key — Flow parser emits `Literal` (not `StringLiteral`) for property keys.
    Literal {
        value: serde_json::Value,
    },
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

/// Type parameter declaration (`<T: Bound, U = Default>`).
#[derive(Debug, Clone, Deserialize)]
pub struct TypeParameterDeclaration {
    pub params: Vec<TypeParameter>,
}

/// A single type parameter in a declaration.
#[derive(Debug, Clone, Deserialize)]
pub struct TypeParameter {
    pub name: String,
    /// Bound is wrapped: `{"type": "TypeAnnotation", "typeAnnotation": <actual>}`
    #[serde(default)]
    pub bound: Option<TypeAnnotationWrapper>,
    #[serde(default)]
    pub default: Option<Box<TypeAnnotation>>,
    #[serde(default)]
    pub variance: Option<Variance>,
}

/// Wrapper node the Flow parser emits around type annotations in certain positions
/// (e.g., type parameter bounds, function parameter types).
#[derive(Debug, Clone, Deserialize)]
pub struct TypeAnnotationWrapper {
    #[serde(rename = "typeAnnotation")]
    pub type_annotation: TypeAnnotation,
}
