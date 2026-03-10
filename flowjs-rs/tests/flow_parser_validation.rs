//! Validate generated Flow declarations against the official Flow parser.
//!
//! Uses the `flow-parser` npm package (js_of_ocaml output) executed via
//! `quick-js` (QuickJS engine). No node/npm required at test time; the
//! vendored `flow_parser.js` is embedded at compile time.

use flowjs_rs::{Config, Flow};
use quick_js::Context;

// ── Vendored parser ────────────────────────────────────────────────────

const FLOW_PARSER_JS: &str = include_str!("fixtures/flow_parser.js");

/// Parse a Flow source string through the official parser.
/// Returns `Ok(())` on success, `Err(error_messages)` on parse failure.
fn validate_flow(flow_source: &str) -> Result<(), String> {
    let ctx = Context::new().map_err(|e| format!("Failed to create JS context: {e}"))?;

    // Provide CommonJS `exports` + console stubs (js_of_ocaml runtime needs console)
    ctx.eval("var exports = {}; var console = { log: function(){}, error: function(){}, warn: function(){} };")
        .map_err(|e| format!("Failed to init shims: {e}"))?;

    // Load the parser
    ctx.eval(FLOW_PARSER_JS)
        .map_err(|e| format!("Failed to load flow_parser.js: {e}"))?;

    let script = format!(
        r#"
(function() {{
    var code = {};
    var ast = exports.parse(code, {{ types: true }});
    if (ast.errors && ast.errors.length > 0) {{
        return JSON.stringify(ast.errors);
    }}
    return "";
}})()
"#,
        serde_json::to_string(flow_source).unwrap()
    );

    let result = ctx
        .eval_as::<String>(&script)
        .map_err(|e| format!("Failed to run parse: {e}"))?;

    if result.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Flow parse errors for:\n{flow_source}\n\nErrors:\n{result}"
        ))
    }
}

/// Wrap a type declaration in `// @flow` + `export` like the real export logic.
fn as_flow_file(decl: &str) -> String {
    format!("// @flow\nexport {decl}\n")
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

// ── Tests ───────────────────────────────────────────────────────────────

#[test]
fn validate_simple_struct() {
    // Arrange
    let cfg = Config::new();
    let source = as_flow_file(&SimpleStruct::decl(&cfg));

    // Act and Assert
    validate_flow(&source).expect("SimpleStruct should produce valid Flow");
}

#[test]
fn validate_option_fields() {
    // Arrange
    let cfg = Config::new();
    let source = as_flow_file(&WithOption::decl(&cfg));

    // Act and Assert
    validate_flow(&source).expect("WithOption should produce valid Flow");
}

#[test]
fn validate_vec_fields() {
    // Arrange
    let cfg = Config::new();
    let source = as_flow_file(&WithVec::decl(&cfg));

    // Act and Assert
    validate_flow(&source).expect("WithVec should produce valid Flow");
}

#[test]
fn validate_nested_struct() {
    // Arrange
    let cfg = Config::new();
    let source = as_flow_file(&Nested::decl(&cfg));

    // Act and Assert
    validate_flow(&source).expect("Nested should produce valid Flow");
}

#[test]
fn validate_newtype() {
    // Arrange
    let cfg = Config::new();
    let source = as_flow_file(&Newtype::decl(&cfg));

    // Act and Assert
    validate_flow(&source).expect("Newtype should produce valid Flow");
}

#[test]
fn validate_unit_struct() {
    // Arrange
    let cfg = Config::new();
    let source = as_flow_file(&UnitStruct::decl(&cfg));

    // Act and Assert
    validate_flow(&source).expect("UnitStruct should produce valid Flow");
}

#[test]
fn validate_camel_case_rename() {
    // Arrange
    let cfg = Config::new();
    let source = as_flow_file(&CamelCase::decl(&cfg));

    // Act and Assert
    validate_flow(&source).expect("CamelCase should produce valid Flow");
}

#[test]
fn validate_untagged_enum() {
    // Arrange
    let cfg = Config::new();
    let source = as_flow_file(&UntaggedUnion::decl(&cfg));

    // Act and Assert
    validate_flow(&source).expect("UntaggedUnion should produce valid Flow");
}

#[test]
fn validate_tagged_enum() {
    // Arrange
    let cfg = Config::new();
    let source = as_flow_file(&TaggedUnion::decl(&cfg));

    // Act and Assert
    validate_flow(&source).expect("TaggedUnion should produce valid Flow");
}

/// Batch validation: generate all types and validate in a single engine instance.
#[test]
fn validate_all_types_batch() {
    // Arrange
    let cfg = Config::new();

    let decls: Vec<(&str, String)> = vec![
        ("SimpleStruct", SimpleStruct::decl(&cfg)),
        ("WithOption", WithOption::decl(&cfg)),
        ("WithVec", WithVec::decl(&cfg)),
        ("Nested", Nested::decl(&cfg)),
        ("Newtype", Newtype::decl(&cfg)),
        ("UnitStruct", UnitStruct::decl(&cfg)),
        ("CamelCase", CamelCase::decl(&cfg)),
        ("UntaggedUnion", UntaggedUnion::decl(&cfg)),
        ("TaggedUnion", TaggedUnion::decl(&cfg)),
    ];

    // Act and Assert
    let mut failures = Vec::new();
    for (name, decl) in &decls {
        let source = as_flow_file(decl);
        if let Err(e) = validate_flow(&source) {
            failures.push(format!("{name}: {e}"));
        }
    }

    assert!(
        failures.is_empty(),
        "Flow parse failures:\n{}",
        failures.join("\n\n")
    );
}
