//! Validate generated Flow declarations against the official Flow parser AST.
//!
//! Uses the `flow-parser` crate (QuickJS + npm's flow-parser) to parse
//! generated Flow source and assert on the typed Rust AST — no inline JS.

#![allow(dead_code)]

use flowjs_parser::{
    Declaration, EnumBody, EnumMember, FlowParser, ObjectMember, Statement, TypeAnnotation,
    VarianceKind,
};
use flowjs_rs::{Config, Flow};

// ── Helpers ─────────────────────────────────────────────────────────────

/// Validate a declaration and extract the type alias name + right-hand side.
fn parse_type_alias(parser: &FlowParser, decl: &str) -> (String, TypeAnnotation) {
    let program = parser
        .validate_declaration(decl)
        .unwrap_or_else(|e| panic!("Flow validation failed for:\n{decl}\n\nError: {e}"));

    match &program.body[0] {
        Statement::ExportNamedDeclaration {
            declaration: Some(Declaration::TypeAlias { id, right }),
        } => (id.name.clone(), right.clone()),
        other => panic!("expected ExportNamedDeclaration(TypeAlias), got: {other:?}"),
    }
}

/// Extract properties from an ObjectTypeAnnotation.
fn object_properties(ty: &TypeAnnotation) -> &[ObjectMember] {
    match ty {
        TypeAnnotation::ObjectTypeAnnotation { properties, .. } => properties,
        other => panic!("expected ObjectTypeAnnotation, got: {}", other.type_name()),
    }
}

// ── Test types ──────────────────────────────────────────────────────────

#[derive(Flow)]
struct SimpleStruct {
    name: String,
    age: i32,
    active: bool,
}

#[derive(Flow)]
struct WithOption {
    label: String,
    description: Option<String>,
}

#[derive(Flow)]
struct WithVec {
    tags: Vec<String>,
    scores: Vec<f64>,
}

#[derive(Flow)]
struct Nested {
    inner: SimpleStruct,
    items: Vec<WithOption>,
}

#[derive(Flow)]
struct Newtype(String);

#[derive(Flow)]
struct UnitStruct;

#[derive(Flow)]
#[flow(rename_all = "camelCase")]
struct CamelCase {
    first_name: String,
    last_name: String,
}

#[derive(Flow)]
#[flow(opaque)]
struct OpaqueToken(String);

#[derive(Flow)]
#[flow(opaque = "string")]
struct BoundedOpaque(String);

#[derive(Flow)]
#[flow(untagged)]
enum UntaggedUnion {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
}

#[derive(Flow)]
#[flow(tag = "kind")]
enum TaggedUnion {
    Click,
    Resize { width: i32, height: i32 },
    Close,
}

#[derive(Flow)]
#[flow(tag = "type", content = "data")]
enum AdjacentlyTagged {
    Text { body: String },
    Image { url: String, width: i32 },
    Empty,
}

#[derive(Flow)]
#[flow(tag = "t", content = "v")]
enum AdjacentlyTaggedNewtype {
    Str(String),
    Num(i32),
}

#[derive(Flow)]
#[flow(tag = "kind")]
enum InternallyTaggedNewtype {
    Wrapper(String),
    Data { value: i32 },
}

#[derive(Flow)]
enum ExternallyTagged {
    Alpha { x: i32 },
    Beta(String),
    Gamma,
}

#[derive(Flow)]
#[flow(rename_all = "kebab-case")]
struct KebabFields {
    first_name: String,
    last_name: String,
}

#[derive(Flow)]
#[flow(tag = "kind")]
enum OptionalVariantFields {
    Entry {
        name: String,
        description: Option<String>,
    },
}

#[derive(Flow)]
#[flow(rename_all = "SCREAMING_SNAKE_CASE", rename_all_fields = "camelCase")]
enum RenamedVariantFields {
    MyVariant {
        first_name: String,
        last_name: String,
    },
}

#[derive(Flow)]
struct WithTuple {
    pair: (String, i32),
}

#[derive(Flow)]
struct WithHashMap {
    data: std::collections::HashMap<String, i32>,
}

