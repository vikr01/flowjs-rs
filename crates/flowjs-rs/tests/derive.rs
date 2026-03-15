//! Integration tests for the Flow derive macro.

#![allow(dead_code)]

use flowjs_rs::{Config, Flow};

#[derive(Flow)]
struct Simple {
    name: String,
    age: i32,
}

#[derive(Flow)]
#[flow(rename = "RenamedUser")]
struct User {
    #[flow(rename = "userId")]
    user_id: i32,
    first_name: String,
}

#[derive(Flow)]
#[flow(rename_all = "camelCase")]
struct CamelFields {
    first_name: String,
    last_name: String,
}

#[derive(Flow)]
struct WithOption {
    name: String,
    bio: Option<String>,
}

#[derive(Flow)]
struct WithVec {
    tags: Vec<String>,
}

#[derive(Flow)]
struct Newtype(String);

#[derive(Flow)]
#[flow(opaque)]
struct OpaqueToken(String);

#[derive(Flow)]
#[flow(opaque = "string")]
struct BoundedOpaque(String);

#[derive(Flow)]
#[flow(opaque)]
struct OpaqueWrapper<T: Flow> {
    thing: T,
}

#[derive(Flow)]
struct UnitStruct;

#[derive(Flow)]
#[flow(untagged)]
enum Shape {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
}

#[derive(Flow)]
#[flow(tag = "kind")]
enum TaggedEvent {
    Click,
    Resize { width: i32, height: i32 },
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
enum Status {
    Active,
    Paused,
    Off,
}

#[derive(Flow)]
#[flow(flow_enum = "string")]
enum Color {
    Red,
    #[flow(rename = "green_color")]
    Green,
    Blue,
}

#[derive(Flow)]
#[flow(flow_enum)]
enum EmptyEnum {}

#[derive(Flow)]
#[flow(flow_enum = "number")]
#[repr(i32)]
enum HttpStatus {
    Ok = 200,
    NotFound = 404,
    Internal = 500,
}

#[derive(Flow)]
#[flow(flow_enum = "number")]
enum AutoIncrement {
    A,
    B,
    C,
}

#[derive(Flow)]
#[flow(flow_enum = "boolean")]
enum Toggle {
    On,
    Off,
}

#[derive(Flow)]
struct WithCallback {
    handler: fn(String, i32) -> bool,
}

#[test]
fn simple_struct_decl() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = Simple::decl(&cfg);

    // Assert
    assert!(decl.contains("type Simple"), "should declare type Simple");
    assert!(decl.contains("name: string"), "should have +name field");
    assert!(decl.contains("age: number"), "should have +age field");
}

#[test]
fn renamed_struct() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = User::decl(&cfg);

    // Assert
    assert!(
        decl.contains("type RenamedUser"),
        "should use renamed type name"
    );
    assert!(decl.contains("userId: number"), "should use renamed field");
}

#[test]
fn camel_case_fields() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = CamelFields::decl(&cfg);

    // Assert
    assert!(
        decl.contains("firstName: string"),
        "should camelCase first_name"
    );
    assert!(
        decl.contains("lastName: string"),
        "should camelCase last_name"
    );
}

#[test]
fn option_becomes_nullable() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = WithOption::decl(&cfg);

    // Assert
    assert!(
        decl.contains("bio: ?string"),
        "Option<String> without serde skip should be always-present nullable"
    );
}

#[test]
fn vec_becomes_readonlyarray() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = WithVec::decl(&cfg);

    // Assert
    assert!(
        decl.contains("$ReadOnlyArray<string>"),
        "Vec<String> should become $ReadOnlyArray<string>"
    );
}

#[test]
fn newtype_inlines_inner() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = Newtype::decl(&cfg);

    // Assert
    assert_eq!(
        decl, "type Newtype = string;",
        "newtype should inline to inner type"
    );
}

#[test]
fn opaque_type_fully() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = OpaqueToken::decl(&cfg);

    // Assert
    assert_eq!(
        decl, "opaque type OpaqueToken = string;",
        "opaque newtype with body"
    );
}

#[test]
fn opaque_type_bounded() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = BoundedOpaque::decl(&cfg);

    // Assert
    assert_eq!(
        decl, "opaque type BoundedOpaque: string = string;",
        "opaque with bound and body"
    );
}

