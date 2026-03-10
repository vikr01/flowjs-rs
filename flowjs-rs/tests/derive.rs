//! Integration tests for the Flow derive macro.

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

#[test]
fn simple_struct_decl() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = Simple::decl(&cfg);

    // Assert
    assert!(decl.contains("type Simple"), "should declare type Simple");
    assert!(decl.contains("+name: string"), "should have +name field");
    assert!(decl.contains("+age: number"), "should have +age field");
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
    assert!(
        decl.contains("+userId: number"),
        "should use renamed field"
    );
}

#[test]
fn camel_case_fields() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = CamelFields::decl(&cfg);

    // Assert
    assert!(
        decl.contains("+firstName: string"),
        "should camelCase first_name"
    );
    assert!(
        decl.contains("+lastName: string"),
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
        decl.contains("+bio?: ?string"),
        "Option<String> should become optional nullable"
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
        decl, "declare export opaque type OpaqueToken;",
        "should be fully opaque"
    );
}

#[test]
fn opaque_type_bounded() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = BoundedOpaque::decl(&cfg);

    // Assert
    assert_eq!(
        decl, "declare export opaque type BoundedOpaque: string;",
        "should be bounded opaque"
    );
}

#[test]
fn unit_struct() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = UnitStruct::decl(&cfg);

    // Assert
    assert_eq!(
        decl, "type UnitStruct = void;",
        "unit struct should be void"
    );
}

#[test]
fn untagged_enum() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = Shape::decl(&cfg);

    // Assert
    assert!(decl.contains("type Shape"), "should declare Shape");
    assert!(decl.contains("+radius: number"), "should have radius field");
    assert!(decl.contains("+width: number"), "should have width field");
    assert!(decl.contains(" | "), "should be union");
}

#[test]
fn tagged_enum() {
    // Arrange and Act
    let cfg = Config::new();
    let decl = TaggedEvent::decl(&cfg);

    // Assert
    assert!(decl.contains("+kind: 'Click'"), "should have Click tag");
    assert!(decl.contains("+kind: 'Resize'"), "should have Resize tag");
    assert!(
        decl.contains("+width: number"),
        "Resize should have width field"
    );
}