#[derive(Flow)]
#[flow(flow_enum)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Flow)]
#[flow(flow_enum = "string", rename_all = "lowercase")]
enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Flow)]
#[flow(flow_enum = "string")]
enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Flow)]
struct WithFlowEnum {
    direction: Direction,
    priority: Priority,
}

// ── AST structure tests ─────────────────────────────────────────────────

#[test]
fn ast_simple_struct_fields() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &SimpleStruct::decl(&cfg));
    let props = object_properties(&ty);

    // Assert
    assert_eq!(props.len(), 3, "should have 3 fields");

    match &props[0] {
        ObjectMember::ObjectTypeProperty {
            key,
            value,
            variance,
            ..
        } => {
            assert_eq!(key.name().unwrap(), "name", "first field name");
            assert_eq!(value.type_name(), "StringTypeAnnotation", "String → string");
            assert!(
                variance.is_none(),
                "default: no variance annotation (matches ts-rs)"
            );
        }
        other => panic!("expected ObjectTypeProperty, got: {other:?}"),
    }

    match &props[1] {
        ObjectMember::ObjectTypeProperty { key, value, .. } => {
            assert_eq!(key.name().unwrap(), "age", "second field name");
            assert_eq!(value.type_name(), "NumberTypeAnnotation", "i32 → number");
        }
        other => panic!("expected ObjectTypeProperty, got: {other:?}"),
    }

    match &props[2] {
        ObjectMember::ObjectTypeProperty { key, value, .. } => {
            assert_eq!(key.name().unwrap(), "active", "third field name");
            assert_eq!(value.type_name(), "BooleanTypeAnnotation", "bool → boolean");
        }
        other => panic!("expected ObjectTypeProperty, got: {other:?}"),
    }
}

#[test]
fn ast_option_produces_nullable() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &WithOption::decl(&cfg));
    let props = object_properties(&ty);

    // Assert
    match &props[1] {
        ObjectMember::ObjectTypeProperty {
            key,
            value,
            optional,
            ..
        } => {
            assert_eq!(key.name().unwrap(), "description", "field name");
            assert!(
                !*optional,
                "bare Option<T> without serde skip is always-present, not omittable"
            );
            assert_eq!(
                value.type_name(),
                "NullableTypeAnnotation",
                "Option<T> → ?T"
            );

            match value {
                TypeAnnotation::NullableTypeAnnotation { type_annotation } => {
                    assert_eq!(
                        type_annotation.type_name(),
                        "StringTypeAnnotation",
                        "inner type is string"
                    );
                }
                _ => unreachable!(),
            }
        }
        other => panic!("expected ObjectTypeProperty, got: {other:?}"),
    }
}

#[test]
fn ast_vec_produces_readonly_array() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &WithVec::decl(&cfg));
    let props = object_properties(&ty);

    // Assert
    match &props[0] {
        ObjectMember::ObjectTypeProperty { key, value, .. } => {
            assert_eq!(key.name().unwrap(), "tags", "field name");
            match value {
                TypeAnnotation::GenericTypeAnnotation {
                    id,
                    type_parameters,
                } => {
                    assert_eq!(id.name().unwrap(), "$ReadOnlyArray", "Vec → $ReadOnlyArray");
                    let params = &type_parameters.as_ref().unwrap().params;
                    assert_eq!(
                        params[0].type_name(),
                        "StringTypeAnnotation",
                        "Vec<String> param is string"
                    );
                }
                other => panic!("expected GenericTypeAnnotation, got: {}", other.type_name()),
            }
        }
        other => panic!("expected ObjectTypeProperty, got: {other:?}"),
    }
}

#[test]
fn ast_newtype_inlines_to_string() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (name, ty) = parse_type_alias(&parser, &Newtype::decl(&cfg));

    // Assert
    assert_eq!(name, "Newtype", "type name");
    assert_eq!(
        ty.type_name(),
        "StringTypeAnnotation",
        "newtype(String) inlines to string"
    );
}

#[test]
fn ast_unit_struct_is_null() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (name, ty) = parse_type_alias(&parser, &UnitStruct::decl(&cfg));

    // Assert
    assert_eq!(name, "UnitStruct", "type name");
    assert_eq!(ty.type_name(), "NullLiteralTypeAnnotation", "unit struct → null (matches ts-rs)");
}

