// ═══════════════════════════════════════════════════════════════════════════════
// FORMULA ENGINE — WASM ENTRY POINT (lib.rs)
// ═══════════════════════════════════════════════════════════════════════════════
// Exports: compile, evaluate, batch_evaluate, validate, get_dependencies
// All data crosses the WASM boundary as JSON strings.

pub mod error;
pub mod types;
pub mod lexer;
pub mod parser;
pub mod compiler;
pub mod vm;
pub mod functions;

use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

use compiler::Chunk;
use error::FormulaError;
use types::Value;
use vm::{EvalContext, VM};

// ─── Compiled formula cache ───────────────────────────────────────────────────

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_HANDLE: AtomicU32 = AtomicU32::new(1);

thread_local! {
    static FORMULA_CACHE: RefCell<HashMap<u32, Chunk>> = RefCell::new(HashMap::new());
}

// ─── Serde types for JSON bridge ──────────────────────────────────────────────

#[derive(Serialize)]
struct CompileResult {
    ok: bool,
    handle: Option<u32>,
    error: Option<String>,
    error_pos: Option<(usize, usize)>,
}

#[derive(Serialize)]
struct EvalResult {
    ok: bool,
    value: Option<JsonValue>,
    error: Option<String>,
}

#[derive(Serialize)]
struct BatchResult {
    ok: bool,
    values: Option<Vec<JsonValue>>,
    error: Option<String>,
}

#[derive(Serialize)]
struct ValidateResult {
    ok: bool,
    errors: Vec<ValidationError>,
    dependencies: Vec<String>,
}

#[derive(Serialize)]
struct ValidationError {
    message: String,
    start: usize,
    end: usize,
}

/// JSON-friendly value representation
#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "value")]
enum JsonValue {
    #[serde(rename = "number")]
    Number(f64),
    #[serde(rename = "text")]
    Text(String),
    #[serde(rename = "boolean")]
    Boolean(bool),
    #[serde(rename = "date")]
    Date(f64),
    #[serde(rename = "dateRange")]
    DateRange { start: f64, end: f64 },
    #[serde(rename = "array")]
    Array(Vec<JsonValue>),
    #[serde(rename = "empty")]
    Empty,
}

impl From<Value> for JsonValue {
    fn from(v: Value) -> Self {
        match v {
            Value::Number(n) => JsonValue::Number(n),
            Value::Text(s) => JsonValue::Text(s),
            Value::Boolean(b) => JsonValue::Boolean(b),
            Value::Date(ms) => JsonValue::Date(ms as f64),
            Value::DateRange(s, e) => JsonValue::DateRange { start: s as f64, end: e as f64 },
            Value::Array(arr) => JsonValue::Array(arr.into_iter().map(|v| v.into()).collect()),
            Value::RichText(spans) => {
                JsonValue::Text(spans.iter().map(|s| s.text.as_str()).collect())
            }
            Value::Empty => JsonValue::Empty,
        }
    }
}

impl From<JsonValue> for Value {
    fn from(jv: JsonValue) -> Self {
        match jv {
            JsonValue::Number(n) => Value::Number(n),
            JsonValue::Text(s) => Value::Text(s),
            JsonValue::Boolean(b) => Value::Boolean(b),
            JsonValue::Date(ms) => Value::Date(ms as i64),
            JsonValue::DateRange { start, end } => Value::DateRange(start as i64, end as i64),
            JsonValue::Array(arr) => Value::Array(arr.into_iter().map(|v| v.into()).collect()),
            JsonValue::Empty => Value::Empty,
        }
    }
}

// ─── Internal compile pipeline ────────────────────────────────────────────────

fn compile_formula(source: &str) -> Result<Chunk, FormulaError> {
    // Lex
    let lex = lexer::Lexer::new(source);
    let tokens = lex.tokenize()?;

    // Parse
    let parser = parser::Parser::new(tokens);
    let ast = parser.parse()?;

    // Compile to bytecode
    let comp = compiler::Compiler::new();
    let chunk = comp.compile(&ast)?;

    Ok(chunk)
}

fn eval_chunk(chunk: &Chunk, props: HashMap<String, Value>) -> Result<Value, FormulaError> {
    types::clear_closures();
    let ctx = EvalContext::with_properties(props);
    let mut vm = VM::new();
    vm.execute(chunk, &ctx)
}

fn parse_properties(props_json: &str) -> HashMap<String, Value> {
    let parsed: Result<HashMap<String, JsonValue>, _> = serde_json::from_str(props_json);
    match parsed {
        Ok(map) => map.into_iter().map(|(k, v)| (k, v.into())).collect(),
        Err(_) => HashMap::new(),
    }
}

// ─── WASM Exports ─────────────────────────────────────────────────────────────

