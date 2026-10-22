//! Cargo tests for the formula VM fuel budget (P4-2-a).
//! These tests exercise the instruction budget, BudgetExceeded error,
//! and the per-evaluation fuel semantics.

use crate::error::ErrorKind;
use crate::types::Value;
use crate::vm::{VM, EvalContext, DEFAULT_MAX_FUEL};
use std::collections::HashMap;

/// The engine's native grammar uses `+` for concatenation and lowercase builtin
/// names; `desugarFormula` (src/lib/formula/desugarFormula.ts) rewrites the
/// Excel sugar `UPPER("ok") & IF(2 >= 1, "!", "?")` into exactly this form
/// before the wasm ever sees it, so this is the bytecode the browser runs.
const SUGAR_FORMULA: &str = r#"upper("ok") + if(2 >= 1, "!", "?")"#;

/// A three-level nested `map` over 10 elements: 10³ = 1 000 closure calls.
const NESTED3_FORMULA: &str = "map([1,2,3,4,5,6,7,8,9,10], x => map([1,2,3,4,5,6,7,8,9,10], y => map([1,2,3,4,5,6,7,8,9,10], z => z)))";

/// Instructions consumed by one evaluation, measured on a fresh `VM::new()`.
fn instruction_count(formula: &str) -> u64 {
    let chunk = crate::compile_formula(formula).unwrap();
    let mut vm = VM::new();
    vm.execute(&chunk, &EvalContext::new()).unwrap();
    DEFAULT_MAX_FUEL - vm.fuel
}

#[test]
fn one_plus_one_under_default_budget() {
    // 1 + 1 should evaluate to 2 under the default budget (1_000_000 instructions)
    let chunk = crate::compile_formula("1 + 1").unwrap();
    let result = crate::eval_chunk(&chunk, HashMap::new());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Value::Number(2.0));
}

#[test]
fn budget_exceeded_with_small_budget() {
    // A formula that exceeds the budget must fail with BudgetExceeded.
    let chunk = crate::compile_formula("map([1,2,3,4,5,6,7,8,9,10], x => x)").unwrap();
    let err = VM::with_budget(5)
        .execute(&chunk, &EvalContext::new())
        .expect_err("5 instructions cannot map 10 elements");
    assert_eq!(err.kind.code(), "budget_exceeded");
}

#[test]
fn explicit_budget_large_enough_to_succeed() {
    // The same formula under a large explicit budget must succeed.
    let chunk = crate::compile_formula("map([1,2,3,4,5,6,7,8,9,10], x => x)").unwrap();
    let result = VM::with_budget(1_000_000).execute(&chunk, &EvalContext::new());
    assert!(result.is_ok());
}

#[test]
fn boundary_is_exact() {
    // The boundary is exact: a formula whose instruction count is measured
    // in the test succeeds at exactly that budget and fails at one less.
    let chunk = crate::compile_formula("1 + 1").unwrap();
    let instruction_count = chunk.code.len() as u64;
    // Succeed at exactly that budget
    let result = VM::with_budget(instruction_count).execute(&chunk, &EvalContext::new());
    assert!(result.is_ok(), "at exact budget, formula should succeed");
    // Fail at one less than budget
    let err = VM::with_budget(instruction_count - 1)
        .execute(&chunk, &EvalContext::new())
        .expect_err("one instruction short must trip the budget");
    assert_eq!(err.kind.code(), "budget_exceeded");
}

#[test]
fn budget_is_per_evaluation() {
    // Two evaluations through `eval_chunk` each get the whole budget, because
    // `eval_chunk` builds a fresh `VM` per evaluation.
    // The shared-budget case is pinned by `one_vm_shares_its_budget_across_executions`.
    let chunk = crate::compile_formula("1 + 1").unwrap();
    let result1 = crate::eval_chunk(&chunk, HashMap::new());
    let result2 = crate::eval_chunk(&chunk, HashMap::new());
    assert!(result1.is_ok(), "first eval_chunk must succeed");
    assert!(result2.is_ok(), "second eval_chunk must succeed");
}

