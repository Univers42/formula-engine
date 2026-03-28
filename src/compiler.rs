// ═══════════════════════════════════════════════════════════════════════════════
// FORMULA ENGINE — COMPILER (AST → Bytecode)
// ═══════════════════════════════════════════════════════════════════════════════
// Walks the AST and emits a flat bytecode sequence for the stack VM.

use crate::error::{ErrorKind, FormulaError, Span};
use crate::parser::{AstNode, BinOp, Expr, UnaryOp};
use crate::types::Value;

// ─── Bytecode Instructions ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum OpCode {
    /// Push a constant from the constant pool
    Constant(usize),
    /// Pop top of stack
    Pop,
    /// Push Empty value
    PushEmpty,

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Negate,

    // Comparison
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,

    // Logical
    And,
    Or,
    Not,

    // Control flow
    /// Jump if top-of-stack is falsy (pops condition)
    JumpIfFalse(usize),
    /// Unconditional jump
    Jump(usize),

    // Variables
    /// Load a local variable by slot index
    GetLocal(usize),
    /// Set a local variable by slot index
    SetLocal(usize),

    // Property access
    /// Load a property by name (index into string pool)
    LoadProp(usize),

    // Function calls
    /// Call built-in function by name index, with N arguments on stack
    CallBuiltin(usize, u8),

    // Lambda
    /// Create a closure: captures current scope, body at code offset, param count
    MakeClosure(usize, u8),
    /// Call a closure on top of stack with N args below it
    CallClosure(u8),

    /// Return / end of program
    Return,
}

// ─── Compiled Bytecode Chunk ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub strings: Vec<String>,
    /// Property names referenced by the formula (for dependency tracking)
    pub dependencies: Vec<String>,
    /// Lambda bodies indexed by their offset
    pub lambda_bodies: Vec<LambdaBody>,
}

#[derive(Debug, Clone)]
pub struct LambdaBody {
    pub code: Vec<OpCode>,
    pub param_count: u8,
    pub param_names: Vec<String>,
}

// ─── Compiler State ───────────────────────────────────────────────────────────

struct Scope {
    locals: Vec<(String, usize)>, // (name, slot)
    depth: usize,
}