/// Compile a formula string → returns JSON { ok, handle?, error? }
#[wasm_bindgen]
pub fn compile(formula: &str) -> String {
    let result = match compile_formula(formula) {
        Ok(chunk) => {
            let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
            FORMULA_CACHE.with(|cache| {
                cache.borrow_mut().insert(handle, chunk);
            });
            CompileResult {
                ok: true,
                handle: Some(handle),
                error: None,
                error_pos: None,
            }
        }
        Err(e) => {
            let pos = e.span.as_ref().map(|s| (s.start, s.end));
            CompileResult {
                ok: false,
                handle: None,
                error: Some(e.to_string()),
                error_pos: pos,
            }
        }
    };
    serde_json::to_string(&result).unwrap_or_else(|_| r#"{"ok":false,"error":"serialization error"}"#.into())
}

/// Evaluate a compiled formula with given properties JSON → returns JSON { ok, value?, error? }
#[wasm_bindgen]
pub fn evaluate(handle: u32, props_json: &str) -> String {
    let result = FORMULA_CACHE.with(|cache| {
        let cache = cache.borrow();
        match cache.get(&handle) {
            Some(chunk) => {
                let props = parse_properties(props_json);
                match eval_chunk(chunk, props) {
                    Ok(val) => EvalResult {
                        ok: true,
                        value: Some(val.into()),
                        error: None,
                    },
                    Err(e) => EvalResult {
                        ok: false,
                        value: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            None => EvalResult {
                ok: false,
                value: None,
                error: Some(format!("Formula handle {} not found", handle)),
            },
        }
    });
    serde_json::to_string(&result).unwrap_or_else(|_| r#"{"ok":false,"error":"serialization error"}"#.into())
}

/// Batch evaluate a formula over multiple row contexts → returns JSON { ok, values?, error? }
#[wasm_bindgen]
pub fn batch_evaluate(handle: u32, rows_json: &str) -> String {
    let result = FORMULA_CACHE.with(|cache| {
        let cache = cache.borrow();
        match cache.get(&handle) {
            Some(chunk) => {
                let rows: Vec<HashMap<String, JsonValue>> =
                    serde_json::from_str(rows_json).unwrap_or_default();
                let mut values = Vec::with_capacity(rows.len());

                for row in rows {
                    let props: HashMap<String, Value> =
                        row.into_iter().map(|(k, v)| (k, v.into())).collect();
                    match eval_chunk(chunk, props) {
                        Ok(val) => values.push(val.into()),
                        Err(_) => values.push(JsonValue::Empty),
                    }
                }

                BatchResult {
                    ok: true,
                    values: Some(values),
                    error: None,
                }
            }
            None => BatchResult {
                ok: false,
                values: None,
                error: Some(format!("Formula handle {} not found", handle)),
            },
        }
    });
    serde_json::to_string(&result).unwrap_or_else(|_| r#"{"ok":false,"error":"serialization error"}"#.into())
}

/// Validate a formula without executing → returns JSON { ok, errors[], dependencies[] }
#[wasm_bindgen]
pub fn validate(formula: &str) -> String {
    let result = match compile_formula(formula) {
        Ok(chunk) => ValidateResult {
            ok: true,
            errors: Vec::new(),
            dependencies: chunk.dependencies.clone(),
        },
        Err(e) => {
            let span = e.span.clone().unwrap_or(error::Span { start: 0, end: 0 });
            ValidateResult {
                ok: false,
                errors: vec![ValidationError {
                    message: e.to_string(),
                    start: span.start,
                    end: span.end,
                }],
                dependencies: Vec::new(),
            }
        }
    };
    serde_json::to_string(&result).unwrap_or_else(|_| r#"{"ok":false,"errors":[],"dependencies":[]}"#.into())
}

/// Get dependencies of a compiled formula → JSON string array
#[wasm_bindgen]
pub fn get_dependencies(handle: u32) -> String {
    let deps = FORMULA_CACHE.with(|cache| {
        cache
            .borrow()
            .get(&handle)
            .map(|c| c.dependencies.clone())
            .unwrap_or_default()
    });
    serde_json::to_string(&deps).unwrap_or_else(|_| "[]".into())
}

/// Free a compiled formula from cache
#[wasm_bindgen]
pub fn free_formula(handle: u32) {
    FORMULA_CACHE.with(|cache| {
        cache.borrow_mut().remove(&handle);
    });
}

/// One-shot compile + evaluate (convenience for single evaluations)
#[wasm_bindgen]
pub fn eval_formula(formula: &str, props_json: &str) -> String {
    let result = match compile_formula(formula) {
        Ok(chunk) => {
            let props = parse_properties(props_json);
            match eval_chunk(&chunk, props) {
                Ok(val) => EvalResult {
                    ok: true,
                    value: Some(val.into()),
                    error: None,
                },
                Err(e) => EvalResult {
                    ok: false,
                    value: None,
                    error: Some(e.to_string()),
                },
            }
        }
        Err(e) => EvalResult {
            ok: false,
            value: None,
            error: Some(e.to_string()),
        },
    };
    serde_json::to_string(&result).unwrap_or_else(|_| r#"{"ok":false,"error":"serialization error"}"#.into())
}
