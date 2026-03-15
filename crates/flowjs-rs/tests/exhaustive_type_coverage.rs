//! Exhaustive type coverage tests.
//!
//! Every Rust type that has a `Flow` impl is tested here:
//! 1. The generated Flow declaration parses without errors
//! 2. The AST structure matches the expected Flow type node
//!
//! This file is the single source of truth for correctness before publishing.

#![allow(dead_code)]

use flowjs_parser::{
    Declaration, EnumBody, EnumMember, FlowParser, ObjectMember, Statement, TypeAnnotation,
    VarianceKind,
};
use flowjs_rs::{Config, Flow};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, LinkedList, VecDeque};

// ── Helpers ─────────────────────────────────────────────────────────────

fn parser() -> FlowParser {
    FlowParser::new().expect("failed to create parser")
}

fn cfg() -> Config {
    Config::new()
}

/// Parse a type alias declaration → (name, rhs TypeAnnotation).
fn parse_alias(p: &FlowParser, decl: &str) -> (String, TypeAnnotation) {
    let program = p
        .validate_declaration(decl)
        .unwrap_or_else(|e| panic!("parse failed:\n{decl}\n\n{e}"));
    match &program.body[0] {
        Statement::ExportNamedDeclaration {
            declaration: Some(Declaration::TypeAlias { id, right }),
        } => (id.name.clone(), right.clone()),
        other => panic!("expected TypeAlias, got: {other:?}\n\ndecl: {decl}"),
    }
}

/// Parse a Flow enum declaration → (name, EnumBody).
fn parse_enum(p: &FlowParser, decl: &str) -> (String, EnumBody) {
    let program = p
        .validate_declaration(decl)
        .unwrap_or_else(|e| panic!("parse failed:\n{decl}\n\n{e}"));
    match &program.body[0] {
        Statement::ExportNamedDeclaration {
            declaration: Some(Declaration::EnumDeclaration { id, body }),
        } => (id.name.clone(), body.clone()),
        other => panic!("expected EnumDeclaration, got: {other:?}\n\ndecl: {decl}"),
    }
}

/// Parse an opaque type declaration → (name, Option<supertype>).
fn parse_opaque(p: &FlowParser, decl: &str) -> (String, Option<TypeAnnotation>) {
    let program = p
        .validate_declaration(decl)
        .unwrap_or_else(|e| panic!("parse failed:\n{decl}\n\n{e}"));
    match &program.body[0] {
        // `export opaque type Foo = body` → ExportNamedDeclaration(OpaqueType)
        Statement::ExportNamedDeclaration {
            declaration: Some(decl),
        }
        | Statement::DeclareExportDeclaration {
            declaration: Some(decl),
        } => match decl {
            Declaration::OpaqueType { id, supertype }
            | Declaration::DeclareOpaqueType { id, supertype } => {
                (id.name.clone(), supertype.clone())
            }
            other => panic!("expected OpaqueType, got: {other:?}"),
        },
        other => panic!("expected ExportNamedDeclaration(OpaqueType), got: {other:?}"),
    }
}

/// Extract properties from an ObjectTypeAnnotation.
fn props(ty: &TypeAnnotation) -> &[ObjectMember] {
    match ty {
        TypeAnnotation::ObjectTypeAnnotation { properties, .. } => properties,
        other => panic!("expected ObjectTypeAnnotation, got: {}", other.type_name()),
    }
}

/// Extract a single field's value type from an object type by field name.
fn field_type<'a>(ty: &'a TypeAnnotation, name: &str) -> &'a TypeAnnotation {
    for p in props(ty) {
        if let ObjectMember::ObjectTypeProperty { key, value, .. } = p {
            if key.name() == Some(name) {
                return value;
            }
        }
    }
    panic!("field `{name}` not found in {ty:?}");
}

/// Assert a field has the expected type annotation node name.
fn assert_field(ty: &TypeAnnotation, field: &str, expected_type_name: &str) {
    let ft = field_type(ty, field);
    assert_eq!(
        ft.type_name(),
        expected_type_name,
        "field `{field}`: expected {expected_type_name}, got {}",
        ft.type_name()
    );
}

/// Assert a field is a GenericTypeAnnotation with the given identifier.
fn assert_field_generic(ty: &TypeAnnotation, field: &str, expected_id: &str) {
    let ft = field_type(ty, field);
    match ft {
        TypeAnnotation::GenericTypeAnnotation { id, .. } => {
            assert_eq!(
                id.name().unwrap(),
                expected_id,
                "field `{field}`: expected generic id `{expected_id}`"
            );
        }
        other => panic!(
            "field `{field}`: expected GenericTypeAnnotation, got {}",
            other.type_name()
        ),
    }
}

/// Assert a field is covariant (+).
fn assert_covariant(ty: &TypeAnnotation, field: &str) {
    for p in props(ty) {
        if let ObjectMember::ObjectTypeProperty {
            key, variance, ..
        } = p
        {
            if key.name() == Some(field) {
                let v = variance.as_ref().expect("should have variance");
                assert_eq!(v.kind, VarianceKind::Plus, "field `{field}` should be covariant (+)");
                return;
            }
        }
    }
    panic!("field `{field}` not found");
}