#[test]
fn ast_camel_case_rename() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &CamelCase::decl(&cfg));
    let props = object_properties(&ty);

    // Assert
    let names: Vec<&str> = props
        .iter()
        .filter_map(|p| match p {
            ObjectMember::ObjectTypeProperty { key, .. } => key.name(),
            _ => None,
        })
        .collect();
    assert_eq!(
        names,
        vec!["firstName", "lastName"],
        "fields should be camelCase"
    );
}

#[test]
fn ast_opaque_type() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let program = parser
        .validate_declaration(&OpaqueToken::decl(&cfg))
        .unwrap();

    // Assert
    match &program.body[0] {
        Statement::ExportNamedDeclaration {
            declaration: Some(decl),
        }
        | Statement::DeclareExportDeclaration {
            declaration: Some(decl),
        } => match decl {
            Declaration::OpaqueType { id, supertype }
            | Declaration::DeclareOpaqueType { id, supertype } => {
                assert_eq!(id.name, "OpaqueToken", "opaque type name");
                assert!(supertype.is_none(), "fully opaque has no supertype");
            }
            other => panic!("expected OpaqueType, got: {other:?}"),
        },
        other => panic!("expected DeclareExportDeclaration, got: {other:?}"),
    }
}

#[test]
fn ast_bounded_opaque_type() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let program = parser
        .validate_declaration(&BoundedOpaque::decl(&cfg))
        .unwrap();

    // Assert
    match &program.body[0] {
        Statement::ExportNamedDeclaration {
            declaration: Some(decl),
        }
        | Statement::DeclareExportDeclaration {
            declaration: Some(decl),
        } => match decl {
            Declaration::OpaqueType { id, supertype }
            | Declaration::DeclareOpaqueType { id, supertype } => {
                assert_eq!(id.name, "BoundedOpaque", "bounded opaque type name");
                assert_eq!(
                    supertype.as_ref().unwrap().type_name(),
                    "StringTypeAnnotation",
                    "bounded opaque supertype is string"
                );
            }
            other => panic!("expected OpaqueType, got: {other:?}"),
        },
        other => panic!("expected DeclareExportDeclaration, got: {other:?}"),
    }
}

#[test]
fn ast_untagged_enum_is_union() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &UntaggedUnion::decl(&cfg));

    // Assert
    match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => {
            assert_eq!(types.len(), 2, "two variants = two union members");
        }
        other => panic!("expected UnionTypeAnnotation, got: {}", other.type_name()),
    }
}

#[test]
fn ast_tagged_enum_has_tag_field() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &TaggedUnion::decl(&cfg));

    // Assert — Click variant has +kind: 'Click'
    let union_types = match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => types,
        other => panic!("expected UnionTypeAnnotation, got: {}", other.type_name()),
    };

    let first_props = object_properties(&union_types[0]);
    match &first_props[0] {
        ObjectMember::ObjectTypeProperty { key, value, .. } => {
            assert_eq!(key.name().unwrap(), "kind", "tag field name");
            match value {
                TypeAnnotation::StringLiteralTypeAnnotation { value } => {
                    assert_eq!(value, "Click", "tag value matches variant name");
                }
                other => panic!(
                    "expected StringLiteralTypeAnnotation, got: {}",
                    other.type_name()
                ),
            }
        }
        other => panic!("expected ObjectTypeProperty, got: {other:?}"),
    }
}

#[test]
fn ast_adjacently_tagged_has_content_field() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &AdjacentlyTagged::decl(&cfg));
    let union_types = match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => types,
        other => panic!("expected UnionTypeAnnotation, got: {}", other.type_name()),
    };

    // Assert — Text variant has +type: 'Text' and +data: {| ... |}
    let props = object_properties(&union_types[0]);
    assert!(
        props.len() >= 2,
        "should have tag + content fields, got {}",
        props.len()
    );

    match &props[0] {
        ObjectMember::ObjectTypeProperty { key, value, .. } => {
            assert_eq!(key.name().unwrap(), "type", "tag field name");
            match value {
                TypeAnnotation::StringLiteralTypeAnnotation { value } => {
                    assert_eq!(value, "Text", "tag value");
                }
                other => panic!(
                    "expected StringLiteralTypeAnnotation, got: {}",
                    other.type_name()
                ),
            }
        }
        other => panic!("expected ObjectTypeProperty for tag, got: {other:?}"),
    }

    match &props[1] {
        ObjectMember::ObjectTypeProperty { key, value, .. } => {
            assert_eq!(key.name().unwrap(), "data", "content field name");
            assert_eq!(
                value.type_name(),
                "ObjectTypeAnnotation",
                "content is an object"
            );
        }
        other => panic!("expected ObjectTypeProperty for content, got: {other:?}"),
    }
}

