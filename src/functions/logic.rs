// ═══════════════════════════════════════════════════════════════════════════════
// LOGIC / GENERAL FUNCTIONS (12+ functions)
// ═══════════════════════════════════════════════════════════════════════════════

use crate::types::Value;

/// empty(value) → boolean — checks if value is empty/null
pub fn empty(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Boolean(true));
    }
    Ok(Value::Boolean(args[0].is_empty()))
}

/// not(value) → boolean
pub fn not(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Boolean(true));
    }
    Ok(Value::Boolean(!args[0].to_bool()))
}

/// and(a, b, ...) → boolean
pub fn and(args: Vec<Value>) -> crate::error::Result<Value> {
    Ok(Value::Boolean(args.iter().all(|v| v.to_bool())))
}

/// or(a, b, ...) → boolean
pub fn or(args: Vec<Value>) -> crate::error::Result<Value> {
    Ok(Value::Boolean(args.iter().any(|v| v.to_bool())))
}

/// equal(a, b) → boolean
pub fn equal(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Ok(Value::Boolean(false));
    }
    Ok(Value::Boolean(args[0] == args[1]))
}

/// unequal(a, b) → boolean
pub fn unequal(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Ok(Value::Boolean(true));
    }
    Ok(Value::Boolean(args[0] != args[1]))
}

/// larger(a, b) → boolean
pub fn larger(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Ok(Value::Boolean(false));
    }
    Ok(Value::Boolean(args[0] > args[1]))
}

/// smaller(a, b) → boolean
pub fn smaller(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Ok(Value::Boolean(false));
    }
    Ok(Value::Boolean(args[0] < args[1]))
}

/// largerEq(a, b) → boolean
pub fn larger_eq(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Ok(Value::Boolean(false));
    }
    Ok(Value::Boolean(args[0] >= args[1]))
}

/// smallerEq(a, b) → boolean
pub fn smaller_eq(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Ok(Value::Boolean(false));
    }
    Ok(Value::Boolean(args[0] <= args[1]))
}

/// toBoolean(value) → boolean
pub fn to_boolean(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Boolean(false));
    }
    Ok(Value::Boolean(args[0].to_bool()))
}

/// toString(value) → text
pub fn to_string(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Text(String::new()));
    }
    Ok(Value::Text(args[0].to_text()))
}

/// typeof(value) → text ("number", "text", "boolean", "date", "array", "empty")
pub fn type_of(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Text("empty".into()));
    }
    Ok(Value::Text(args[0].type_name().to_string()))
}
