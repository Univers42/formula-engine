// ═══════════════════════════════════════════════════════════════════════════════
// ARRAY FUNCTIONS (17+ functions)
// ═══════════════════════════════════════════════════════════════════════════════
// Higher-order functions (map, filter, etc.) execute closures via the VM.

use crate::compiler::Chunk;
use crate::error::{ErrorKind, FormulaError};
use crate::types::{ClosureData, Value};
use crate::vm::{EvalContext, VM};

fn expect_array(args: &[Value], idx: usize, fname: &str) -> crate::error::Result<Vec<Value>> {
    match args.get(idx) {
        Some(Value::Array(arr)) => Ok(arr.clone()),
        Some(Value::Empty) => Ok(Vec::new()),
        _ => Err(FormulaError::new(
            ErrorKind::TypeError(format!("{}: expected array at argument {}", fname, idx + 1)),
            None,
        )),
    }
}

fn expect_closure(args: &[Value], idx: usize, fname: &str) -> crate::error::Result<ClosureData> {
    args.get(idx)
        .and_then(|v| Value::as_closure(v))
        .ok_or_else(|| {
            FormulaError::new(
                ErrorKind::TypeError(format!("{}: expected function at argument {}", fname, idx + 1)),
                None,
            )
        })
}

fn call_closure_with(
    closure: &ClosureData,
    args: &[Value],
    chunk: &Chunk,
    ctx: &EvalContext,
    vm: &mut VM,
    call_depth: usize,
) -> crate::error::Result<Value> {
    let lambda = &chunk.lambda_bodies[closure.body_idx];
    vm.call_closure(closure, lambda, args, chunk, ctx, call_depth + 1)
}

/// map(array, fn) → array
pub fn map(
    args: Vec<Value>,
    chunk: &Chunk,
    ctx: &EvalContext,
    vm: &mut VM,
    call_depth: usize,
) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("map".into(), "2".into(), args.len()),
            None,
        ));
    }
    let arr = expect_array(&args, 0, "map")?;
    let closure = expect_closure(&args, 1, "map")?;
    let mut result = Vec::with_capacity(arr.len());
    for (i, item) in arr.into_iter().enumerate() {
        let val = call_closure_with(
            &closure,
            &[item, Value::Number(i as f64)],
            chunk, ctx, vm, call_depth,
        )?;
        result.push(val);
    }
    Ok(Value::Array(result))
}

/// filter(array, fn) → array
pub fn filter(
    args: Vec<Value>,
    chunk: &Chunk,
    ctx: &EvalContext,
    vm: &mut VM,
    call_depth: usize,
) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("filter".into(), "2".into(), args.len()),
            None,
        ));
    }
    let arr = expect_array(&args, 0, "filter")?;
    let closure = expect_closure(&args, 1, "filter")?;
    let mut result = Vec::new();
    for (i, item) in arr.into_iter().enumerate() {
        let val = call_closure_with(
            &closure,
            &[item.clone(), Value::Number(i as f64)],
            chunk, ctx, vm, call_depth,
        )?;
        if val.to_bool() {
            result.push(item);
        }
    }
    Ok(Value::Array(result))
}

/// find(array, fn) → value | empty
pub fn find(
    args: Vec<Value>,
    chunk: &Chunk,
    ctx: &EvalContext,
    vm: &mut VM,
    call_depth: usize,
) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("find".into(), "2".into(), args.len()),
            None,
        ));
    }
    let arr = expect_array(&args, 0, "find")?;
    let closure = expect_closure(&args, 1, "find")?;
    for (i, item) in arr.into_iter().enumerate() {
        let val = call_closure_with(
            &closure,
            &[item.clone(), Value::Number(i as f64)],
            chunk, ctx, vm, call_depth,
        )?;
        if val.to_bool() {
            return Ok(item);
        }
    }
    Ok(Value::Empty)
}

/// findIndex(array, fn) → number | -1
pub fn find_index(
    args: Vec<Value>,
    chunk: &Chunk,
    ctx: &EvalContext,
    vm: &mut VM,
    call_depth: usize,
) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("findIndex".into(), "2".into(), args.len()),
            None,
        ));
    }
    let arr = expect_array(&args, 0, "findIndex")?;
    let closure = expect_closure(&args, 1, "findIndex")?;
    for (i, item) in arr.into_iter().enumerate() {
        let val = call_closure_with(
            &closure,
            &[item, Value::Number(i as f64)],
            chunk, ctx, vm, call_depth,
        )?;
        if val.to_bool() {
            return Ok(Value::Number(i as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

/// some(array, fn) → boolean
pub fn some(
    args: Vec<Value>,
    chunk: &Chunk,
    ctx: &EvalContext,
    vm: &mut VM,
    call_depth: usize,
) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("some".into(), "2".into(), args.len()),
            None,
        ));
    }
    let arr = expect_array(&args, 0, "some")?;
    let closure = expect_closure(&args, 1, "some")?;
    for item in arr {
        let val = call_closure_with(&closure, &[item], chunk, ctx, vm, call_depth)?;
        if val.to_bool() {
            return Ok(Value::Boolean(true));
        }
    }
    Ok(Value::Boolean(false))
}

/// every(array, fn) → boolean
pub fn every(
    args: Vec<Value>,
    chunk: &Chunk,
    ctx: &EvalContext,
    vm: &mut VM,
    call_depth: usize,
) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("every".into(), "2".into(), args.len()),
            None,
        ));
    }
    let arr = expect_array(&args, 0, "every")?;
    let closure = expect_closure(&args, 1, "every")?;
    for item in arr {
        let val = call_closure_with(&closure, &[item], chunk, ctx, vm, call_depth)?;
        if !val.to_bool() {
            return Ok(Value::Boolean(false));
        }
    }
    Ok(Value::Boolean(true))
}

