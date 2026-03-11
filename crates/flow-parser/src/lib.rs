//! Typed Rust bindings for Facebook's Flow parser via QuickJS.
//!
//! ```rust
//! use flow_parser::FlowParser;
//!
//! let parser = FlowParser::new().unwrap();
//!
//! // Parse and get a typed AST
//! let ast = parser.parse("type Foo = string;").unwrap();
//!
//! // Validate that source is syntactically valid Flow
//! parser.validate("type Bar = {| +x: number |}").unwrap();
//! ```

mod ast;

pub use ast::*;

use quick_js::Context;

const FLOW_PARSER_JS: &str = include_str!(concat!(env!("OUT_DIR"), "/flow_parser.js"));

/// Flow parser error.
#[derive(Debug)]
pub enum Error {
    /// QuickJS runtime error.
    Runtime(String),
    /// Flow parser reported syntax errors.
    Parse(Vec<String>),
    /// Failed to deserialize the AST JSON.
    Deserialize(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(msg) => write!(f, "runtime error: {msg}"),
            Self::Parse(errors) => {
                write!(f, "parse errors: {}", errors.join("; "))
            }
            Self::Deserialize(msg) => write!(f, "AST deserialization error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

/// A reusable Flow parser instance backed by QuickJS.
///
/// Create once, parse many times. The QuickJS context is reused across calls.
pub struct FlowParser {
    ctx: Context,
}

impl FlowParser {
    /// Create a new parser instance.
    ///
    /// Loads the Flow parser JS into a QuickJS context.
    pub fn new() -> Result<Self, Error> {
        let ctx = Context::new().map_err(|e| Error::Runtime(e.to_string()))?;

        ctx.eval(
            "var exports = {}; \
             var console = { log: function(){}, error: function(){}, warn: function(){} };",
        )
        .map_err(|e| Error::Runtime(e.to_string()))?;

        ctx.eval(FLOW_PARSER_JS)
            .map_err(|e| Error::Runtime(e.to_string()))?;

        Ok(Self { ctx })
    }

    /// Parse Flow source and return the raw JSON AST.
    pub fn parse_json(&self, source: &str) -> Result<String, Error> {
        let source_json =
            serde_json::to_string(source).map_err(|e| Error::Runtime(e.to_string()))?;

        let script = format!(
            "(function() {{\
                var ast = exports.parse({source_json}, {{ types: true }});\
                return JSON.stringify(ast);\
            }})()"
        );

        self.ctx
            .eval_as(&script)
            .map_err(|e| Error::Runtime(e.to_string()))
    }

    /// Parse Flow source and return the typed AST.
    pub fn parse(&self, source: &str) -> Result<Program, Error> {
        let json = self.parse_json(source)?;

        let program: Program =
            serde_json::from_str(&json).map_err(|e| Error::Deserialize(e.to_string()))?;

        if !program.errors.is_empty() {
            let messages = program.errors.iter().map(|e| e.message.clone()).collect();
            return Err(Error::Parse(messages));
        }

        Ok(program)
    }

    /// Validate that source is syntactically valid Flow.
    ///
    /// Returns `Ok(())` if the source parses without errors, `Err` otherwise.
    pub fn validate(&self, source: &str) -> Result<(), Error> {
        self.parse(source).map(|_| ())
    }

    /// Validate a Flow type declaration by wrapping it in a `// @flow` file.
    ///
    /// Accepts a bare declaration like `type Foo = string;` and wraps it as:
    /// ```text
    /// // @flow
    /// export type Foo = string;
    /// ```
    ///
    /// Declarations starting with `declare export` are wrapped without adding
    /// `export` (they already include it).
    pub fn validate_declaration(&self, decl: &str) -> Result<Program, Error> {
        let source = if decl.starts_with("declare export") {
            format!("// @flow\n{decl}\n")
        } else {
            format!("// @flow\nexport {decl}\n")
        };
        self.parse(&source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_type_alias() {
        // Arrange
        let parser = FlowParser::new().expect("failed to create parser");

        // Act
        let program = parser
            .parse("export type Foo = string;")
            .expect("failed to parse");

        // Assert
        assert_eq!(program.body.len(), 1, "should have one statement");
        match &program.body[0] {
            Statement::ExportNamedDeclaration {
                declaration: Some(Declaration::TypeAlias { id, right }),
            } => {
                assert_eq!(id.name, "Foo", "type name");
                assert_eq!(right.type_name(), "StringTypeAnnotation", "right-hand side");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parse_exact_object() {
        // Arrange
        let parser = FlowParser::new().expect("failed to create parser");

        // Act
        let program = parser
            .parse("export type T = {| +name: string, +age: number |};")
            .expect("failed to parse");

        // Assert
        match &program.body[0] {
            Statement::ExportNamedDeclaration {
                declaration: Some(Declaration::TypeAlias { right, .. }),
            } => match right {
                TypeAnnotation::ObjectTypeAnnotation {
                    properties, exact, ..
                } => {
                    assert!(*exact, "should be exact object");
                    assert_eq!(properties.len(), 2, "should have 2 properties");
                }
                other => panic!("expected ObjectTypeAnnotation, got: {}", other.type_name()),
            },
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parse_error_reported() {
        // Arrange
        let parser = FlowParser::new().expect("failed to create parser");

        // Act
        let result = parser.parse("type = ;");

        // Assert
        assert!(result.is_err(), "should report parse error");
    }

    #[test]
    fn parser_reusable() {
        // Arrange
        let parser = FlowParser::new().expect("failed to create parser");

        // Act and Assert
        assert!(parser.parse("type A = string;").is_ok(), "first parse");
        assert!(parser.parse("type B = number;").is_ok(), "second parse");
    }

    #[test]
    fn validate_valid_source() {
        // Arrange
        let parser = FlowParser::new().expect("failed to create parser");

        // Act and Assert
        assert!(
            parser.validate("type T = {| +x: number |}").is_ok(),
            "valid Flow should pass"
        );
    }

    #[test]
    fn validate_invalid_source() {
        // Arrange
        let parser = FlowParser::new().expect("failed to create parser");

        // Act and Assert
        assert!(
            parser.validate("type = ;").is_err(),
            "invalid Flow should fail"
        );
    }

    #[test]
    fn validate_declaration_type_alias() {
        // Arrange
        let parser = FlowParser::new().expect("failed to create parser");

        // Act
        let program = parser
            .validate_declaration("type Foo = string;")
            .expect("should parse");

        // Assert
        match &program.body[0] {
            Statement::ExportNamedDeclaration {
                declaration: Some(Declaration::TypeAlias { id, .. }),
            } => {
                assert_eq!(id.name, "Foo", "type name preserved");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn validate_declaration_opaque() {
        // Arrange
        let parser = FlowParser::new().expect("failed to create parser");

        // Act
        let program = parser
            .validate_declaration("declare export opaque type Token;")
            .expect("should parse");

        // Assert
        match &program.body[0] {
            Statement::DeclareExportDeclaration { declaration: Some(decl) } => match decl {
                Declaration::OpaqueType { id, .. } | Declaration::DeclareOpaqueType { id, .. } => {
                    assert_eq!(id.name, "Token", "opaque type name");
                }
                other => panic!("expected OpaqueType, got: {other:?}"),
            },
            other => panic!("unexpected statement: {other:?}"),
        }
    }
}