#[test]
fn generic_opaque_type() {
    let cfg = Config::new();
    let decl = OpaqueWrapper::<flowjs_rs::Dummy>::decl(&cfg);
    eprintln!("GENERIC OPAQUE: {decl}");
    assert!(decl.contains("opaque type OpaqueWrapper<T>"), "should have generic param: {decl}");
    assert!(decl.contains("thing: T"), "should have body with generic field: {decl}");
}

#[test]
fn unit_struct() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = UnitStruct::decl(&cfg);

    // Assert
    assert_eq!(
        decl, "type UnitStruct = null;",
        "unit struct should be null (matches ts-rs)"
    );
}

#[test]
fn untagged_enum() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = Shape::decl(&cfg);

    // Assert
    assert!(decl.contains("type Shape"), "should declare Shape");
    assert!(decl.contains("radius: number"), "should have radius field");
    assert!(decl.contains("width: number"), "should have width field");
    assert!(decl.contains(" | "), "should be union");
}

#[test]
fn tagged_enum() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = TaggedEvent::decl(&cfg);

    // Assert
    assert!(decl.contains("kind: 'Click'"), "should have Click tag");
    assert!(decl.contains("kind: 'Resize'"), "should have Resize tag");
    assert!(
        decl.contains("width: number"),
        "Resize should have width field"
    );
}

#[test]
fn flow_enum_symbol() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = Direction::decl(&cfg);

    // Assert
    assert!(decl.starts_with("enum Direction {"), "should be enum declaration, got: {decl}");
    assert!(decl.contains("Up,"), "should have Up member");
    assert!(decl.contains("Down,"), "should have Down member");
    assert!(decl.contains("Left,"), "should have Left member");
    assert!(decl.contains("Right,"), "should have Right member");
    assert!(!decl.contains("type "), "should not use type alias syntax");
}

#[test]
fn flow_enum_string_with_rename() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = Status::decl(&cfg);

    // Assert
    assert!(
        decl.starts_with("enum Status of string {"),
        "should be string enum, got: {decl}"
    );
    assert!(decl.contains("Active = 'active',"), "Active with lowercase value");
    assert!(decl.contains("Paused = 'paused',"), "Paused with lowercase value");
    assert!(decl.contains("Off = 'off',"), "Off with lowercase value");
}

#[test]
fn flow_enum_string_per_variant_rename() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = Color::decl(&cfg);

    // Assert
    assert!(decl.contains("Red = 'Red',"), "Red keeps original name");
    assert!(decl.contains("Green = 'green_color',"), "Green uses per-variant rename");
    assert!(decl.contains("Blue = 'Blue',"), "Blue keeps original name");
}

#[test]
fn flow_enum_inline_is_name() {
    // Arrange and Act
    let cfg = Config::new();

    // Assert — flow enum inline() returns the name, not an expansion
    assert_eq!(Direction::inline(&cfg), "Direction", "inline should be type name");
    assert_eq!(Status::inline(&cfg), "Status", "inline should be type name");
}

#[test]
fn flow_enum_is_enum() {
    // Assert
    assert!(Direction::IS_ENUM, "flow enum should report IS_ENUM = true");
}

#[test]
fn flow_enum_number_with_discriminants() {
    let cfg = Config::new();
    let decl = HttpStatus::decl(&cfg);
    assert!(decl.starts_with("enum HttpStatus of number {"), "should be number enum: {decl}");
    assert!(decl.contains("Ok = 200,"), "Ok = 200: {decl}");
    assert!(decl.contains("NotFound = 404,"), "NotFound = 404: {decl}");
    assert!(decl.contains("Internal = 500,"), "Internal = 500: {decl}");
}

#[test]
fn flow_enum_number_auto_increment() {
    let cfg = Config::new();
    let decl = AutoIncrement::decl(&cfg);
    assert!(decl.contains("A = 0,"), "A = 0: {decl}");
    assert!(decl.contains("B = 1,"), "B = 1: {decl}");
    assert!(decl.contains("C = 2,"), "C = 2: {decl}");
}

#[test]
fn flow_enum_boolean() {
    let cfg = Config::new();
    let decl = Toggle::decl(&cfg);
    assert!(decl.starts_with("enum Toggle of boolean {"), "should be boolean enum: {decl}");
    assert!(decl.contains("On = true,"), "On = true: {decl}");
    assert!(decl.contains("Off = false,"), "Off = false: {decl}");
}

#[test]
fn fn_pointer_produces_function_type() {
    let cfg = Config::new();
    let decl = WithCallback::decl(&cfg);
    assert!(
        decl.contains("(arg0: string, arg1: number) => boolean"),
        "fn pointer should become Flow function type: {decl}"
    );
}
