// ═══════════════════════════════════════════════════════════════════════════════
// FORMULA ENGINE — VIRTUAL MACHINE (Stack-based bytecode executor)
// ═══════════════════════════════════════════════════════════════════════════════
// Executes compiled bytecode chunks against a property context.
// Supports closures, local variables, and null-propagating arithmetic.

use std::collections::HashMap;
use crate::compiler::{Chunk, LambdaBody, OpCode};
use crate::error::{ErrorKind, FormulaError};
use crate::functions;
use crate::types::{ClosureData, Value};

// Re-export ClosureData as Closure for convenience
pub type Closure = ClosureData;

// ─── Evaluation Context ───────────────────────────────────────────────────────

/// Properties (row data) passed from JavaScript
pub struct EvalContext {
    pub properties: HashMap<String, Value>,
}

impl EvalContext {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }

    pub fn with_properties(props: HashMap<String, Value>) -> Self {
        Self { properties: props }
    }
}

// ─── VM ───────────────────────────────────────────────────────────────────────

pub struct VM {
    stack: Vec<Value>,
    locals: Vec<Value>,
    max_stack: usize,
}

const MAX_STACK_SIZE: usize = 10_000;
const MAX_CALL_DEPTH: usize = 256;

impl VM {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(256),
            locals: Vec::new(),
            max_stack: MAX_STACK_SIZE,
        }
    }

    pub fn execute(&mut self, chunk: &Chunk, ctx: &EvalContext) -> crate::error::Result<Value> {
        self.stack.clear();
        self.locals.clear();
        // Pre-allocate enough local slots
        self.locals.resize(64, Value::Empty);

        self.run(&chunk.code, chunk, ctx, 0)?;

        Ok(self.stack.pop().unwrap_or(Value::Empty))
    }

    fn run(
        &mut self,
        code: &[OpCode],
        chunk: &Chunk,
        ctx: &EvalContext,
        call_depth: usize,
    ) -> crate::error::Result<()> {
        if call_depth > MAX_CALL_DEPTH {
            return Err(FormulaError::new(ErrorKind::StackOverflow, None));
        }

        let mut ip = 0;

        while ip < code.len() {
            if self.stack.len() > self.max_stack {
                return Err(FormulaError::new(ErrorKind::StackOverflow, None));
            }

            match &code[ip] {
                OpCode::Constant(idx) => {
                    self.stack.push(chunk.constants[*idx].clone());
                }
                OpCode::PushEmpty => {
                    self.stack.push(Value::Empty);
                }
                OpCode::Pop => {
                    self.stack.pop();
                }

                // ─── Arithmetic ───────────────────────────────
                OpCode::Add => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(self.op_add(a, b));
                }
                OpCode::Sub => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(self.op_arith(a, b, |x, y| x - y));
                }
                OpCode::Mul => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(self.op_arith(a, b, |x, y| x * y));
                }
                OpCode::Div => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    if let Value::Number(bv) = &b {
                        if *bv == 0.0 {
                            return Err(FormulaError::new(ErrorKind::DivisionByZero, None));
                        }
                    }
                    self.stack.push(self.op_arith(a, b, |x, y| x / y));
                }
                OpCode::Mod => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(self.op_arith(a, b, |x, y| x % y));
                }
                OpCode::Pow => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(self.op_arith(a, b, |x, y| x.powf(y)));
                }
                OpCode::Negate => {
                    let val = self.pop()?;
                    match val {
                        Value::Number(n) => self.stack.push(Value::Number(-n)),
                        Value::Empty => self.stack.push(Value::Empty),
                        _ => {
                            return Err(FormulaError::new(
                                ErrorKind::TypeError(format!(
                                    "Cannot negate {}",
                                    val.type_name()
                                )),
                                None,
                            ));
                        }
                    }
                }

                // ─── Comparison ───────────────────────────────
                OpCode::Eq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(Value::Boolean(a == b));
                }
                OpCode::Neq => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(Value::Boolean(a != b));
                }
                OpCode::Lt => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(Value::Boolean(a < b));
                }
                OpCode::Gt => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(Value::Boolean(a > b));
                }
                OpCode::Lte => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack
                        .push(Value::Boolean(a < b || a == b));
                }
                OpCode::Gte => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack
                        .push(Value::Boolean(a > b || a == b));
                }

                // ─── Logical ──────────────────────────────────
                OpCode::And => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(Value::Boolean(a.to_bool() && b.to_bool()));
                }
                OpCode::Or => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(Value::Boolean(a.to_bool() || b.to_bool()));
                }
                OpCode::Not => {
                    let val = self.pop()?;
                    self.stack.push(Value::Boolean(!val.to_bool()));
                }

                // ─── Control flow ─────────────────────────────
                OpCode::JumpIfFalse(target) => {
                    let val = self.stack.last().unwrap_or(&Value::Empty);
                    if !val.to_bool() {
                        ip = *target;
                        continue;
                    }
                }
                OpCode::Jump(target) => {
                    ip = *target;
                    continue;
                }

                // ─── Variables ────────────────────────────────
                OpCode::GetLocal(slot) => {
                    self.ensure_locals(*slot);
                    self.stack.push(self.locals[*slot].clone());
                }
                OpCode::SetLocal(slot) => {
                    self.ensure_locals(*slot);
                    let val = self.stack.last().cloned().unwrap_or(Value::Empty);
                    self.locals[*slot] = val;
                }

                // ─── Properties ───────────────────────────────
                OpCode::LoadProp(str_idx) => {
                    let name = &chunk.strings[*str_idx];
                    let val = ctx
                        .properties
                        .get(name)
                        .cloned()
                        .unwrap_or(Value::Empty);
                    self.stack.push(val);
                }

                // ─── Function calls ───────────────────────────
                OpCode::CallBuiltin(name_idx, arg_count) => {
                    let name = &chunk.strings[*name_idx];
                    let argc = *arg_count as usize;
                    let start = self.stack.len().saturating_sub(argc);
                    let args: Vec<Value> = self.stack.drain(start..).collect();

                    let result =
                        functions::call_builtin(name, args, chunk, ctx, self, call_depth)?;
                    self.stack.push(result);
                }

                // ─── Closures ─────────────────────────────────
                OpCode::MakeClosure(body_idx, _param_count) => {
                    // Capture current locals
                    let captured: Vec<(usize, Value)> = self
                        .locals
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| !matches!(v, Value::Empty))
                        .map(|(i, v)| (i, v.clone()))
                        .collect();

                    let closure = ClosureData {
                        body_idx: *body_idx,
                        captured_locals: captured,
                    };
                    self.stack.push(Value::closure_marker(closure));
                }

                OpCode::CallClosure(arg_count) => {
                    let argc = *arg_count as usize;
                    // Closure is below the args
                    let closure_pos = self.stack.len() - argc - 1;
                    let closure_val = self.stack[closure_pos].clone();

                    if let Some(closure) = Value::as_closure(&closure_val) {
                        let lambda = &chunk.lambda_bodies[closure.body_idx];
                        let start = self.stack.len().saturating_sub(argc);
                        let args: Vec<Value> = self.stack.drain(start..).collect();
                        // Remove the closure value itself
                        self.stack.pop();

                        let result = self.call_closure(
                            &closure, lambda, &args, chunk, ctx, call_depth,
                        )?;
                        self.stack.push(result);
                    } else {
                        return Err(FormulaError::new(
                            ErrorKind::TypeError("Expected a function/lambda".into()),
                            None,
                        ));
                    }
                }

                OpCode::Return => {
                    return Ok(());
                }
            }

            ip += 1;
        }

        Ok(())
    }

    // ─── Helpers ──────────────────────────────────────────────

    fn pop(&mut self) -> crate::error::Result<Value> {
        self.stack.pop().ok_or_else(|| {
            FormulaError::new(ErrorKind::StackOverflow, None)
        })
    }

    fn ensure_locals(&mut self, slot: usize) {
        if slot >= self.locals.len() {
            self.locals.resize(slot + 1, Value::Empty);
        }
    }

    /// Null-propagating arithmetic: if either side is Empty, result is Empty.
    fn op_arith<F>(&self, a: Value, b: Value, f: F) -> Value
    where
        F: Fn(f64, f64) -> f64,
    {
        match (&a, &b) {
            (Value::Empty, _) | (_, Value::Empty) => Value::Empty,
            _ => {
                let an = a.to_number();
                let bn = b.to_number();
                match (an, bn) {
                    (Some(x), Some(y)) => Value::Number(f(x, y)),
                    _ => Value::Empty,
                }
            }
        }
    }

    /// Add: numbers add, strings concatenate, date + number = date shifted
    fn op_add(&self, a: Value, b: Value) -> Value {
        match (&a, &b) {
            (Value::Empty, _) | (_, Value::Empty) => Value::Empty,
            (Value::Text(s1), Value::Text(s2)) => {
                Value::Text(format!("{}{}", s1, s2))
            }
            (Value::Text(s), _) => {
                Value::Text(format!("{}{}", s, b.to_text()))
            }
            (_, Value::Text(s)) => {
                Value::Text(format!("{}{}", a.to_text(), s))
            }
            (Value::Date(d), Value::Number(n)) => {
                // Add days to date
                let ms_per_day = 86_400_000i64;
                Value::Date(d + (*n as i64) * ms_per_day)
            }
            (Value::Number(n), Value::Date(d)) => {
                let ms_per_day = 86_400_000i64;
                Value::Date(d + (*n as i64) * ms_per_day)
            }
            _ => {
                let an = a.to_number();
                let bn = b.to_number();
                match (an, bn) {
                    (Some(x), Some(y)) => Value::Number(x + y),
                    _ => Value::Empty,
                }
            }
        }
    }

    /// Execute a closure/lambda with given arguments
    pub fn call_closure(
        &mut self,
        closure: &ClosureData,
        lambda: &LambdaBody,
        args: &[Value],
        chunk: &Chunk,
        ctx: &EvalContext,
        call_depth: usize,
    ) -> crate::error::Result<Value> {
        // Save locals
        let saved_locals = self.locals.clone();

        // Restore captured locals
        for (slot, val) in &closure.captured_locals {
            self.ensure_locals(*slot);
            self.locals[*slot] = val.clone();
        }

        // Bind parameters
        for (i, arg) in args.iter().enumerate() {
            let slot = i;
            self.ensure_locals(slot);
            self.locals[slot] = arg.clone();
        }

        // Execute lambda body
        let saved_stack_len = self.stack.len();
        self.run(&lambda.code, chunk, ctx, call_depth + 1)?;

        let result = if self.stack.len() > saved_stack_len {
            self.stack.pop().unwrap_or(Value::Empty)
        } else {
            Value::Empty
        };

        // Restore locals
        self.locals = saved_locals;

        Ok(result)
    }
}