/// Assert a field is optional (key-optional `field?:`).
fn assert_optional(ty: &TypeAnnotation, field: &str, expected: bool) {
    for p in props(ty) {
        if let ObjectMember::ObjectTypeProperty {
            key, optional, ..
        } = p
        {
            if key.name() == Some(field) {
                assert_eq!(
                    *optional, expected,
                    "field `{field}`: optional={optional}, expected={expected}"
                );
                return;
            }
        }
    }
    panic!("field `{field}` not found");
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 1: Primitive type impls
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct AllPrimitives {
    f_bool: bool,
    f_i8: i8,
    f_i16: i16,
    f_i32: i32,
    f_i64: i64,
    f_i128: i128,
    f_u8: u8,
    f_u16: u16,
    f_u32: u32,
    f_u64: u64,
    f_u128: u128,
    f_f32: f32,
    f_f64: f64,
    f_usize: usize,
    f_isize: isize,
    f_char: char,
    f_string: String,
}

#[test]
fn primitives_parse_and_match() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &AllPrimitives::decl(&cfg()));

    assert_field(&ty, "f_bool", "BooleanTypeAnnotation");
    assert_field(&ty, "f_i8", "NumberTypeAnnotation");
    assert_field(&ty, "f_i16", "NumberTypeAnnotation");
    assert_field(&ty, "f_i32", "NumberTypeAnnotation");
    assert_field(&ty, "f_i64", "BigIntTypeAnnotation");
    assert_field(&ty, "f_i128", "BigIntTypeAnnotation");
    assert_field(&ty, "f_u8", "NumberTypeAnnotation");
    assert_field(&ty, "f_u16", "NumberTypeAnnotation");
    assert_field(&ty, "f_u32", "NumberTypeAnnotation");
    assert_field(&ty, "f_u64", "BigIntTypeAnnotation");
    assert_field(&ty, "f_u128", "BigIntTypeAnnotation");
    assert_field(&ty, "f_f32", "NumberTypeAnnotation");
    assert_field(&ty, "f_f64", "NumberTypeAnnotation");
    assert_field(&ty, "f_usize", "NumberTypeAnnotation");
    assert_field(&ty, "f_isize", "NumberTypeAnnotation");
    assert_field(&ty, "f_char", "StringTypeAnnotation");
    assert_field(&ty, "f_string", "StringTypeAnnotation");
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 2: Unit / void / bottom types
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct UnitStruct;

#[derive(Flow)]
struct WithPhantom {
    marker: std::marker::PhantomData<String>,
}

#[test]
fn unit_struct_is_null() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &UnitStruct::decl(&cfg()));
    assert_eq!(ty.type_name(), "NullLiteralTypeAnnotation");
}

#[test]
fn phantom_data_is_void() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &WithPhantom::decl(&cfg()));
    assert_field(&ty, "marker", "VoidTypeAnnotation");
}

#[test]
fn unit_type_is_null() {
    let c = cfg();
    assert_eq!(<() as Flow>::name(&c), "null");
}

#[test]
fn infallible_is_empty() {
    let c = cfg();
    assert_eq!(std::convert::Infallible::name(&c), "empty");
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 3: Option<T> → ?T
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct WithOptions {
    required: String,
    nullable: Option<String>,
    nested_nullable: Option<Option<i32>>,
}

#[test]
fn option_is_nullable_not_optional() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &WithOptions::decl(&cfg()));

    // Option<String> → ?string (always present, but nullable)
    assert_field(&ty, "nullable", "NullableTypeAnnotation");
    assert_optional(&ty, "nullable", false);

    // Unwrap NullableTypeAnnotation → inner is string
    match field_type(&ty, "nullable") {
        TypeAnnotation::NullableTypeAnnotation { type_annotation } => {
            assert_eq!(type_annotation.type_name(), "StringTypeAnnotation");
        }
        _ => unreachable!(),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 4: Collections → $ReadOnlyArray<T>
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct AllCollections {
    vec: Vec<i32>,
    hash_set: HashSet<String>,
    btree_set: BTreeSet<i32>,
    vec_deque: VecDeque<bool>,
    linked_list: LinkedList<f64>,
}

#[test]
fn all_collections_are_readonly_array() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &AllCollections::decl(&cfg()));

    for field in ["vec", "hash_set", "btree_set", "vec_deque", "linked_list"] {
        assert_field_generic(&ty, field, "$ReadOnlyArray");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 5: Maps → { [key: K]: V }
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct AllMaps {
    hash_map: HashMap<String, i32>,
    btree_map: BTreeMap<String, bool>,
}

#[test]
fn maps_are_objects_with_indexers() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &AllMaps::decl(&cfg()));

    for field in ["hash_map", "btree_map"] {
        let ft = field_type(&ty, field);
        match ft {
            TypeAnnotation::ObjectTypeAnnotation { indexers, .. } => {
                assert!(!indexers.is_empty(), "field `{field}` should have indexer");
                assert_eq!(indexers[0].key.type_name(), "StringTypeAnnotation");
            }
            other => panic!("field `{field}`: expected ObjectTypeAnnotation, got {}", other.type_name()),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 6: Tuples
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct WithTuples {
    pair: (String, i32),
    triple: (bool, f64, String),
    single: (i32,),
}

#[test]
fn tuples_are_tuple_type_annotation() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &WithTuples::decl(&cfg()));

    match field_type(&ty, "pair") {
        TypeAnnotation::TupleTypeAnnotation { element_types } => {
            assert_eq!(element_types.len(), 2);
            assert_eq!(element_types[0].type_name(), "StringTypeAnnotation");
            assert_eq!(element_types[1].type_name(), "NumberTypeAnnotation");
        }
        other => panic!("expected TupleTypeAnnotation, got {}", other.type_name()),
    }

    match field_type(&ty, "triple") {
        TypeAnnotation::TupleTypeAnnotation { element_types } => {
            assert_eq!(element_types.len(), 3);
        }
        other => panic!("expected TupleTypeAnnotation, got {}", other.type_name()),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 7: Fixed-size arrays
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct WithArrays {
    small: [i32; 3],
}

#[test]
fn small_array_is_tuple() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &WithArrays::decl(&cfg()));

    match field_type(&ty, "small") {
        TypeAnnotation::TupleTypeAnnotation { element_types } => {
            assert_eq!(element_types.len(), 3);
            for et in element_types {
                assert_eq!(et.type_name(), "NumberTypeAnnotation");
            }
        }
        other => panic!("expected TupleTypeAnnotation, got {}", other.type_name()),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 8: Wrapper types (transparent)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct AllWrappers {
    boxed: Box<String>,
    arc: std::sync::Arc<i32>,
    rc: std::rc::Rc<bool>,
    cell: std::cell::Cell<f64>,
    ref_cell: std::cell::RefCell<String>,
    mutex: std::sync::Mutex<i32>,
    rw_lock: std::sync::RwLock<bool>,
    wrapping: std::num::Wrapping<u32>,
    saturating: std::num::Saturating<i64>,
}

#[test]
fn wrappers_are_transparent() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &AllWrappers::decl(&cfg()));

    assert_field(&ty, "boxed", "StringTypeAnnotation");
    assert_field(&ty, "arc", "NumberTypeAnnotation");
    assert_field(&ty, "rc", "BooleanTypeAnnotation");
    assert_field(&ty, "cell", "NumberTypeAnnotation");
    assert_field(&ty, "ref_cell", "StringTypeAnnotation");
    assert_field(&ty, "mutex", "NumberTypeAnnotation");
    assert_field(&ty, "rw_lock", "BooleanTypeAnnotation");
    assert_field(&ty, "wrapping", "NumberTypeAnnotation");
    assert_field(&ty, "saturating", "BigIntTypeAnnotation"); // Saturating<i64> → i64 → bigint
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 9: NonZero types
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct AllNonZero {
    nz_u8: std::num::NonZeroU8,
    nz_u16: std::num::NonZeroU16,
    nz_u32: std::num::NonZeroU32,
    nz_u64: std::num::NonZeroU64,
    nz_u128: std::num::NonZeroU128,
    nz_usize: std::num::NonZeroUsize,
    nz_i8: std::num::NonZeroI8,
    nz_i16: std::num::NonZeroI16,
    nz_i32: std::num::NonZeroI32,
    nz_i64: std::num::NonZeroI64,
    nz_i128: std::num::NonZeroI128,
    nz_isize: std::num::NonZeroIsize,
}

#[test]
fn nonzero_types_are_number() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &AllNonZero::decl(&cfg()));

    // Small NonZero types → number
    for field in ["nz_u8", "nz_u16", "nz_u32", "nz_usize", "nz_i8", "nz_i16", "nz_i32", "nz_isize"] {
        assert_field(&ty, field, "NumberTypeAnnotation");
    }
    // Large NonZero types → bigint (delegates to underlying i64/u64/i128/u128)
    for field in ["nz_u64", "nz_u128", "nz_i64", "nz_i128"] {
        assert_field(&ty, field, "BigIntTypeAnnotation");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 10: String-like types
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct StringLikes {
    path: std::path::PathBuf,
    system_time: std::time::SystemTime,
    ip: std::net::IpAddr,
    ipv4: std::net::Ipv4Addr,
    ipv6: std::net::Ipv6Addr,
    sock: std::net::SocketAddr,
    sock4: std::net::SocketAddrV4,
    sock6: std::net::SocketAddrV6,
}

#[test]
fn string_like_types_are_string() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &StringLikes::decl(&cfg()));

    for field in ["path", "system_time", "ip", "ipv4", "ipv6", "sock", "sock4", "sock6"] {
        assert_field(&ty, field, "StringTypeAnnotation");
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 11: Range / Duration
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct StructuralTypes {
    range: std::ops::Range<i32>,
    range_incl: std::ops::RangeInclusive<f64>,
    duration: std::time::Duration,
}

#[test]
fn range_and_duration_are_objects() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &StructuralTypes::decl(&cfg()));

    // Range → { +start: number, +end: number }
    let range_ty = field_type(&ty, "range");
    assert_eq!(range_ty.type_name(), "ObjectTypeAnnotation");
    assert_field(range_ty, "start", "NumberTypeAnnotation");
    assert_field(range_ty, "end", "NumberTypeAnnotation");

    // Duration → {| +secs: number, +nanos: number |}
    let dur_ty = field_type(&ty, "duration");
    assert_eq!(dur_ty.type_name(), "ObjectTypeAnnotation");
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 12: Result<T, E>
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct WithResult {
    result: Result<String, i32>,
}