/// reduce(array, fn, initial?) → value
pub fn reduce(
    args: Vec<Value>,
    chunk: &Chunk,
    ctx: &EvalContext,
    vm: &mut VM,
    call_depth: usize,
) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("reduce".into(), "2-3".into(), args.len()),
            None,
        ));
    }
    let arr = expect_array(&args, 0, "reduce")?;
    let closure = expect_closure(&args, 1, "reduce")?;
    let mut acc = if args.len() >= 3 {
        args[2].clone()
    } else if !arr.is_empty() {
        arr[0].clone()
    } else {
        return Ok(Value::Empty);
    };

    let start = if args.len() >= 3 { 0 } else { 1 };
    for item in arr.into_iter().skip(start) {
        acc = call_closure_with(&closure, &[acc, item], chunk, ctx, vm, call_depth)?;
    }
    Ok(acc)
}

/// flat(array, depth?) → array
pub fn flat(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    let arr = expect_array(&args, 0, "flat")?;
    let depth = args.get(1).and_then(|v| v.to_number()).unwrap_or(1.0) as usize;
    Ok(Value::Array(flatten(arr, depth)))
}

fn flatten(arr: Vec<Value>, depth: usize) -> Vec<Value> {
    if depth == 0 {
        return arr;
    }
    let mut result = Vec::new();
    for item in arr {
        match item {
            Value::Array(inner) => result.extend(flatten(inner, depth - 1)),
            _ => result.push(item),
        }
    }
    result
}

/// sort(array) → sorted array (numbers ascending, strings alphabetical)
pub fn sort(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    let mut arr = expect_array(&args, 0, "sort")?;
    arr.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Ok(Value::Array(arr))
}

/// reverse(array) → reversed array
pub fn reverse(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    let mut arr = expect_array(&args, 0, "reverse")?;
    arr.reverse();
    Ok(Value::Array(arr))
}

/// unique(array) → array with duplicates removed
pub fn unique(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    let arr = expect_array(&args, 0, "unique")?;
    let mut seen = Vec::new();
    let mut result = Vec::new();
    for item in arr {
        let key = item.to_text();
        if !seen.contains(&key) {
            seen.push(key);
            result.push(item);
        }
    }
    Ok(Value::Array(result))
}

/// at(array, index) → value
pub fn at(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("at".into(), "2".into(), args.len()),
            None,
        ));
    }
    let arr = expect_array(&args, 0, "at")?;
    let idx = args[1].to_number().unwrap_or(0.0) as i64;
    let actual_idx = if idx < 0 {
        (arr.len() as i64 + idx) as usize
    } else {
        idx as usize
    };
    Ok(arr.get(actual_idx).cloned().unwrap_or(Value::Empty))
}

/// first(array) → value
pub fn first(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Empty);
    }
    let arr = expect_array(&args, 0, "first")?;
    Ok(arr.into_iter().next().unwrap_or(Value::Empty))
}

/// last(array) → value
pub fn last(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Empty);
    }
    let arr = expect_array(&args, 0, "last")?;
    Ok(arr.into_iter().last().unwrap_or(Value::Empty))
}

/// sum(array) → number
pub fn sum(args: Vec<Value>) -> crate::error::Result<Value> {
    let mut total = 0.0;
    for arg in &args {
        match arg {
            Value::Array(arr) => {
                for v in arr {
                    if let Some(n) = v.to_number() {
                        total += n;
                    }
                }
            }
            _ => {
                if let Some(n) = arg.to_number() {
                    total += n;
                }
            }
        }
    }
    Ok(Value::Number(total))
}

/// count(array) → number (counts non-empty elements)
pub fn count(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Number(0.0));
    }
    match &args[0] {
        Value::Array(arr) => {
            let n = arr.iter().filter(|v| !v.is_empty()).count();
            Ok(Value::Number(n as f64))
        }
        Value::Empty => Ok(Value::Number(0.0)),
        _ => Ok(Value::Number(1.0)),
    }
}

/// average(array) → number
pub fn average(args: Vec<Value>) -> crate::error::Result<Value> {
    let mut total = 0.0;
    let mut count = 0;
    for arg in &args {
        match arg {
            Value::Array(arr) => {
                for v in arr {
                    if let Some(n) = v.to_number() {
                        total += n;
                        count += 1;
                    }
                }
            }
            _ => {
                if let Some(n) = arg.to_number() {
                    total += n;
                    count += 1;
                }
            }
        }
    }
    if count == 0 {
        Ok(Value::Empty)
    } else {
        Ok(Value::Number(total / count as f64))
    }
}

/// median(array) → number
pub fn median(args: Vec<Value>) -> crate::error::Result<Value> {
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
    nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = nums.len() / 2;
    if nums.len() % 2 == 0 {
        Ok(Value::Number((nums[mid - 1] + nums[mid]) / 2.0))
    } else {
        Ok(Value::Number(nums[mid]))
    }
}

/// Internal: construct array from N items on the stack
pub fn construct(args: Vec<Value>) -> crate::error::Result<Value> {
    Ok(Value::Array(args))
}

/// includes(array, value) → boolean
pub fn includes(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("includes".into(), "2".into(), args.len()),
            None,
        ));
    }
    let arr = expect_array(&args, 0, "includes")?;
    let needle = &args[1];
    Ok(Value::Boolean(arr.iter().any(|v| v == needle)))
}