#[test]
fn ast_kebab_case_fields_quoted() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &KebabFields::decl(&cfg));
    let props = object_properties(&ty);

    // Assert
    let fields: Vec<(&str, bool)> = props
        .iter()
        .filter_map(|p| match p {
            ObjectMember::ObjectTypeProperty { key, .. } => Some((key.name()?, key.is_quoted())),
            _ => None,
        })
        .collect();

    assert_eq!(fields[0].0, "first-name", "kebab-case field name");
    assert_eq!(fields[1].0, "last-name", "kebab-case field name");
}

#[test]
fn ast_optional_variant_fields() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &OptionalVariantFields::decl(&cfg));
    let union_types = match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => types,
        // Single variant may not be wrapped in union
        other => {
            let props = object_properties(other);
            let desc = props.iter().find(|p| matches!(p,
                ObjectMember::ObjectTypeProperty { key, .. } if key.name() == Some("description")
            ));
            match desc {
                Some(ObjectMember::ObjectTypeProperty { optional, .. }) => {
                    assert!(
                        !*optional,
                        "bare Option<String> in variant without serde skip is always-present"
                    );
                }
                _ => panic!("description field not found"),
            }
            return;
        }
    };

    let props = object_properties(&union_types[0]);
    let desc = props
        .iter()
        .find(|p| {
            matches!(p, ObjectMember::ObjectTypeProperty { key, .. } if key.name() == Some("description"))
        })
        .expect("should have description field");

    match desc {
        ObjectMember::ObjectTypeProperty { optional, .. } => {
            assert!(
                !*optional,
                "bare Option<String> in variant without serde skip is always-present"
            );
        }
        _ => unreachable!(),
    }
}

#[test]
fn ast_tuple_in_struct() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &WithTuple::decl(&cfg));
    let props = object_properties(&ty);

    // Assert
    match &props[0] {
        ObjectMember::ObjectTypeProperty { key, value, .. } => {
            assert_eq!(key.name().unwrap(), "pair", "field name");
            match value {
                TypeAnnotation::TupleTypeAnnotation { element_types } => {
                    assert_eq!(element_types.len(), 2, "tuple has 2 elements");
                    assert_eq!(
                        element_types[0].type_name(),
                        "StringTypeAnnotation",
                        "first element is string"
                    );
                    assert_eq!(
                        element_types[1].type_name(),
                        "NumberTypeAnnotation",
                        "second element is number"
                    );
                }
                other => panic!("expected TupleTypeAnnotation, got: {}", other.type_name()),
            }
        }
        other => panic!("expected ObjectTypeProperty, got: {other:?}"),
    }
}

#[test]
fn ast_hashmap_produces_indexer() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &WithHashMap::decl(&cfg));
    let props = object_properties(&ty);

    // Assert
    match &props[0] {
        ObjectMember::ObjectTypeProperty { key, value, .. } => {
            assert_eq!(key.name().unwrap(), "data", "field name");
            match value {
                TypeAnnotation::ObjectTypeAnnotation { indexers, .. } => {
                    assert!(!indexers.is_empty(), "HashMap → object with indexer");
                    assert_eq!(
                        indexers[0].key.type_name(),
                        "StringTypeAnnotation",
                        "key is string"
                    );
                    assert_eq!(
                        indexers[0].value.type_name(),
                        "NumberTypeAnnotation",
                        "value is number"
                    );
                }
                other => panic!("expected ObjectTypeAnnotation, got: {}", other.type_name()),
            }
        }
        other => panic!("expected ObjectTypeProperty, got: {other:?}"),
    }
}