#[test]
fn result_is_union_of_ok_err() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &WithResult::decl(&cfg()));

    let result_ty = field_type(&ty, "result");
    match result_ty {
        TypeAnnotation::UnionTypeAnnotation { types } => {
            assert_eq!(types.len(), 2, "Result should be union of 2");
            // First: {| Ok: string |}
            let ok_props = props(&types[0]);
            assert!(ok_props.iter().any(|p| matches!(p,
                ObjectMember::ObjectTypeProperty { key, .. } if key.name() == Some("Ok")
            )), "should have Ok field");
            // Second: {| Err: number |}
            let err_props = props(&types[1]);
            assert!(err_props.iter().any(|p| matches!(p,
                ObjectMember::ObjectTypeProperty { key, .. } if key.name() == Some("Err")
            )), "should have Err field");
        }
        other => panic!("expected UnionTypeAnnotation, got {}", other.type_name()),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 13: Function pointers
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct WithFunctions {
    no_args: fn() -> String,
    one_arg: fn(i32) -> bool,
    two_args: fn(String, f64) -> (),
    three_args: fn(bool, i32, String) -> f64,
}

#[test]
fn fn_pointers_are_function_type_annotation() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &WithFunctions::decl(&cfg()));

    for field in ["no_args", "one_arg", "two_args", "three_args"] {
        assert_field(&ty, field, "FunctionTypeAnnotation");
    }

    // Verify parameter count and return type
    match field_type(&ty, "two_args") {
        TypeAnnotation::FunctionTypeAnnotation {
            params,
            return_type,
            ..
        } => {
            assert_eq!(params.len(), 2, "fn(String, f64) has 2 params");
            assert_eq!(return_type.type_name(), "NullLiteralTypeAnnotation", "-> () is null");
        }
        _ => unreachable!(),
    }

    match field_type(&ty, "no_args") {
        TypeAnnotation::FunctionTypeAnnotation {
            params,
            return_type,
            ..
        } => {
            assert_eq!(params.len(), 0, "fn() has 0 params");
            assert_eq!(return_type.type_name(), "StringTypeAnnotation", "-> String");
        }
        _ => unreachable!(),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 14: Struct derive variants
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct Newtype(String);

#[derive(Flow)]
struct TupleStruct(i32, bool, String);

#[derive(Flow)]
#[flow(rename = "CustomName")]
struct Renamed {
    x: i32,
}

#[derive(Flow)]
#[flow(rename_all = "camelCase")]
struct CamelFields {
    first_name: String,
    last_name: String,
}

#[derive(Flow)]
#[flow(rename_all = "kebab-case")]
struct KebabFields {
    field_one: String,
    field_two: i32,
}

#[test]
fn newtype_inlines_inner() {
    let p = parser();
    let (name, ty) = parse_alias(&p, &Newtype::decl(&cfg()));
    assert_eq!(name, "Newtype");
    assert_eq!(ty.type_name(), "StringTypeAnnotation");
}

#[test]
fn tuple_struct_is_tuple() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &TupleStruct::decl(&cfg()));
    match &ty {
        TypeAnnotation::TupleTypeAnnotation { element_types } => {
            assert_eq!(element_types.len(), 3);
        }
        other => panic!("expected TupleTypeAnnotation, got {}", other.type_name()),
    }
}

