// ═══════════════════════════════════════════════════════════════════════════════
// FORMULA ENGINE — ERROR TYPES
// ═══════════════════════════════════════════════════════════════════════════════
// Position-aware, type-aware error reporting for parse, compile, and runtime.

use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize)]
pub enum ErrorKind {
    // Lexer errors
    UnexpectedCharacter(char),
    UnterminatedString,
    InvalidNumber(String),

    // Parser errors
    UnexpectedToken(String),
    ExpectedToken(String, String), // expected, got
    UnexpectedEof,
    InvalidAssignment,

    // Compiler errors
    TooManyConstants,
    TooManyLocals,

    // Runtime errors
    TypeError(String),
    DivisionByZero,
    PropertyNotFound(String),
    FunctionNotFound(String),
    ArityMismatch(String, String, usize), // name, expected_description, got
    IndexOutOfBounds(i64, usize),
    InvalidRegex(String),
    StackOverflow,
    CircularDependency(Vec<String>),

    // General
    Internal(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct FormulaError {
    pub kind: ErrorKind,
    pub message: String,
    pub span: Option<Span>,
}

impl FormulaError {
    pub fn new(kind: ErrorKind, span: Option<Span>) -> Self {
        let message = match &kind {
            ErrorKind::UnexpectedCharacter(c) => format!("Unexpected character '{}'", c),
            ErrorKind::UnterminatedString => "Unterminated string literal".into(),
            ErrorKind::InvalidNumber(s) => format!("Invalid number '{}'", s),
            ErrorKind::UnexpectedToken(t) => format!("Unexpected token '{}'", t),
            ErrorKind::ExpectedToken(exp, got) => {
                format!("Expected {}, got '{}'", exp, got)
            }
            ErrorKind::UnexpectedEof => "Unexpected end of formula".into(),
            ErrorKind::InvalidAssignment => "Invalid assignment target".into(),
            ErrorKind::TooManyConstants => "Too many constants in formula (max 65536)".into(),
            ErrorKind::TooManyLocals => "Too many local variables (max 256)".into(),
            ErrorKind::TypeError(msg) => msg.clone(),
            ErrorKind::DivisionByZero => "Division by zero".into(),
            ErrorKind::PropertyNotFound(name) => {
                format!("Property '{}' not found", name)
            }
            ErrorKind::FunctionNotFound(name) => {
                format!("Function '{}' not found", name)
            }
            ErrorKind::ArityMismatch(name, expected, got) => {
                format!(
                    "Function '{}' expects {} argument(s), got {}",
                    name, expected, got
                )
            }
            ErrorKind::IndexOutOfBounds(idx, len) => {
                format!("Index {} out of bounds (length {})", idx, len)
            }
            ErrorKind::InvalidRegex(pat) => format!("Invalid regex pattern: {}", pat),
            ErrorKind::StackOverflow => "Stack overflow — formula too deeply nested".into(),
            ErrorKind::CircularDependency(chain) => {
                format!("Circular dependency: {}", chain.join(" → "))
            }
            ErrorKind::Internal(msg) => format!("Internal error: {}", msg),
        };
        Self { kind, message, span }
    }

    pub fn runtime(kind: ErrorKind) -> Self {
        Self::new(kind, None)
    }
}

impl fmt::Display for FormulaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = &self.span {
            write!(f, "[{}..{}] {}", span.start, span.end, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for FormulaError {}

pub type Result<T> = std::result::Result<T, FormulaError>;
