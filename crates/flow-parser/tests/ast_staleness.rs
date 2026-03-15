//! AST staleness detection.
//!
//! Parses a comprehensive set of Flow source constructs and asserts that
//! NO nodes deserialize to `Other`. When the flow-parser npm package adds
//! new AST node types, this test will fail — signaling that `ast.rs` needs
//! a new variant added.
//!
//! This is the deterministic mechanism that keeps the AST up to date
//! without human intervention: CI runs this test against the latest
//! flow-parser, and any new node type causes a clear, actionable failure.

use flowjs_parser::{FlowParser, Statement, Declaration, TypeAnnotation, ObjectMember, EnumBody, EnumMember};

fn parser() -> FlowParser {
    FlowParser::new().unwrap()
}

/// Recursively walk a TypeAnnotation and panic if any node is `Other`.
fn assert_no_other_type(ty: &TypeAnnotation, context: &str) {
    match ty {
        TypeAnnotation::Other => {
            panic!("TypeAnnotation::Other found in: {context}. A new Flow type node needs to be added to ast.rs");
        }
        TypeAnnotation::NullableTypeAnnotation { type_annotation } => {
            assert_no_other_type(type_annotation, &format!("{context} > NullableTypeAnnotation"));
        }
        TypeAnnotation::ObjectTypeAnnotation { properties, indexers, call_properties, internal_slots, .. } => {
            for (i, p) in properties.iter().enumerate() {
                assert_no_other_member(p, &format!("{context} > property[{i}]"));
            }
            for (i, idx) in indexers.iter().enumerate() {
                assert_no_other_type(&idx.key, &format!("{context} > indexer[{i}].key"));
                assert_no_other_type(&idx.value, &format!("{context} > indexer[{i}].value"));
            }
            for (i, cp) in call_properties.iter().enumerate() {
                assert_no_other_type(&cp.value, &format!("{context} > callProperty[{i}]"));
            }
            for (i, slot) in internal_slots.iter().enumerate() {
                assert_no_other_type(&slot.value, &format!("{context} > internalSlot[{i}]"));
            }
        }
        TypeAnnotation::UnionTypeAnnotation { types }
        | TypeAnnotation::IntersectionTypeAnnotation { types } => {
            for (i, t) in types.iter().enumerate() {
                assert_no_other_type(t, &format!("{context} > member[{i}]"));
            }
        }
        TypeAnnotation::GenericTypeAnnotation { type_parameters, .. } => {
            if let Some(tp) = type_parameters {
                for (i, p) in tp.params.iter().enumerate() {
                    assert_no_other_type(p, &format!("{context} > typeParam[{i}]"));
                }
            }
        }
        TypeAnnotation::TupleTypeAnnotation { element_types } => {
            for (i, t) in element_types.iter().enumerate() {
                assert_no_other_type(t, &format!("{context} > element[{i}]"));
            }
        }
        TypeAnnotation::ArrayTypeAnnotation { element_type } => {
            assert_no_other_type(element_type, &format!("{context} > elementType"));
        }
        TypeAnnotation::TypeofTypeAnnotation { .. } => {
            // typeof arguments are value-level expressions (Identifier, MemberExpression),
            // not type-level nodes. They may deserialize as Other, which is expected.
        }
        TypeAnnotation::FunctionTypeAnnotation { params, return_type, rest, this_constraint, .. } => {
            for (i, p) in params.iter().enumerate() {
                assert_no_other_type(&p.type_annotation, &format!("{context} > param[{i}]"));
            }
            assert_no_other_type(return_type, &format!("{context} > returnType"));
            if let Some(r) = rest {
                assert_no_other_type(&r.type_annotation, &format!("{context} > rest"));
            }
            if let Some(t) = this_constraint {
                assert_no_other_type(&t.type_annotation, &format!("{context} > this"));
            }
        }
        TypeAnnotation::InterfaceTypeAnnotation { body, .. } => {
            assert_no_other_type(body, &format!("{context} > interfaceBody"));
        }
        TypeAnnotation::IndexedAccessType { object_type, index_type }
        | TypeAnnotation::OptionalIndexedAccessType { object_type, index_type, .. } => {
            assert_no_other_type(object_type, &format!("{context} > objectType"));
            assert_no_other_type(index_type, &format!("{context} > indexType"));
        }
        TypeAnnotation::KeyofTypeAnnotation { argument } => {
            assert_no_other_type(argument, &format!("{context} > keyof"));
        }
        TypeAnnotation::ConditionalTypeAnnotation { check_type, extends_type, true_type, false_type } => {
            assert_no_other_type(check_type, &format!("{context} > checkType"));
            assert_no_other_type(extends_type, &format!("{context} > extendsType"));
            assert_no_other_type(true_type, &format!("{context} > trueType"));
            assert_no_other_type(false_type, &format!("{context} > falseType"));
        }
        TypeAnnotation::TypeOperator { type_annotation, .. } => {
            assert_no_other_type(type_annotation, &format!("{context} > typeOperator"));
        }
        // Leaf nodes — no children to recurse into
        TypeAnnotation::StringTypeAnnotation
        | TypeAnnotation::NumberTypeAnnotation
        | TypeAnnotation::BooleanTypeAnnotation
        | TypeAnnotation::VoidTypeAnnotation
        | TypeAnnotation::MixedTypeAnnotation
        | TypeAnnotation::AnyTypeAnnotation
        | TypeAnnotation::EmptyTypeAnnotation
        | TypeAnnotation::NullLiteralTypeAnnotation
        | TypeAnnotation::BigIntTypeAnnotation
        | TypeAnnotation::SymbolTypeAnnotation
        | TypeAnnotation::StringLiteralTypeAnnotation { .. }
        | TypeAnnotation::NumberLiteralTypeAnnotation { .. }
        | TypeAnnotation::BooleanLiteralTypeAnnotation { .. }
        | TypeAnnotation::BigIntLiteralTypeAnnotation { .. }
        | TypeAnnotation::ExistsTypeAnnotation
        | TypeAnnotation::InferTypeAnnotation { .. }
        | TypeAnnotation::ComponentTypeAnnotation { .. } => {}
    }
}