#[test]
fn renamed_struct() {
    let p = parser();
    let (name, _) = parse_alias(&p, &Renamed::decl(&cfg()));
    assert_eq!(name, "CustomName");
}

#[test]
fn camel_case_field_names() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &CamelFields::decl(&cfg()));
    let names: Vec<&str> = props(&ty)
        .iter()
        .filter_map(|p| match p {
            ObjectMember::ObjectTypeProperty { key, .. } => key.name(),
            _ => None,
        })
        .collect();
    assert_eq!(names, vec!["firstName", "lastName"]);
}

#[test]
fn kebab_case_fields_are_quoted() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &KebabFields::decl(&cfg()));
    let keys: Vec<(Option<&str>, bool)> = props(&ty)
        .iter()
        .filter_map(|p| match p {
            ObjectMember::ObjectTypeProperty { key, .. } => Some((key.name(), key.is_quoted())),
            _ => None,
        })
        .collect();
    assert_eq!(keys[0].0, Some("field-one"));
    assert!(keys[0].1, "kebab-case should be quoted");
}

#[test]
fn default_fields_have_no_variance_annotation() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &AllPrimitives::decl(&cfg()));
    // Default matches ts-rs: no readonly/variance annotation
    for prop in props(&ty) {
        if let ObjectMember::ObjectTypeProperty { key, variance, .. } = prop {
            assert!(
                variance.is_none(),
                "field `{}` should have no variance by default (matches ts-rs)",
                key.name().unwrap_or("?")
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 15: Opaque types
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
#[flow(opaque)]
struct OpaqueToken(String);

#[derive(Flow)]
#[flow(opaque = "string")]
struct BoundedOpaque(String);

#[test]
fn opaque_fully() {
    let p = parser();
    let (name, supertype) = parse_opaque(&p, &OpaqueToken::decl(&cfg()));
    assert_eq!(name, "OpaqueToken");
    assert!(supertype.is_none());
}

#[test]
fn opaque_bounded() {
    let p = parser();
    let (name, supertype) = parse_opaque(&p, &BoundedOpaque::decl(&cfg()));
    assert_eq!(name, "BoundedOpaque");
    assert_eq!(
        supertype.unwrap().type_name(),
        "StringTypeAnnotation"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 16: Enum variants (union types)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
#[flow(untagged)]
enum Untagged {
    Circle { radius: f64 },
    Rect { w: f64, h: f64 },
}

#[derive(Flow)]
#[flow(tag = "kind")]
enum InternallyTagged {
    Click,
    Move { x: i32, y: i32 },
}

#[derive(Flow)]
#[flow(tag = "t", content = "v")]
enum AdjacentlyTagged {
    Str(String),
    Num(i32),
}

#[derive(Flow)]
enum ExternallyTagged {
    Alpha { x: i32 },
    Beta(String),
    Gamma,
}

#[test]
fn untagged_enum_is_union() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &Untagged::decl(&cfg()));
    match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => {
            assert_eq!(types.len(), 2);
        }
        other => panic!("expected union, got {}", other.type_name()),
    }
}

#[test]
fn internally_tagged_has_discriminant() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &InternallyTagged::decl(&cfg()));
    let union_types = match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => types,
        other => panic!("expected union, got {}", other.type_name()),
    };

    // Click variant: {| +kind: 'Click' |}
    let click_props = props(&union_types[0]);
    match &click_props[0] {
        ObjectMember::ObjectTypeProperty { key, value, .. } => {
            assert_eq!(key.name().unwrap(), "kind");
            match value {
                TypeAnnotation::StringLiteralTypeAnnotation { value } => {
                    assert_eq!(value, "Click");
                }
                other => panic!("expected StringLiteral, got {}", other.type_name()),
            }
        }
        other => panic!("expected prop, got {other:?}"),
    }
}

#[test]
fn adjacently_tagged_has_tag_and_content() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &AdjacentlyTagged::decl(&cfg()));
    let union_types = match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => types,
        other => panic!("expected union, got {}", other.type_name()),
    };

    let p = props(&union_types[0]);
    assert!(p.len() >= 2, "should have tag + content");
    let tag_name = match &p[0] {
        ObjectMember::ObjectTypeProperty { key, .. } => key.name().unwrap(),
        _ => panic!("expected prop"),
    };
    let content_name = match &p[1] {
        ObjectMember::ObjectTypeProperty { key, .. } => key.name().unwrap(),
        _ => panic!("expected prop"),
    };
    assert_eq!(tag_name, "t");
    assert_eq!(content_name, "v");
}

#[test]
fn externally_tagged_has_variant_keys() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &ExternallyTagged::decl(&cfg()));
    match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => {
            assert_eq!(types.len(), 3);
        }
        other => panic!("expected union, got {}", other.type_name()),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 17: Flow enums — all 4 representations
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
#[flow(flow_enum)]
enum SymbolEnum {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Flow)]
#[flow(flow_enum = "string", rename_all = "lowercase")]
enum StringEnum {
    Active,
    Paused,
    Stopped,
}

#[derive(Flow)]
#[flow(flow_enum = "number")]
#[repr(i32)]
enum NumberEnum {
    Ok = 200,
    NotFound = 404,
    Internal = 500,
}

#[derive(Flow)]
#[flow(flow_enum = "number")]
enum AutoNumberEnum {
    A,
    B,
    C,
}

#[derive(Flow)]
#[flow(flow_enum = "boolean")]
enum BoolEnum {
    Yes,
    No,
}

