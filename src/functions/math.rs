// ═══════════════════════════════════════════════════════════════════════════════
// MATH FUNCTIONS (26 functions)
// ═══════════════════════════════════════════════════════════════════════════════

use crate::error::{ErrorKind, FormulaError};
use crate::types::Value;

fn expect_num(args: &[Value], idx: usize, fname: &str) -> crate::error::Result<f64> {
    args.get(idx)
        .and_then(|v| v.to_number())
        .ok_or_else(|| {
            FormulaError::new(
                ErrorKind::TypeError(format!("{}: expected number at argument {}", fname, idx + 1)),
                None,
            )
        })
}

fn null_guard_1<F: Fn(f64) -> f64>(args: Vec<Value>, f: F) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Empty);
    }
    match &args[0] {
        Value::Empty => Ok(Value::Empty),
        _ => match args[0].to_number() {
            Some(n) => Ok(Value::Number(f(n))),
            None => Ok(Value::Empty),
        },
    }
}

pub fn abs(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.abs())
}

pub fn ceil(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.ceil())
}

pub fn floor(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.floor())
}

pub fn round(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Empty);
    }
    if args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let n = args[0].to_number().unwrap_or(0.0);
    let precision = args.get(1).and_then(|v| v.to_number()).unwrap_or(0.0) as i32;
    if precision == 0 {
        Ok(Value::Number(n.round()))
    } else {
        let factor = 10f64.powi(precision);
        Ok(Value::Number((n * factor).round() / factor))
    }
}

pub fn max(args: Vec<Value>) -> crate::error::Result<Value> {
    // Flatten arrays first
    let mut nums = Vec::new();
    for arg in &args {
        match arg {
            Value::Array(arr) => {
                for v in arr {
                    if let Some(n) = v.to_number() {
                        nums.push(n);
                    }
                }
            }
            Value::Empty => {}
            _ => {
                if let Some(n) = arg.to_number() {
                    nums.push(n);
                }
            }
        }
    }
    if nums.is_empty() {
        return Ok(Value::Empty);
    }
    Ok(Value::Number(
        nums.into_iter().fold(f64::NEG_INFINITY, f64::max),
    ))
}

pub fn min(args: Vec<Value>) -> crate::error::Result<Value> {
    let mut nums = Vec::new();
    for arg in &args {
        match arg {
            Value::Array(arr) => {
                for v in arr {
                    if let Some(n) = v.to_number() {
                        nums.push(n);
                    }
                }
            }
            Value::Empty => {}
            _ => {
                if let Some(n) = arg.to_number() {
                    nums.push(n);
                }
            }
        }
    }
    if nums.is_empty() {
        return Ok(Value::Empty);
    }
    Ok(Value::Number(
        nums.into_iter().fold(f64::INFINITY, f64::min),
    ))
}

pub fn sqrt(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.sqrt())
}

pub fn cbrt(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.cbrt())
}

pub fn pow(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("pow".into(), "2".into(), args.len()),
            None,
        ));
    }
    if args[0].is_empty() || args[1].is_empty() {
        return Ok(Value::Empty);
    }
    let base = expect_num(&args, 0, "pow")?;
    let exp = expect_num(&args, 1, "pow")?;
    Ok(Value::Number(base.powf(exp)))
}

pub fn exp(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.exp())
}

pub fn ln(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.ln())
}

pub fn log10(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.log10())
}

pub fn log2(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.log2())
}

pub fn sign(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| {
        if n > 0.0 {
            1.0
        } else if n < 0.0 {
            -1.0
        } else {
            0.0
        }
    })
}

pub fn pi(_args: Vec<Value>) -> crate::error::Result<Value> {
    Ok(Value::Number(std::f64::consts::PI))
}

pub fn euler(_args: Vec<Value>) -> crate::error::Result<Value> {
    Ok(Value::Number(std::f64::consts::E))
}

pub fn modulo(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("mod".into(), "2".into(), args.len()),
            None,
        ));
    }
    if args[0].is_empty() || args[1].is_empty() {
        return Ok(Value::Empty);
    }
    let a = expect_num(&args, 0, "mod")?;
    let b = expect_num(&args, 1, "mod")?;
    Ok(Value::Number(a % b))
}

pub fn unary_minus(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| -n)
}

pub fn unary_plus(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n)
}

pub fn to_number(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Empty);
    }
    match &args[0] {
        Value::Empty => Ok(Value::Number(0.0)),
        Value::Number(n) => Ok(Value::Number(*n)),
        Value::Boolean(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
        Value::Text(s) => match s.trim().parse::<f64>() {
            Ok(n) => Ok(Value::Number(n)),
            Err(_) => Ok(Value::Empty),
        },
        Value::Date(ms) => Ok(Value::Number(*ms as f64)),
        _ => Ok(Value::Empty),
    }
}

pub fn sin(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.sin())
}

pub fn cos(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.cos())
}

pub fn tan(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.tan())
}

pub fn asin(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.asin())
}

pub fn acos(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.acos())
}

pub fn atan(args: Vec<Value>) -> crate::error::Result<Value> {
    null_guard_1(args, |n| n.atan())
}