fn assert_no_other_member(member: &ObjectMember, context: &str) {
    match member {
        ObjectMember::Other => {
            panic!("ObjectMember::Other found in: {context}. A new object member node needs to be added to ast.rs");
        }
        ObjectMember::ObjectTypeProperty { value, .. } => {
            assert_no_other_type(value, &format!("{context} > value"));
        }
        ObjectMember::ObjectTypeSpreadProperty { argument } => {
            assert_no_other_type(argument, &format!("{context} > spread"));
        }
    }
}

fn assert_no_other_declaration(decl: &Declaration, context: &str) {
    match decl {
        Declaration::Other => {
            panic!("Declaration::Other found in: {context}. A new declaration node needs to be added to ast.rs");
        }
        Declaration::TypeAlias { right, .. } => {
            assert_no_other_type(right, &format!("{context} > TypeAlias.right"));
        }
        Declaration::OpaqueType { supertype, .. }
        | Declaration::DeclareOpaqueType { supertype, .. } => {
            if let Some(st) = supertype {
                assert_no_other_type(st, &format!("{context} > supertype"));
            }
        }
        Declaration::EnumDeclaration { body, .. } => {
            assert_no_other_enum_body(body, context);
        }
        Declaration::InterfaceDeclaration { body, .. } => {
            assert_no_other_type(body, &format!("{context} > InterfaceDeclaration.body"));
        }
        Declaration::DeclareTypeAlias { right, .. } => {
            assert_no_other_type(right, &format!("{context} > DeclareTypeAlias.right"));
        }
        Declaration::DeclareClass { .. }
        | Declaration::DeclareFunction { .. }
        | Declaration::DeclareVariable { .. }
        | Declaration::DeclareModule { .. } => {}
    }
}

fn assert_no_other_enum_body(body: &EnumBody, context: &str) {
    match body {
        EnumBody::Other => {
            panic!("EnumBody::Other found in: {context}. A new enum body node needs to be added to ast.rs");
        }
        _ => {
            for (i, m) in body.members().iter().enumerate() {
                if matches!(m, EnumMember::Other) {
                    panic!("EnumMember::Other found in: {context} > member[{i}]. A new enum member node needs to be added to ast.rs");
                }
            }
        }
    }
}

fn assert_no_other_statement(stmt: &Statement, context: &str) {
    match stmt {
        Statement::Other => {
            // Statement::Other is acceptable for statement types we intentionally don't model
            // (e.g., VariableDeclaration, ExpressionStatement). Only declarations matter.
        }
        Statement::ExportNamedDeclaration { declaration: Some(d) }
        | Statement::DeclareExportDeclaration { declaration: Some(d) } => {
            assert_no_other_declaration(d, context);
        }
        Statement::EnumDeclaration { body, .. } => {
            assert_no_other_enum_body(body, context);
        }
        _ => {}
    }
}