#[test]
fn symbol_enum_parses() {
    let p = parser();
    let (name, body) = parse_enum(&p, &SymbolEnum::decl(&cfg()));
    assert_eq!(name, "SymbolEnum");
    let members = body.members();
    assert_eq!(members.len(), 4);
    // All defaulted
    for m in members {
        assert!(matches!(m, EnumMember::EnumDefaultedMember { .. }));
    }
}

#[test]
fn string_enum_has_values() {
    let p = parser();
    let (name, body) = parse_enum(&p, &StringEnum::decl(&cfg()));
    assert_eq!(name, "StringEnum");
    assert_eq!(body.type_name(), "EnumStringBody");
    let members = body.members();
    assert_eq!(members.len(), 3);

    match &members[0] {
        EnumMember::EnumStringMember { id, init } => {
            assert_eq!(id.name, "Active");
            assert_eq!(init.value, "active");
        }
        other => panic!("expected EnumStringMember, got {other:?}"),
    }
}

#[test]
fn number_enum_has_discriminants() {
    let p = parser();
    let (name, body) = parse_enum(&p, &NumberEnum::decl(&cfg()));
    assert_eq!(name, "NumberEnum");
    assert_eq!(body.type_name(), "EnumNumberBody");
    let members = body.members();
    assert_eq!(members.len(), 3);

    match &members[0] {
        EnumMember::EnumNumberMember { id, init } => {
            assert_eq!(id.name, "Ok");
            assert_eq!(init.value, 200.0);
        }
        other => panic!("expected EnumNumberMember, got {other:?}"),
    }
    match &members[1] {
        EnumMember::EnumNumberMember { id, init } => {
            assert_eq!(id.name, "NotFound");
            assert_eq!(init.value, 404.0);
        }
        other => panic!("expected EnumNumberMember, got {other:?}"),
    }
}

#[test]
fn number_enum_auto_increments() {
    let p = parser();
    let (_, body) = parse_enum(&p, &AutoNumberEnum::decl(&cfg()));
    let members = body.members();
    match &members[0] {
        EnumMember::EnumNumberMember { init, .. } => assert_eq!(init.value, 0.0),
        other => panic!("expected EnumNumberMember, got {other:?}"),
    }
    match &members[1] {
        EnumMember::EnumNumberMember { init, .. } => assert_eq!(init.value, 1.0),
        other => panic!("expected EnumNumberMember, got {other:?}"),
    }
    match &members[2] {
        EnumMember::EnumNumberMember { init, .. } => assert_eq!(init.value, 2.0),
        other => panic!("expected EnumNumberMember, got {other:?}"),
    }
}

#[test]
fn boolean_enum_has_true_false() {
    let p = parser();
    let (name, body) = parse_enum(&p, &BoolEnum::decl(&cfg()));
    assert_eq!(name, "BoolEnum");
    assert_eq!(body.type_name(), "EnumBooleanBody");
    let members = body.members();
    assert_eq!(members.len(), 2);

    match &members[0] {
        EnumMember::EnumBooleanMember { id, init } => {
            assert_eq!(id.name, "Yes");
            assert!(init.value);
        }
        other => panic!("expected EnumBooleanMember, got {other:?}"),
    }
    match &members[1] {
        EnumMember::EnumBooleanMember { id, init } => {
            assert_eq!(id.name, "No");
            assert!(!init.value);
        }
        other => panic!("expected EnumBooleanMember, got {other:?}"),
    }
}

#[test]
fn flow_enum_inline_is_name_reference() {
    let c = cfg();
    assert_eq!(SymbolEnum::inline(&c), "SymbolEnum");
    assert_eq!(StringEnum::inline(&c), "StringEnum");
    assert_eq!(NumberEnum::inline(&c), "NumberEnum");
    assert_eq!(BoolEnum::inline(&c), "BoolEnum");
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 18: Generics
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct Wrapper<T: Flow> {
    value: T,
    label: String,
}

#[test]
fn generic_type_has_params() {
    let c = cfg();
    let decl = Wrapper::<flowjs_rs::Dummy>::decl(&c);
    assert!(decl.contains("Wrapper<T>"), "should have generic param: {decl}");
    assert!(decl.contains("value: T"), "should use param: {decl}");

    let p = parser();
    p.validate_declaration(&decl)
        .unwrap_or_else(|e| panic!("generic type should parse: {e}\n{decl}"));
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 19: Flatten (intersection)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct Base {
    id: i32,
}

#[derive(Flow)]
struct Extended {
    name: String,
    #[flow(flatten)]
    base: Base,
}

#[test]
fn flatten_produces_intersection() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &Extended::decl(&cfg()));
    assert_eq!(
        ty.type_name(),
        "IntersectionTypeAnnotation",
        "flatten should produce intersection"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 20: Serde compat — optional fields
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow, serde::Serialize)]
struct SerdeOptional {
    required: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    omittable: Option<String>,
    #[flow(optional)]
    explicit_opt: String,
}

#[test]
fn serde_skip_serializing_if_makes_optional() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &SerdeOptional::decl(&cfg()));

    assert_optional(&ty, "required", false);
    assert_optional(&ty, "omittable", true);
    assert_optional(&ty, "explicit_opt", true);
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 21: Flow enum used as field type
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct UsesEnums {
    direction: SymbolEnum,
    status: StringEnum,
    code: NumberEnum,
    flag: BoolEnum,
}

#[test]
fn flow_enum_fields_are_generic_type_refs() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &UsesEnums::decl(&cfg()));

    // Flow enums are referenced by name (GenericTypeAnnotation)
    assert_field_generic(&ty, "direction", "SymbolEnum");
    assert_field_generic(&ty, "status", "StringEnum");
    assert_field_generic(&ty, "code", "NumberEnum");
    assert_field_generic(&ty, "flag", "BoolEnum");
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 22: serde_json::Value (requires serde-json-impl feature)
// ═══════════════════════════════════════════════════════════════════════

// serde_json::Value → mixed is tested via name/inline assertions.
// The impl is behind `serde-json-impl` feature. When enabled:
//   serde_json::Value::inline() = "mixed"
//   serde_json::Value::name()   = "JsonValue"
// This is tested in the testing project (flowjs-rs-testing) which enables the feature.

