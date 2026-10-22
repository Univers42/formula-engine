/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   error.rs                                           :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: dlesieur <dlesieur@student.42.fr>          +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2026/05/18 21:19:17 by dlesieur          #+#    #+#             */
/*   Updated: 2026/05/18 21:19:17 by dlesieur         ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

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
    BudgetExceeded { limit: u64 },

    // General
    Internal(String),
}

impl ErrorKind {
    /// Stable machine-readable code for this error kind.
    /// Used by the TS bridge to distinguish error types (e.g. budget_exceeded
    /// vs stack_overflow vs engine_unavailable).
    pub fn code(&self) -> &'static str {
        match self {
            ErrorKind::UnexpectedCharacter(_) => "unexpected_character",
            ErrorKind::UnterminatedString => "unterminated_string",
            ErrorKind::InvalidNumber(_) => "invalid_number",
            ErrorKind::UnexpectedToken(_) => "unexpected_token",
            ErrorKind::ExpectedToken(_, _) => "expected_token",
            ErrorKind::UnexpectedEof => "unexpected_eof",
            ErrorKind::InvalidAssignment => "invalid_assignment",
            ErrorKind::TooManyConstants => "too_many_constants",
            ErrorKind::TooManyLocals => "too_many_locals",
            ErrorKind::TypeError(_) => "type_error",
            ErrorKind::DivisionByZero => "division_by_zero",
            ErrorKind::PropertyNotFound(_) => "property_not_found",
            ErrorKind::FunctionNotFound(_) => "function_not_found",
            ErrorKind::ArityMismatch(_, _, _) => "arity_mismatch",
            ErrorKind::IndexOutOfBounds(_, _) => "index_out_of_bounds",
            ErrorKind::InvalidRegex(_) => "invalid_regex",
            ErrorKind::StackOverflow => "stack_overflow",
            ErrorKind::CircularDependency(_) => "circular_dependency",
            ErrorKind::BudgetExceeded { .. } => "budget_exceeded",
            ErrorKind::Internal(_) => "internal_error",
        }
    }
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
            ErrorKind::BudgetExceeded { limit } => {
                format!("Formula budget exceeded: more than {limit} instructions").into()
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
