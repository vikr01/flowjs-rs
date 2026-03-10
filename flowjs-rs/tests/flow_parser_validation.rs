//! Validate generated Flow declarations against the official Flow parser.
//!
//! These tests shell out to `node` with the `flow-parser` npm package.
//! They are `#[ignore]`d by default so CI does not require node/flow-parser.
//! Run explicitly: `cargo test -- --ignored`

use flowjs_rs::{Config, Flow};
use std::process::Command;

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

// ── Helpers ─────────────────────────────────────────────────────────────

/// Check that `node` and `flow-parser` are available. Returns the NODE_PATH
/// value to use, or None if unavailable.
fn flow_parser_available() -> Option<String> {
    let node_path_output = Command::new("npm").args(["root", "-g"]).output().ok()?;
    if !node_path_output.status.success() {
        return None;
    }
    let node_path = String::from_utf8(node_path_output.stdout).ok()?.trim().to_owned();

    let check = Command::new("node")
        .env("NODE_PATH", &node_path)
        .args([
            "-e",
            "try { require('flow-parser'); process.exit(0) } catch(e) { process.exit(1) }",
        ])
        .status()
        .ok()?;
    if check.success() {
        Some(node_path)
    } else {
        None
    }
}

/// Parse a Flow declaration string through the official parser.
/// Returns Ok(()) on success, Err(error_messages) on parse failure.
fn validate_flow(node_path: &str, flow_source: &str) -> Result<(), String> {
    let script = format!(
        r#"
const fp = require('flow-parser');
const code = {};
const ast = fp.parse(code, {{ types: true }});
if (ast.errors.length > 0) {{
    console.error(JSON.stringify(ast.errors, null, 2));
    process.exit(1);
}} else {{
    process.exit(0);
}}
"#,
        serde_json::to_string(flow_source).unwrap()
    );

    let output = Command::new("node")
        .env("NODE_PATH", node_path)
        .args(["-e", &script])
        .output()
        .map_err(|e| format!("Failed to run node: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "Flow parse errors for:\n{flow_source}\n\nErrors:\n{stderr}"
        ))
    }
}

/// Wrap a type declaration in `// @flow` + `export` like the real export logic.
fn as_flow_file(decl: &str) -> String {
    format!("// @flow\nexport {decl}\n")
}

// ── Tests ───────────────────────────────────────────────────────────────

#[test]
#[ignore]
fn validate_simple_struct() {
    // Arrange
    let node_path = flow_parser_available().expect("node + flow-parser required");
    let cfg = Config::new();
    let source = as_flow_file(&SimpleStruct::decl(&cfg));

    // Act and Assert
    validate_flow(&node_path, &source).expect("SimpleStruct should produce valid Flow");
}

#[test]
#[ignore]
fn validate_option_fields() {
    // Arrange
    let node_path = flow_parser_available().expect("node + flow-parser required");
    let cfg = Config::new();
    let source = as_flow_file(&WithOption::decl(&cfg));

    // Act and Assert
    validate_flow(&node_path, &source).expect("WithOption should produce valid Flow");
}

#[test]
#[ignore]
fn validate_vec_fields() {
    // Arrange
    let node_path = flow_parser_available().expect("node + flow-parser required");
    let cfg = Config::new();
    let source = as_flow_file(&WithVec::decl(&cfg));

    // Act and Assert
    validate_flow(&node_path, &source).expect("WithVec should produce valid Flow");
}

#[test]
#[ignore]
fn validate_nested_struct() {
    // Arrange
    let node_path = flow_parser_available().expect("node + flow-parser required");
    let cfg = Config::new();
    let source = as_flow_file(&Nested::decl(&cfg));

    // Act and Assert
    validate_flow(&node_path, &source).expect("Nested should produce valid Flow");
}

#[test]
#[ignore]
fn validate_newtype() {
    // Arrange
    let node_path = flow_parser_available().expect("node + flow-parser required");
    let cfg = Config::new();
    let source = as_flow_file(&Newtype::decl(&cfg));

    // Act and Assert
    validate_flow(&node_path, &source).expect("Newtype should produce valid Flow");
}

#[test]
#[ignore]
fn validate_unit_struct() {
    // Arrange
    let node_path = flow_parser_available().expect("node + flow-parser required");
    let cfg = Config::new();
    let source = as_flow_file(&UnitStruct::decl(&cfg));

    // Act and Assert
    validate_flow(&node_path, &source).expect("UnitStruct should produce valid Flow");
}

#[test]
#[ignore]
fn validate_camel_case_rename() {
    // Arrange
    let node_path = flow_parser_available().expect("node + flow-parser required");
    let cfg = Config::new();
    let source = as_flow_file(&CamelCase::decl(&cfg));

    // Act and Assert
    validate_flow(&node_path, &source).expect("CamelCase should produce valid Flow");
}

#[test]
#[ignore]
fn validate_untagged_enum() {
    // Arrange
    let node_path = flow_parser_available().expect("node + flow-parser required");
    let cfg = Config::new();
    let source = as_flow_file(&UntaggedUnion::decl(&cfg));

    // Act and Assert
    validate_flow(&node_path, &source).expect("UntaggedUnion should produce valid Flow");
}

#[test]
#[ignore]
fn validate_tagged_enum() {
    // Arrange
    let node_path = flow_parser_available().expect("node + flow-parser required");
    let cfg = Config::new();
    let source = as_flow_file(&TaggedUnion::decl(&cfg));

    // Act and Assert
    validate_flow(&node_path, &source).expect("TaggedUnion should produce valid Flow");
}

/// Batch validation: generate all types and validate in a single node invocation
/// for faster execution.
#[test]
#[ignore]
fn validate_all_types_batch() {
    // Arrange
    let node_path = flow_parser_available().expect("node + flow-parser required");
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
        if let Err(e) = validate_flow(&node_path, &source) {
            failures.push(format!("{name}: {e}"));
        }
    }

    assert!(
        failures.is_empty(),
        "Flow parse failures:\n{}",
        failures.join("\n\n")
    );
}