// ═══════════════════════════════════════════════════════════════════════
// SECTION 23: Cow
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct WithCow {
    borrowed: std::borrow::Cow<'static, str>,
}

#[test]
fn cow_is_transparent() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &WithCow::decl(&cfg()));
    assert_field(&ty, "borrowed", "StringTypeAnnotation");
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 24: Missing enum variant shapes × tagging modes
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
enum ExtTaggedFull {
    Unit,
    Named { x: i32, y: String },
    Newtype(bool),
    Tuple(i32, String),
}

#[derive(Flow)]
#[flow(tag = "t")]
enum IntTaggedFull {
    Unit,
    Named { x: i32 },
    Newtype(String),
}

#[derive(Flow)]
#[flow(tag = "t", content = "c")]
enum AdjTaggedFull {
    Unit,
    Named { x: i32 },
    Newtype(String),
    Tuple(i32, bool),
}

#[derive(Flow)]
#[flow(untagged)]
enum UntaggedFull {
    Unit,
    Named { x: i32 },
    Newtype(String),
    Tuple(i32, bool),
}

#[test]
fn ext_tagged_all_variant_shapes() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &ExtTaggedFull::decl(&cfg()));
    match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => assert_eq!(types.len(), 4),
        other => panic!("expected union, got {}", other.type_name()),
    }
}

#[test]
fn int_tagged_all_variant_shapes() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &IntTaggedFull::decl(&cfg()));
    match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => assert_eq!(types.len(), 3),
        other => panic!("expected union, got {}", other.type_name()),
    }
}

#[test]
fn adj_tagged_all_variant_shapes() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &AdjTaggedFull::decl(&cfg()));
    match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => assert_eq!(types.len(), 4),
        other => panic!("expected union, got {}", other.type_name()),
    }
}

#[test]
fn untagged_all_variant_shapes() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &UntaggedFull::decl(&cfg()));
    match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => assert_eq!(types.len(), 4),
        other => panic!("expected union, got {}", other.type_name()),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 25: Container type/as overrides
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
#[flow(type = "$ReadOnlyArray<string>")]
struct TypeOverrideContainer(Vec<u8>);

#[derive(Flow)]
#[flow(as = "Vec<String>")]
struct AsOverrideContainer {
    _data: Vec<u8>,
}

#[test]
fn container_type_override() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &TypeOverrideContainer::decl(&cfg()));
    match &ty {
        TypeAnnotation::GenericTypeAnnotation { id, .. } => {
            assert_eq!(id.name().unwrap(), "$ReadOnlyArray");
        }
        other => panic!("expected GenericTypeAnnotation, got {}", other.type_name()),
    }
}

#[test]
fn container_as_override() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &AsOverrideContainer::decl(&cfg()));
    match &ty {
        TypeAnnotation::GenericTypeAnnotation { id, .. } => {
            assert_eq!(id.name().unwrap(), "$ReadOnlyArray");
        }
        other => panic!("expected GenericTypeAnnotation, got {}", other.type_name()),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 26: Field attributes (skip, inline, type, as, rename)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct FieldSkip {
    visible: String,
    #[flow(skip)]
    hidden: i32,
}

#[derive(Flow)]
struct FieldTypeOverride {
    #[flow(type = "mixed")]
    data: Vec<u8>,
}

#[derive(Flow)]
struct FieldAsOverride {
    #[flow(as = "String")]
    data: Vec<u8>,
}

#[derive(Flow)]
struct FieldInline {
    #[flow(inline)]
    inner: Base,
}

#[derive(Flow)]
struct FieldRename {
    #[flow(rename = "customName")]
    original: String,
}

#[test]
fn field_skip_omits_field() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &FieldSkip::decl(&cfg()));
    let p = props(&ty);
    assert_eq!(p.len(), 1, "skipped field should not appear");
    assert_field(&ty, "visible", "StringTypeAnnotation");
}

#[test]
fn field_type_override() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &FieldTypeOverride::decl(&cfg()));
    assert_field(&ty, "data", "MixedTypeAnnotation");
}

#[test]
fn field_as_override() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &FieldAsOverride::decl(&cfg()));
    assert_field(&ty, "data", "StringTypeAnnotation");
}

#[test]
fn field_inline_expands_type() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &FieldInline::decl(&cfg()));
    // Inlined Base should produce an object, not a GenericTypeAnnotation reference
    assert_field(&ty, "inner", "ObjectTypeAnnotation");
}

#[test]
fn field_rename() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &FieldRename::decl(&cfg()));
    let names: Vec<&str> = props(&ty)
        .iter()
        .filter_map(|p| match p {
            ObjectMember::ObjectTypeProperty { key, .. } => key.name(),
            _ => None,
        })
        .collect();
    assert_eq!(names, vec!["customName"]);
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 27: Remaining rename_all inflections
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
#[flow(rename_all = "UPPERCASE")]
struct UpperFields { my_field: String }

#[derive(Flow)]
#[flow(rename_all = "snake_case")]
struct SnakeFields { myField: String }

#[derive(Flow)]
#[flow(rename_all = "PascalCase")]
struct PascalFields { my_field: String }

#[derive(Flow)]
#[flow(rename_all = "SCREAMING_SNAKE_CASE")]
struct ScreamSnakeFields { my_field: String }

#[derive(Flow)]
#[flow(rename_all = "SCREAMING-KEBAB-CASE")]
struct ScreamKebabFields { my_field: String }