/// Parse a Flow source and assert no `Other` nodes in the type-level AST.
fn validate_no_others(p: &FlowParser, label: &str, source: &str) {
    let program = p
        .parse(source)
        .unwrap_or_else(|e| panic!("[{label}] parse failed: {e}\n\nsource:\n{source}"));

    for (i, stmt) in program.body.iter().enumerate() {
        assert_no_other_statement(stmt, &format!("[{label}] body[{i}]"));
    }
}

// ── Test: comprehensive Flow constructs ─────────────────────────────────

#[test]
fn no_other_nodes_in_type_aliases() {
    let p = parser();
    validate_no_others(&p, "primitives", "
        export type T1 = string;
        export type T2 = number;
        export type T3 = boolean;
        export type T4 = void;
        export type T5 = mixed;
        export type T6 = any;
        export type T7 = empty;
        export type T8 = null;
        export type T9 = bigint;
        export type T10 = symbol;
    ");
}

#[test]
fn no_other_nodes_in_literals() {
    let p = parser();
    validate_no_others(&p, "literals", "
        export type T1 = 'hello';
        export type T2 = 42;
        export type T3 = true;
        export type T4 = false;
    ");
}

#[test]
fn no_other_nodes_in_objects() {
    let p = parser();
    validate_no_others(&p, "objects", "
        export type T1 = {| +x: number, +y: string |};
        export type T2 = { x: number, y?: string };
        export type T3 = { [key: string]: number };
        export type T4 = { ...T1 };
    ");
}

#[test]
fn no_other_nodes_in_unions_intersections() {
    let p = parser();
    validate_no_others(&p, "unions_intersections", "
        export type T1 = string | number | boolean;
        export type T2 = {| +a: number |} & {| +b: string |};
        export type T3 = 'a' | 'b' | 'c';
    ");
}

#[test]
fn no_other_nodes_in_generics() {
    let p = parser();
    validate_no_others(&p, "generics", "
        export type T1 = Array<string>;
        export type T2 = $ReadOnlyArray<number>;
        export type T3<T> = {| +value: T |};
        export type T4 = Map<string, number>;
    ");
}

#[test]
fn no_other_nodes_in_functions() {
    let p = parser();
    validate_no_others(&p, "functions", "
        export type T1 = () => void;
        export type T2 = (x: string) => number;
        export type T3 = (a: string, b: number, c: boolean) => void;
        export type T4 = (...args: Array<string>) => void;
    ");
}

#[test]
fn no_other_nodes_in_tuples_arrays() {
    let p = parser();
    validate_no_others(&p, "tuples_arrays", "
        export type T1 = [string, number];
        export type T2 = [boolean, string, number];
        export type T3 = number[];
    ");
}

#[test]
fn no_other_nodes_in_nullable() {
    let p = parser();
    validate_no_others(&p, "nullable", "
        export type T1 = ?string;
        export type T2 = ?{| +x: number |};
    ");
}

#[test]
fn no_other_nodes_in_opaque_types() {
    let p = parser();
    validate_no_others(&p, "opaque", "
        declare export opaque type Token;
        declare export opaque type Id: string;
    ");
}

#[test]
fn no_other_nodes_in_enums() {
    let p = parser();
    validate_no_others(&p, "enums", "
        enum Direction { Up, Down, Left, Right, }
        enum Status of string { Active = 'active', Paused = 'paused', }
        enum Code of number { Ok = 200, NotFound = 404, }
        enum Flag of boolean { Yes = true, No = false, }
    ");
}

#[test]
fn no_other_nodes_in_interfaces() {
    let p = parser();
    validate_no_others(&p, "interfaces", "
        export interface Foo { +x: number; +y: string; }
        export interface Bar extends Foo { +z: boolean; }
    ");
}

#[test]
fn no_other_nodes_in_typeof() {
    let p = parser();
    // typeof at the type level references values, not types — the argument
    // may use value-level AST nodes (Identifier, MemberExpression) that differ
    // from type-level nodes. Test the TypeofTypeAnnotation wrapper only.
    validate_no_others(&p, "typeof", "
        export type T = typeof undefined;
    ");
}

#[test]
fn no_other_nodes_in_indexed_access() {
    let p = parser();
    validate_no_others(&p, "indexed_access", "
        export type Obj = {| +x: number, +y: string |};
        export type T = Obj['x'];
    ");
}

#[test]
fn no_other_nodes_in_keyof() {
    let p = parser();
    validate_no_others(&p, "keyof", "
        export type Obj = {| +x: number |};
        export type T = keyof Obj;
    ");
}