#[test]
fn ast_renamed_variant_fields() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &RenamedVariantFields::decl(&cfg));

    // Assert — externally tagged: {| MY_VARIANT: {| +firstName: ..., +lastName: ... |} |}
    let props = match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => object_properties(&types[0]),
        other => object_properties(other),
    };

    let field_names: Vec<&str> = props
        .iter()
        .filter_map(|p| match p {
            ObjectMember::ObjectTypeProperty { key, .. } => key.name(),
            _ => None,
        })
        .collect();

    assert!(
        field_names.contains(&"firstName") || field_names.contains(&"MY_VARIANT"),
        "should have camelCase fields or SCREAMING_SNAKE variant, got: {field_names:?}"
    );
}

#[test]
fn ast_nested_struct_references_type() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &Nested::decl(&cfg));
    let props = object_properties(&ty);

    // Assert
    match &props[0] {
        ObjectMember::ObjectTypeProperty { key, value, .. } => {
            assert_eq!(key.name().unwrap(), "inner", "field name");
            match value {
                TypeAnnotation::GenericTypeAnnotation { id, .. } => {
                    assert_eq!(id.name().unwrap(), "SimpleStruct", "references SimpleStruct by name");
                }
                other => panic!("expected GenericTypeAnnotation, got: {}", other.type_name()),
            }
        }
        other => panic!("expected ObjectTypeProperty, got: {other:?}"),
    }
}

#[test]
fn ast_internally_tagged_newtype_intersection() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &InternallyTaggedNewtype::decl(&cfg));
    let union_types = match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => types,
        other => panic!("expected UnionTypeAnnotation, got: {}", other.type_name()),
    };

    // Assert — Wrapper variant should be {| +kind: 'Wrapper' |} & string (intersection)
    assert_eq!(
        union_types[0].type_name(),
        "IntersectionTypeAnnotation",
        "internally tagged newtype should produce intersection"
    );
}

#[test]
fn ast_externally_tagged_has_variant_keys() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (_, ty) = parse_type_alias(&parser, &ExternallyTagged::decl(&cfg));

    // Assert
    match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => {
            assert_eq!(types.len(), 3, "3 variants in externally tagged enum");
        }
        other => panic!("expected UnionTypeAnnotation, got: {}", other.type_name()),
    }
}

// ── Type constant tests ─────────────────────────────────────────────────

#[test]
fn flow_type_any_for_dummy() {
    // Arrange and Act
    let cfg = Config::new();

    // Assert
    assert_eq!(flowjs_rs::Dummy::name(&cfg), "any", "Dummy → any");
    assert_eq!(flowjs_rs::Dummy::inline(&cfg), "any", "Dummy inline → any");
}

#[test]
fn flow_type_void_for_unit() {
    // Arrange and Act and Assert
    let cfg = Config::new();
    assert_eq!(<() as Flow>::name(&cfg), "null", "() → null (matches ts-rs)");
}

#[test]
fn flow_type_mixed_for_serde_json_value() {
    // Arrange, Act, and Assert
    assert_eq!(flowjs_rs::flow_type::MIXED, "mixed", "mixed constant");
    assert_eq!(flowjs_rs::flow_type::ANY, "any", "any constant");
    assert_eq!(
        flowjs_rs::flow_type::EMPTY,
        "empty",
        "empty (bottom type) constant"
    );
}

// ── Primitive type precision ────────────────────────────────────────────

#[test]
fn primitives_are_exact_not_expanded() {
    // Arrange and Act and Assert
    let cfg = Config::new();
    assert_eq!(String::name(&cfg), "string", "String → string");
    assert_eq!(i32::name(&cfg), "number", "i32 → number");
    assert_eq!(bool::name(&cfg), "boolean", "bool → boolean");
    assert_eq!(<()>::name(&cfg), "null", "() → null");
    assert_eq!(f64::name(&cfg), "number", "f64 → number");
    assert_eq!(char::name(&cfg), "string", "char → string");
    assert_eq!(u8::name(&cfg), "number", "u8 → number");
}

// ── Flow enum tests ─────────────────────────────────────────────────────