#[test]
fn all_rename_inflections_parse() {
    let p = parser();
    let c = cfg();
    // Just verify they all parse without error — the inflection logic itself is unit-tested
    for (name, decl) in [
        ("UPPERCASE", UpperFields::decl(&c)),
        ("snake_case", SnakeFields::decl(&c)),
        ("PascalCase", PascalFields::decl(&c)),
        ("SCREAMING_SNAKE_CASE", ScreamSnakeFields::decl(&c)),
        ("SCREAMING-KEBAB-CASE", ScreamKebabFields::decl(&c)),
    ] {
        p.validate_declaration(&decl)
            .unwrap_or_else(|e| panic!("{name}: {e}\n{decl}"));
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 28: Multiple generics, empty struct, empty enum, negative discriminant
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct MultiGeneric<A: Flow, B: Flow> {
    first: A,
    second: B,
}

// Note: truly empty named struct uses a separate test type that
// just has a field to ensure it's not confused with unit struct
#[derive(Flow)]
struct SingleFieldStruct {
    only: String,
}

#[derive(Flow)]
enum EmptyEnum {}

#[derive(Flow)]
#[flow(flow_enum = "number")]
#[repr(i32)]
enum NegativeDisc {
    Neg = -1,
    Zero = 0,
    Pos = 1,
}

#[derive(Flow)]
#[flow(flow_enum)]
enum EmptyFlowEnum {}

#[test]
fn multi_generic_params() {
    let c = cfg();
    let decl = MultiGeneric::<flowjs_rs::Dummy, flowjs_rs::Dummy>::decl(&c);
    assert!(decl.contains("MultiGeneric<A, B>"), "should have 2 generic params: {decl}");
    let p = parser();
    p.validate_declaration(&decl).unwrap();
}

#[test]
fn single_field_struct_parses() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &SingleFieldStruct::decl(&cfg()));
    assert_field(&ty, "only", "StringTypeAnnotation");
}

#[test]
fn empty_enum_is_empty() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &EmptyEnum::decl(&cfg()));
    assert_eq!(ty.type_name(), "EmptyTypeAnnotation");
}

#[test]
fn negative_discriminant_enum() {
    let p = parser();
    let (_, body) = parse_enum(&p, &NegativeDisc::decl(&cfg()));
    let members = body.members();
    match &members[0] {
        EnumMember::EnumNumberMember { id, init } => {
            assert_eq!(id.name, "Neg");
            assert_eq!(init.value, -1.0);
        }
        other => panic!("expected EnumNumberMember, got {other:?}"),
    }
}

#[test]
fn empty_flow_enum_is_empty_type() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &EmptyFlowEnum::decl(&cfg()));
    assert_eq!(ty.type_name(), "EmptyTypeAnnotation");
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 29: Doc comments → JSDoc
// ═══════════════════════════════════════════════════════════════════════

/// This is a documented type.
/// It has multiple lines.
#[derive(Flow)]
struct Documented {
    field: String,
}

#[test]
fn doc_comments_produce_jsdoc() {
    let c = cfg();
    let content = Documented::export_to_string(&c).unwrap();
    assert!(content.contains("/**"), "should have JSDoc open: {content}");
    assert!(content.contains(" * This is a documented type."), "should have doc line: {content}");
    assert!(content.contains(" */"), "should have JSDoc close: {content}");
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 30: Large array → $ReadOnlyArray (exceeds tuple limit)
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct WithLargeArray {
    data: [u8; 128],
}

#[test]
fn large_array_is_readonly_array() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &WithLargeArray::decl(&cfg()));
    assert_field_generic(&ty, "data", "$ReadOnlyArray");
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 31: Variant skip and rename
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
#[flow(tag = "kind")]
enum WithSkippedVariant {
    Kept,
    #[flow(skip)]
    Removed,
    AlsoKept,
}

#[derive(Flow)]
enum WithRenamedVariant {
    #[flow(rename = "custom_name")]
    Original { x: i32 },
}

#[test]
fn variant_skip_omits_variant() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &WithSkippedVariant::decl(&cfg()));
    match &ty {
        TypeAnnotation::UnionTypeAnnotation { types } => {
            assert_eq!(types.len(), 2, "skipped variant should not appear");
        }
        other => panic!("expected union, got {}", other.type_name()),
    }
}

#[test]
fn variant_rename() {
    let c = cfg();
    let decl = WithRenamedVariant::decl(&c);
    assert!(decl.contains("custom_name"), "should use renamed variant: {decl}");
    let p = parser();
    p.validate_declaration(&decl).unwrap();
}

// ═══════════════════════════════════════════════════════════════════════
// SECTION 32: Mutable fields, per-variant untagged, per-variant rename_all
// ═══════════════════════════════════════════════════════════════════════

#[derive(Flow)]
struct VarianceFields {
    normal: String,
    #[flow(readonly)]
    readonly_field: i32,
}

#[test]
fn default_fields_have_no_variance() {
    let p = parser();
    let (_, ty) = parse_alias(&p, &VarianceFields::decl(&cfg()));
    for prop in props(&ty) {
        if let ObjectMember::ObjectTypeProperty { key, variance, .. } = prop {
            if key.name() == Some("normal") {
                assert!(variance.is_none(), "default field should have no variance (matches ts-rs)");
            }
            if key.name() == Some("readonly_field") {
                assert!(
                    variance.as_ref().map(|v| v.kind) == Some(VarianceKind::Plus),
                    "readonly field should be covariant (+)"
                );
            }
        }
    }
}

#[derive(Flow)]
#[flow(tag = "kind")]
enum MixedTagging {
    Tagged { x: i32 },
    #[flow(untagged)]
    Untagged { y: String },
}

#[test]
fn per_variant_untagged() {
    let p = parser();
    let c = cfg();
    let decl = MixedTagging::decl(&c);
    // Tagged variant has +kind: 'Tagged', untagged variant is just the object
    assert!(decl.contains("kind: 'Tagged'"), "tagged variant: {decl}");
    assert!(!decl.contains("kind: 'Untagged'"), "untagged variant should not have tag: {decl}");
    p.validate_declaration(&decl).unwrap();
}

#[derive(Flow)]
#[flow(tag = "t")]
enum PerVariantRenameAll {
    #[flow(rename_all = "camelCase")]
    CamelVariant { field_name: String, other_field: i32 },
    PlainVariant { field_name: String },
}

#[test]
fn per_variant_rename_all_fields() {
    let c = cfg();
    let decl = PerVariantRenameAll::decl(&c);
    // CamelVariant should have camelCase fields
    assert!(decl.contains("fieldName"), "camelCase applied: {decl}");
    // PlainVariant should have original field names
    assert!(decl.contains("field_name"), "plain kept: {decl}");
    let p = parser();
    p.validate_declaration(&decl).unwrap();
}

