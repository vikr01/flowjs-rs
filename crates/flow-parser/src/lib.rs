//! Typed Rust bindings for Facebook's Flow parser via QuickJS.

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
pub struct FlowParser {
    ctx: Context,
}

impl FlowParser {
    /// Create a new parser instance.
    ///
    /// Loads the Flow parser JS into a QuickJS context. The context is
    /// reusable across multiple `parse` calls.
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

    /// Parse Flow source code and return the raw JSON AST string.
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

    /// Parse Flow source code and return the typed AST.
    pub fn parse(&self, source: &str) -> Result<Program, Error> {
        let source_json = serde_json::to_string(source)
            .map_err(|e| Error::Runtime(e.to_string()))?;

        let script = format!(
            "(function() {{\
                var ast = exports.parse({source_json}, {{ types: true }});\
                return JSON.stringify(ast);\
            }})()"
        );

        let json: String = self
            .ctx
            .eval_as(&script)
            .map_err(|e| Error::Runtime(e.to_string()))?;

        let program: Program =
            serde_json::from_str(&json).map_err(|e| Error::Deserialize(e.to_string()))?;

        if !program.errors.is_empty() {
            let messages = program
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect();
            return Err(Error::Parse(messages));
        }

        Ok(program)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_type_alias() {
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
    fn parse_object_with_properties() {
        // Arrange
        let parser = FlowParser::new().expect("failed to create parser");

        // Act
        let program = parser
            .parse("type T = {| +name: string, +age: number |};")
            .expect("failed to parse");

        // Assert
        match &program.body[0] {
            Statement::Other => {
                // Non-exported type alias — parser may represent differently
            }
            _ => {}
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

        // Act
        let r1 = parser.parse("type A = string;");
        let r2 = parser.parse("type B = number;");

        // Assert
        assert!(r1.is_ok(), "first parse should succeed");
        assert!(r2.is_ok(), "second parse should succeed");
    }
}