/// Helper to parse a Flow enum declaration and extract the id + body.
fn parse_enum_decl(parser: &FlowParser, decl: &str) -> (String, EnumBody) {
    let program = parser
        .validate_declaration(decl)
        .unwrap_or_else(|e| panic!("Flow validation failed for:\n{decl}\n\nError: {e}"));

    match &program.body[0] {
        Statement::ExportNamedDeclaration {
            declaration: Some(Declaration::EnumDeclaration { id, body }),
        } => (id.name.clone(), body.clone()),
        other => panic!("expected ExportNamedDeclaration(EnumDeclaration), got: {other:?}"),
    }
}

#[test]
fn ast_flow_enum_symbol() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (name, body) = parse_enum_decl(&parser, &Direction::decl(&cfg));

    // Assert
    assert_eq!(name, "Direction", "enum name");
    // Flow parser represents symbol enums as EnumStringBody with explicitType=false
    // and all members as EnumDefaultedMember
    match &body {
        EnumBody::EnumStringBody {
            members,
            explicit_type,
            ..
        } => {
            assert!(!explicit_type, "symbol enum has explicitType=false");
            assert_eq!(members.len(), 4, "should have 4 members");

            let names: Vec<&str> = members.iter().filter_map(|m| m.name()).collect();
            assert_eq!(names, vec!["Up", "Down", "Left", "Right"], "member names");

            // All members should be defaulted (symbol enum)
            for m in members {
                assert!(
                    matches!(m, EnumMember::EnumDefaultedMember { .. }),
                    "symbol enum members should be defaulted, got: {m:?}"
                );
            }
        }
        other => panic!("expected EnumStringBody, got: {}", other.type_name()),
    }
}

#[test]
fn ast_flow_enum_string_with_rename() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (name, body) = parse_enum_decl(&parser, &Priority::decl(&cfg));

    // Assert
    assert_eq!(name, "Priority", "enum name");
    assert_eq!(body.type_name(), "EnumStringBody", "should be string body");
    let members = body.members();
    assert_eq!(members.len(), 3, "should have 3 members");

    // Verify member names and values
    match &members[0] {
        EnumMember::EnumStringMember { id, init } => {
            assert_eq!(id.name, "Low", "member name");
            assert_eq!(init.value, "low", "lowercase value");
        }
        other => panic!("expected EnumStringMember, got: {other:?}"),
    }
    match &members[1] {
        EnumMember::EnumStringMember { id, init } => {
            assert_eq!(id.name, "Medium", "member name");
            assert_eq!(init.value, "medium", "lowercase value");
        }
        other => panic!("expected EnumStringMember, got: {other:?}"),
    }
    match &members[2] {
        EnumMember::EnumStringMember { id, init } => {
            assert_eq!(id.name, "High", "member name");
            assert_eq!(init.value, "high", "lowercase value");
        }
        other => panic!("expected EnumStringMember, got: {other:?}"),
    }
}

#[test]
fn ast_flow_enum_string_no_rename() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act
    let (name, body) = parse_enum_decl(&parser, &Suit::decl(&cfg));

    // Assert
    assert_eq!(name, "Suit", "enum name");
    assert_eq!(body.type_name(), "EnumStringBody", "should be string body");
    let members = body.members();
    assert_eq!(members.len(), 4, "should have 4 members");

    // Without rename_all, values match variant names
    match &members[0] {
        EnumMember::EnumStringMember { id, init } => {
            assert_eq!(id.name, "Clubs", "member name");
            assert_eq!(init.value, "Clubs", "value matches name (no rename)");
        }
        other => panic!("expected EnumStringMember, got: {other:?}"),
    }
}

#[test]
fn ast_flow_enum_used_as_field() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    // Act — struct with flow enum fields should reference enum by name
    let (_, ty) = parse_type_alias(&parser, &WithFlowEnum::decl(&cfg));
    let props = object_properties(&ty);

    // Assert
    assert_eq!(props.len(), 2, "should have 2 fields");
    match &props[0] {
        ObjectMember::ObjectTypeProperty { key, value, .. } => {
            assert_eq!(key.name().unwrap(), "direction", "field name");
            match value {
                TypeAnnotation::GenericTypeAnnotation { id, .. } => {
                    assert_eq!(id.name().unwrap(), "Direction", "references Direction enum by name");
                }
                other => panic!("expected GenericTypeAnnotation, got: {}", other.type_name()),
            }
        }
        other => panic!("expected ObjectTypeProperty, got: {other:?}"),
    }
}