#[derive(Flow)]
#[flow(tag = "kind")]
enum WithVariantTypeOverride {
    Normal { x: i32 },
    #[flow(type = "string")]
    Overridden,
}

#[test]
fn per_variant_type_override() {
    let c = cfg();
    let decl = WithVariantTypeOverride::decl(&c);
    assert!(decl.contains("string"), "overridden variant: {decl}");
    let p = parser();
    p.validate_declaration(&decl).unwrap();
}

// ═══════════════════════════════════════════════════════════════════════
// FINAL: Batch parse — every type must produce valid Flow
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn every_type_produces_valid_flow() {
    let p = parser();
    let c = cfg();

    let decls: Vec<(&str, String)> = vec![
        // Primitives & structural
        ("AllPrimitives", AllPrimitives::decl(&c)),
        ("UnitStruct", UnitStruct::decl(&c)),
        ("WithPhantom", WithPhantom::decl(&c)),
        ("WithOptions", WithOptions::decl(&c)),
        ("AllCollections", AllCollections::decl(&c)),
        ("AllMaps", AllMaps::decl(&c)),
        ("WithTuples", WithTuples::decl(&c)),
        ("WithArrays", WithArrays::decl(&c)),
        ("AllWrappers", AllWrappers::decl(&c)),
        ("AllNonZero", AllNonZero::decl(&c)),
        ("StringLikes", StringLikes::decl(&c)),
        ("StructuralTypes", StructuralTypes::decl(&c)),
        ("WithResult", WithResult::decl(&c)),
        ("WithFunctions", WithFunctions::decl(&c)),
        // Struct variants
        ("Newtype", Newtype::decl(&c)),
        ("TupleStruct", TupleStruct::decl(&c)),
        ("Renamed", Renamed::decl(&c)),
        ("CamelFields", CamelFields::decl(&c)),
        ("KebabFields", KebabFields::decl(&c)),
        // Opaque
        ("OpaqueToken", OpaqueToken::decl(&c)),
        ("BoundedOpaque", BoundedOpaque::decl(&c)),
        // Enums (union)
        ("Untagged", Untagged::decl(&c)),
        ("InternallyTagged", InternallyTagged::decl(&c)),
        ("AdjacentlyTagged", AdjacentlyTagged::decl(&c)),
        ("ExternallyTagged", ExternallyTagged::decl(&c)),
        // Flow enums (all 4 repr)
        ("SymbolEnum", SymbolEnum::decl(&c)),
        ("StringEnum", StringEnum::decl(&c)),
        ("NumberEnum", NumberEnum::decl(&c)),
        ("AutoNumberEnum", AutoNumberEnum::decl(&c)),
        ("BoolEnum", BoolEnum::decl(&c)),
        // Generics
        ("Wrapper<Dummy>", Wrapper::<flowjs_rs::Dummy>::decl(&c)),
        // Flatten
        ("Base", Base::decl(&c)),
        ("Extended", Extended::decl(&c)),
        // Serde compat
        ("SerdeOptional", SerdeOptional::decl(&c)),
        // Enum field refs
        ("UsesEnums", UsesEnums::decl(&c)),
        // Cow
        ("WithCow", WithCow::decl(&c)),
        // Enum variant shapes × tagging (section 24)
        ("ExtTaggedFull", ExtTaggedFull::decl(&c)),
        ("IntTaggedFull", IntTaggedFull::decl(&c)),
        ("AdjTaggedFull", AdjTaggedFull::decl(&c)),
        ("UntaggedFull", UntaggedFull::decl(&c)),
        // Container overrides (section 25)
        ("TypeOverrideContainer", TypeOverrideContainer::decl(&c)),
        ("AsOverrideContainer", AsOverrideContainer::decl(&c)),
        // Field attributes (section 26)
        ("FieldSkip", FieldSkip::decl(&c)),
        ("FieldTypeOverride", FieldTypeOverride::decl(&c)),
        ("FieldAsOverride", FieldAsOverride::decl(&c)),
        ("FieldInline", FieldInline::decl(&c)),
        ("FieldRename", FieldRename::decl(&c)),
        // Rename inflections (section 27)
        ("UpperFields", UpperFields::decl(&c)),
        ("SnakeFields", SnakeFields::decl(&c)),
        ("PascalFields", PascalFields::decl(&c)),
        ("ScreamSnakeFields", ScreamSnakeFields::decl(&c)),
        ("ScreamKebabFields", ScreamKebabFields::decl(&c)),
        // Multi-generic, empty, negative disc (section 28)
        ("MultiGeneric", MultiGeneric::<flowjs_rs::Dummy, flowjs_rs::Dummy>::decl(&c)),
        ("SingleFieldStruct", SingleFieldStruct::decl(&c)),
        ("EmptyEnum", EmptyEnum::decl(&c)),
        ("NegativeDisc", NegativeDisc::decl(&c)),
        ("EmptyFlowEnum", EmptyFlowEnum::decl(&c)),
        // Doc comments (section 29)
        ("Documented", Documented::decl(&c)),
        // Large array (section 30)
        ("WithLargeArray", WithLargeArray::decl(&c)),
        // Variant skip/rename (section 31)
        ("WithSkippedVariant", WithSkippedVariant::decl(&c)),
        ("WithRenamedVariant", WithRenamedVariant::decl(&c)),
        // Mutable fields, per-variant attrs (section 32)
        ("VarianceFields", VarianceFields::decl(&c)),
        ("MixedTagging", MixedTagging::decl(&c)),
        ("PerVariantRenameAll", PerVariantRenameAll::decl(&c)),
        ("WithVariantTypeOverride", WithVariantTypeOverride::decl(&c)),
    ];

    let mut failures = Vec::new();
    for (name, decl) in &decls {
        if let Err(e) = p.validate_declaration(decl) {
            failures.push(format!("{name}: {e}\n  {decl}"));
        }
    }

    assert!(
        failures.is_empty(),
        "Flow parse failures ({}/{}):\n{}",
        failures.len(),
        decls.len(),
        failures.join("\n\n")
    );

    eprintln!("validated {}/{} types through flow parser", decls.len(), decls.len());
}