#[test]
fn one_vm_shares_its_budget_across_executions() {
    // `execute()` does NOT refill the fuel: two calls on one VM share it, and
    // the second call continues with the remainder left by the first.
    let chunk = crate::compile_formula("1 + 1").unwrap();
    let ctx = EvalContext::new();

    let mut probe = VM::new();
    probe.execute(&chunk, &ctx).unwrap();
    let n = DEFAULT_MAX_FUEL - probe.fuel;
    assert!(n > 0, "1 + 1 must consume at least one instruction");

    // 2n - 1 instructions: the first execute (n) fits, the second does not.
    let mut vm = VM::with_budget(2 * n - 1);
    let first = vm.execute(&chunk, &ctx);
    assert!(first.is_ok(), "first execute must fit in the budget");

    let err = vm
        .execute(&chunk, &ctx)
        .expect_err("second execute must trip the shared budget");
    assert_eq!(err.kind.code(), "budget_exceeded");
}

#[test]
fn nested_map_exceeds_small_budget() {
    // A nested-map formula over N elements must exceed a small budget.
    let chunk = crate::compile_formula(NESTED3_FORMULA).unwrap();
    let err = VM::with_budget(100)
        .execute(&chunk, &EvalContext::new())
        .expect_err("100 instructions cannot run 1000 closure calls");
    assert_eq!(err.kind.code(), "budget_exceeded");
}

#[test]
fn nested_map_succeeds_large_budget() {
    // Same nested-map formula must succeed under a large budget.
    let chunk = crate::compile_formula(NESTED3_FORMULA).unwrap();
    let result = VM::with_budget(1_000_000).execute(&chunk, &EvalContext::new());
    assert!(result.is_ok(), "nested map over 10 elements x 3 levels must succeed with 1M budget");
}

#[test]
fn stack_overflow_distinct_from_budget() {
    // StackOverflow still fires past depth 256 and its code is "stack_overflow",
    // never "budget_exceeded" — the two ceilings stay distinguishable.
    assert_ne!(ErrorKind::StackOverflow.code(), "budget_exceeded");
    assert_eq!(ErrorKind::StackOverflow.code(), "stack_overflow");
    assert_eq!(ErrorKind::BudgetExceeded { limit: 100 }.code(), "budget_exceeded");
}

#[test]
fn batch_answers_ok_with_an_empty_row_when_one_row_fails() {
    // `batch_evaluate` runs every row through its own `eval_chunk` (so every row
    // gets the whole budget) and pushes `Empty` for a row that failed. The batch
    // still answers ok:true — a budget stop is invisible in a batch.
    let compiled = crate::compile(r#"10 / prop("d")"#);
    let compiled_json: serde_json::Value = serde_json::from_str(&compiled).unwrap();
    assert_eq!(compiled_json["ok"], serde_json::json!(true), "compile failed: {}", compiled);
    let handle = compiled_json["handle"].as_u64().unwrap() as u32;

    // Rows use the engine's tagged envelope: {"type":"number","value":2}.
    let rows = r#"[{"d":{"type":"number","value":2}},{"d":{"type":"number","value":0}},{"d":{"type":"number","value":5}}]"#;
    let batch = crate::batch_evaluate(handle, rows);
    println!("batch_evaluate output: {}", batch);

    let batch_json: serde_json::Value = serde_json::from_str(&batch).unwrap();
    assert_eq!(batch_json["ok"], serde_json::json!(true), "batch must answer ok:true");
    let values = batch_json["values"].as_array().unwrap();
    assert_eq!(values.len(), 3);
    assert_eq!(values[0]["value"], serde_json::json!(5.0), "row 0: 10 / 2");
    assert_eq!(values[1]["type"], serde_json::json!("empty"), "row 1: 10 / 0 must be empty");
    assert_eq!(values[2]["value"], serde_json::json!(2.0), "row 2: 10 / 5");
}

#[test]
fn default_budget_has_100x_headroom() {
    // Decision 1: the default budget of 1 000 000 instructions leaves at least
    // 100x headroom over the formulas the app actually evaluates.
    let sugar = instruction_count(SUGAR_FORMULA);
    let nested3 = instruction_count(NESTED3_FORMULA);
    println!("headroom sugar instructions: {}", sugar);
    println!("headroom nested3 instructions: {}", nested3);

    assert!(
        sugar <= DEFAULT_MAX_FUEL / 100,
        "sugar formula costs {} instructions, budget/100 is {}",
        sugar,
        DEFAULT_MAX_FUEL / 100
    );
    assert!(
        nested3 <= DEFAULT_MAX_FUEL / 100,
        "nested3 formula costs {} instructions, budget/100 is {}",
        nested3,
        DEFAULT_MAX_FUEL / 100
    );
}