pub struct Compiler {
    chunk: Chunk,
    scope: Scope,
    next_slot: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk {
                code: Vec::new(),
                constants: Vec::new(),
                strings: Vec::new(),
                dependencies: Vec::new(),
                lambda_bodies: Vec::new(),
            },
            scope: Scope {
                locals: Vec::new(),
                depth: 0,
            },
            next_slot: 0,
        }
    }

    pub fn compile(mut self, ast: &AstNode) -> crate::error::Result<Chunk> {
        self.compile_node(ast)?;
        self.emit(OpCode::Return);
        Ok(self.chunk)
    }

    // ─── Emit helpers ─────────────────────────────────────────

    fn emit(&mut self, op: OpCode) -> usize {
        let idx = self.chunk.code.len();
        self.chunk.code.push(op);
        idx
    }

    fn emit_constant(&mut self, val: Value) -> usize {
        let idx = self.chunk.constants.len();
        self.chunk.constants.push(val);
        self.emit(OpCode::Constant(idx))
    }

    fn add_string(&mut self, s: &str) -> usize {
        if let Some(idx) = self.chunk.strings.iter().position(|x| x == s) {
            idx
        } else {
            let idx = self.chunk.strings.len();
            self.chunk.strings.push(s.to_string());
            idx
        }
    }

    fn add_dependency(&mut self, name: &str) {
        if !self.chunk.dependencies.contains(&name.to_string()) {
            self.chunk.dependencies.push(name.to_string());
        }
    }

    fn resolve_local(&self, name: &str) -> Option<usize> {
        for (n, slot) in self.scope.locals.iter().rev() {
            if n == name {
                return Some(*slot);
            }
        }
        None
    }

    fn define_local(&mut self, name: &str) -> usize {
        let slot = self.next_slot;
        self.next_slot += 1;
        self.scope.locals.push((name.to_string(), slot));
        slot
    }

    fn current_offset(&self) -> usize {
        self.chunk.code.len()
    }

    fn patch_jump(&mut self, idx: usize) {
        let target = self.chunk.code.len();
        match &mut self.chunk.code[idx] {
            OpCode::JumpIfFalse(ref mut dest) => *dest = target,
            OpCode::Jump(ref mut dest) => *dest = target,
            _ => {}
        }
    }

    // ─── Compile AST → bytecode ────────────────────────────────

    fn compile_node(&mut self, node: &AstNode) -> crate::error::Result<()> {
        match &node.expr {
            Expr::Number(n) => {
                self.emit_constant(Value::Number(*n));
            }
            Expr::Text(s) => {
                self.emit_constant(Value::Text(s.clone()));
            }
            Expr::Boolean(b) => {
                self.emit_constant(Value::Boolean(*b));
            }
            Expr::Empty => {
                self.emit(OpCode::PushEmpty);
            }
            Expr::Array(elements) => {
                // Push all elements, then call array construction builtin
                let count = elements.len();
                for el in elements {
                    self.compile_node(el)?;
                }
                let name_idx = self.add_string("__array_construct");
                self.emit(OpCode::CallBuiltin(name_idx, count as u8));
            }

            Expr::Identifier(name) => {
                // Check local scope first, then treat as property
                if let Some(slot) = self.resolve_local(name) {
                    self.emit(OpCode::GetLocal(slot));
                } else {
                    // Treat as property reference (like Notion's `prop("Name")`)
                    let idx = self.add_string(name);
                    self.add_dependency(name);
                    self.emit(OpCode::LoadProp(idx));
                }
            }

            Expr::PropertyAccess(name) => {
                let idx = self.add_string(name);
                self.add_dependency(name);
                self.emit(OpCode::LoadProp(idx));
            }

            Expr::BinaryOp { op, left, right } => {
                // Short-circuit for And/Or
                match op {
                    BinOp::And => {
                        self.compile_node(left)?;
                        let jump = self.emit(OpCode::JumpIfFalse(0));
                        self.emit(OpCode::Pop);
                        self.compile_node(right)?;
                        self.patch_jump(jump);
                        return Ok(());
                    }
                    BinOp::Or => {
                        self.compile_node(left)?;
                        // If truthy, skip right side
                        let jump_true = self.emit(OpCode::JumpIfFalse(0));
                        let jump_end = self.emit(OpCode::Jump(0));
                        self.patch_jump(jump_true);
                        self.emit(OpCode::Pop);
                        self.compile_node(right)?;
                        self.patch_jump(jump_end);
                        return Ok(());
                    }
                    _ => {}
                }

                self.compile_node(left)?;
                self.compile_node(right)?;

                match op {
                    BinOp::Add => self.emit(OpCode::Add),
                    BinOp::Sub => self.emit(OpCode::Sub),
                    BinOp::Mul => self.emit(OpCode::Mul),
                    BinOp::Div => self.emit(OpCode::Div),
                    BinOp::Mod => self.emit(OpCode::Mod),
                    BinOp::Pow => self.emit(OpCode::Pow),
                    BinOp::Eq => self.emit(OpCode::Eq),
                    BinOp::Neq => self.emit(OpCode::Neq),
                    BinOp::Lt => self.emit(OpCode::Lt),
                    BinOp::Gt => self.emit(OpCode::Gt),
                    BinOp::Lte => self.emit(OpCode::Lte),
                    BinOp::Gte => self.emit(OpCode::Gte),
                    BinOp::And | BinOp::Or => unreachable!(),
                };
            }

            Expr::UnaryOp { op, operand } => {
                self.compile_node(operand)?;
                match op {
                    UnaryOp::Negate => self.emit(OpCode::Negate),
                    UnaryOp::Not => self.emit(OpCode::Not),
                };
            }

            Expr::FunctionCall { name, args } => {
                self.compile_function_call(name, args, &node.span)?;
            }

            Expr::Lambda { params, body } => {
                self.compile_lambda(params, body)?;
            }
        }

        Ok(())
    }

    fn compile_function_call(
        &mut self,
        name: &str,
        args: &[AstNode],
        _span: &Span,
    ) -> crate::error::Result<()> {
        match name {
            // Special forms that need compile-time handling
            "if" => self.compile_if(args),
            "ifs" => self.compile_ifs(args),
            "let" => self.compile_let(args),
            "lets" => self.compile_lets(args),
            "prop" => self.compile_prop(args),

            // All other functions: push args, call builtin
            _ => {
                let arg_count = args.len();
                for arg in args {
                    self.compile_node(arg)?;
                }
                let name_idx = self.add_string(name);
                self.emit(OpCode::CallBuiltin(name_idx, arg_count as u8));
                Ok(())
            }
        }
    }

    fn compile_if(&mut self, args: &[AstNode]) -> crate::error::Result<()> {
        // if(condition, then_value) OR if(condition, then_value, else_value)
        if args.is_empty() || args.len() > 3 {
            return Err(FormulaError::new(
                ErrorKind::ArityMismatch("if".into(), "2 or 3".into(), args.len()),
                None,
            ));
        }

        self.compile_node(&args[0])?; // condition
        let jump_false = self.emit(OpCode::JumpIfFalse(0));
        self.emit(OpCode::Pop);

        self.compile_node(&args[1])?; // then
        let jump_end = self.emit(OpCode::Jump(0));

        self.patch_jump(jump_false);
        self.emit(OpCode::Pop);

        if args.len() == 3 {
            self.compile_node(&args[2])?; // else
        } else {
            self.emit(OpCode::PushEmpty);
        }

        self.patch_jump(jump_end);
        Ok(())
    }

    fn compile_ifs(&mut self, args: &[AstNode]) -> crate::error::Result<()> {
        // ifs(cond1, val1, cond2, val2, ...) — optionally odd count makes last the default
        if args.len() < 2 {
            return Err(FormulaError::new(
                ErrorKind::ArityMismatch("ifs".into(), "at least 2".into(), args.len()),
                None,
            ));
        }

        let mut end_jumps = Vec::new();
        let pairs = args.len() / 2;

        for i in 0..pairs {
            self.compile_node(&args[i * 2])?;     // condition
            let jump_next = self.emit(OpCode::JumpIfFalse(0));
            self.emit(OpCode::Pop);
            self.compile_node(&args[i * 2 + 1])?; // value
            end_jumps.push(self.emit(OpCode::Jump(0)));
            self.patch_jump(jump_next);
            self.emit(OpCode::Pop);
        }

        // Default value (odd arg count)
        if args.len() % 2 == 1 {
            self.compile_node(args.last().unwrap())?;
        } else {
            self.emit(OpCode::PushEmpty);
        }

        for j in end_jumps {
            self.patch_jump(j);
        }

        Ok(())
    }

    fn compile_let(&mut self, args: &[AstNode]) -> crate::error::Result<()> {
        // let(name, value, body)
        if args.len() != 3 {
            return Err(FormulaError::new(
                ErrorKind::ArityMismatch("let".into(), "3".into(), args.len()),
                None,
            ));
        }

        let name = match &args[0].expr {
            Expr::Identifier(n) | Expr::Text(n) => n.clone(),
            _ => {
                return Err(FormulaError::new(
                    ErrorKind::TypeError("let: first argument must be a name".into()),
                    Some(args[0].span.clone()),
                ))
            }
        };

        // Compile value
        self.compile_node(&args[1])?;

        // Define local
        let slot = self.define_local(&name);
        self.emit(OpCode::SetLocal(slot));
        self.emit(OpCode::Pop);

        // Compile body
        self.compile_node(&args[2])?;

        // Remove the local from scope
        self.scope.locals.pop();

        Ok(())
    }

    fn compile_lets(&mut self, args: &[AstNode]) -> crate::error::Result<()> {
        // lets(x = expr1, y = expr2, body)
        // Parsed as: [Ident("x"), expr1, Ident("y"), expr2, body]
        if args.len() < 3 || args.len() % 2 == 0 {
            return Err(FormulaError::new(
                ErrorKind::ArityMismatch("lets".into(), "odd number >= 3".into(), args.len()),
                None,
            ));
        }

        let binding_count = (args.len() - 1) / 2;

        for i in 0..binding_count {
            let name = match &args[i * 2].expr {
                Expr::Identifier(n) => n.clone(),
                _ => {
                    return Err(FormulaError::new(
                        ErrorKind::TypeError("lets: binding name must be an identifier".into()),
                        Some(args[i * 2].span.clone()),
                    ))
                }
            };

            self.compile_node(&args[i * 2 + 1])?;
            let slot = self.define_local(&name);
            self.emit(OpCode::SetLocal(slot));
            self.emit(OpCode::Pop);
        }

        // Compile body
        self.compile_node(args.last().unwrap())?;

        // Pop locals
        for _ in 0..binding_count {
            self.scope.locals.pop();
        }

        Ok(())
    }

    fn compile_prop(&mut self, args: &[AstNode]) -> crate::error::Result<()> {
        // prop("Property Name")
        if args.len() != 1 {
            return Err(FormulaError::new(
                ErrorKind::ArityMismatch("prop".into(), "1".into(), args.len()),
                None,
            ));
        }

        match &args[0].expr {
            Expr::Text(name) | Expr::Identifier(name) => {
                let idx = self.add_string(name);
                self.add_dependency(name);
                self.emit(OpCode::LoadProp(idx));
            }
            _ => {
                // Dynamic prop — compile the expression and use runtime prop resolution
                self.compile_node(&args[0])?;
                let name_idx = self.add_string("prop");
                self.emit(OpCode::CallBuiltin(name_idx, 1));
            }
        }

        Ok(())
    }

    fn compile_lambda(&mut self, params: &[String], body: &AstNode) -> crate::error::Result<()> {
        // Save current compiler state
        let saved_locals = self.scope.locals.clone();
        let saved_next_slot = self.next_slot;

        // Define params as locals for the lambda body
        let param_names: Vec<String> = params.to_vec();
        for p in params {
            self.define_local(p);
        }

        // Compile body into a separate code vec
        let mut lambda_code = Vec::new();
        std::mem::swap(&mut self.chunk.code, &mut lambda_code);
        self.compile_node(body)?;
        self.emit(OpCode::Return);
        std::mem::swap(&mut self.chunk.code, &mut lambda_code);

        // Restore scope
        self.scope.locals = saved_locals;
        self.next_slot = saved_next_slot;

        let lambda_idx = self.chunk.lambda_bodies.len();
        self.chunk.lambda_bodies.push(LambdaBody {
            code: lambda_code,
            param_count: params.len() as u8,
            param_names,
        });

        self.emit(OpCode::MakeClosure(lambda_idx, params.len() as u8));

        Ok(())
    }
}