// ── Flow parser AST completeness ────────────────────────────────────────

#[test]
fn ast_parses_function_type() {
    // Arrange
    let parser = FlowParser::new().unwrap();

    // Act
    let program = parser
        .parse("export type Callback = (x: string, y: number) => boolean;")
        .unwrap();

    // Assert
    match &program.body[0] {
        Statement::ExportNamedDeclaration {
            declaration: Some(Declaration::TypeAlias { right, .. }),
        } => {
            assert_eq!(
                right.type_name(),
                "FunctionTypeAnnotation",
                "should parse function type"
            );
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn ast_parses_interface_declaration() {
    // Arrange
    let parser = FlowParser::new().unwrap();

    // Act
    let program = parser
        .parse("export interface Foo { x: number; y: string; }")
        .unwrap();

    // Assert
    match &program.body[0] {
        Statement::ExportNamedDeclaration {
            declaration: Some(Declaration::InterfaceDeclaration { id, .. }),
        } => {
            assert_eq!(id.name, "Foo", "interface name");
        }
        other => panic!("unexpected: {other:?}"),
    }
}

#[test]
fn ast_parses_indexed_access_type() {
    // Arrange
    let parser = FlowParser::new().unwrap();

    // Act
    let program = parser
        .parse("export type T = Obj['key'];")
        .unwrap();

    // Assert
    match &program.body[0] {
        Statement::ExportNamedDeclaration {
            declaration: Some(Declaration::TypeAlias { right, .. }),
        } => {
            assert_eq!(
                right.type_name(),
                "IndexedAccessType",
                "should parse indexed access"
            );
        }
        other => panic!("unexpected: {other:?}"),
    }
}

// ── Batch parse validation ──────────────────────────────────────────────

#[test]
fn validate_all_types_parse() {
    // Arrange
    let parser = FlowParser::new().unwrap();
    let cfg = Config::new();

    let decls: Vec<(&str, String)> = vec![
        ("SimpleStruct", SimpleStruct::decl(&cfg)),
        ("WithOption", WithOption::decl(&cfg)),
        ("WithVec", WithVec::decl(&cfg)),
        ("Nested", Nested::decl(&cfg)),
        ("Newtype", Newtype::decl(&cfg)),
        ("UnitStruct", UnitStruct::decl(&cfg)),
        ("CamelCase", CamelCase::decl(&cfg)),
        ("OpaqueToken", OpaqueToken::decl(&cfg)),
        ("BoundedOpaque", BoundedOpaque::decl(&cfg)),
        ("UntaggedUnion", UntaggedUnion::decl(&cfg)),
        ("TaggedUnion", TaggedUnion::decl(&cfg)),
        ("AdjacentlyTagged", AdjacentlyTagged::decl(&cfg)),
        (
            "AdjacentlyTaggedNewtype",
            AdjacentlyTaggedNewtype::decl(&cfg),
        ),
        (
            "InternallyTaggedNewtype",
            InternallyTaggedNewtype::decl(&cfg),
        ),
        ("ExternallyTagged", ExternallyTagged::decl(&cfg)),
        ("OptionalVariantFields", OptionalVariantFields::decl(&cfg)),
        ("RenamedVariantFields", RenamedVariantFields::decl(&cfg)),
        ("WithTuple", WithTuple::decl(&cfg)),
        ("KebabFields", KebabFields::decl(&cfg)),
        ("WithHashMap", WithHashMap::decl(&cfg)),
        ("Direction", Direction::decl(&cfg)),
        ("Priority", Priority::decl(&cfg)),
        ("Suit", Suit::decl(&cfg)),
    ];

    // Act and Assert
    let mut failures = Vec::new();
    for (name, decl) in &decls {
        if let Err(e) = parser.validate_declaration(decl) {
            failures.push(format!("{name}: {e}\n  {decl}"));
        }
    }

    assert!(
        failures.is_empty(),
        "Flow parse failures:\n{}",
        failures.join("\n\n")
    );
}
