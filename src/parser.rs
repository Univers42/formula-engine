// ═══════════════════════════════════════════════════════════════════════════════
// FORMULA ENGINE — PARSER (Recursive Descent → AST)
// ═══════════════════════════════════════════════════════════════════════════════
// Pratt parser with recursive descent for function calls, let-bindings, and
// lambda expressions. Produces an AST for the compiler.
//
// Grammar (simplified EBNF):
//   expr        = or_expr
//   or_expr     = and_expr ( "or" and_expr )*
//   and_expr    = equality ( "and" equality )*
//   equality    = comparison ( ("==" | "!=") comparison )*
//   comparison  = addition ( ("<" | ">" | "<=" | ">=") addition )*
//   addition    = multiply ( ("+" | "-") multiply )*
//   multiply    = power ( ("*" | "/" | "%") power )*
//   power       = unary ( "^" power )?           (right-assoc)
//   unary       = ("not" | "-") unary | call
//   call        = primary ( "(" args? ")" )*
//   primary     = NUMBER | STRING | "true" | "false" | IDENT | "(" expr ")"

use crate::error::{ErrorKind, FormulaError, Span};
use crate::lexer::{Token, TokenKind};

// ─── AST Node Types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AstNode {
    pub expr: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    Number(f64),
    Text(String),
    Boolean(bool),
    Empty,
    Array(Vec<AstNode>),

    // Variable / property reference
    Identifier(String),

    // Binary operations
    BinaryOp {
        op: BinOp,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },

    // Unary operations
    UnaryOp {
        op: UnaryOp,
        operand: Box<AstNode>,
    },

    // Function call: name(arg1, arg2, ...)
    FunctionCall {
        name: String,
        args: Vec<AstNode>,
    },

    // Lambda: ident => expr  OR  (a, b) => expr
    Lambda {
        params: Vec<String>,
        body: Box<AstNode>,
    },

    // Let binding: let(name, value, body)  — handled as function call
    // Lets binding: lets(name = val, ..., body) — handled as special parse

    // Property access shorthand [field name]
    PropertyAccess(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Negate,
    Not,
}

// ─── Parser ───────────────────────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(mut self) -> crate::error::Result<AstNode> {
        let node = self.expr()?;
        if !self.at_end() {
            let tok = &self.tokens[self.pos];
            return Err(FormulaError::new(
                ErrorKind::UnexpectedToken(format!("{:?}", tok.kind)),
                Some(tok.span.clone()),
            ));
        }
        Ok(node)
    }

    // ─── Helpers ──────────────────────────────────────────────

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_span(&self) -> Span {
        self.tokens[self.pos].span.clone()
    }

    fn at_end(&self) -> bool {
        matches!(self.peek(), TokenKind::Eof)
    }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos];
        if !self.at_end() {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(kind)
    }

    fn expect(&mut self, expected: &TokenKind) -> crate::error::Result<Token> {
        if self.check(expected) {
            Ok(self.advance().clone())
        } else {
            let span = self.peek_span();
            Err(FormulaError::new(
                ErrorKind::ExpectedToken(
                    format!("{:?}", expected),
                    format!("{:?}", self.peek()),
                ),
                Some(span),
            ))
        }
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    // ─── Expression parsing (Pratt-style precedence climbing) ─

    fn expr(&mut self) -> crate::error::Result<AstNode> {
        self.or_expr()
    }

    fn or_expr(&mut self) -> crate::error::Result<AstNode> {
        let mut left = self.and_expr()?;
        while matches!(self.peek(), TokenKind::Or) {
            self.advance();
            let right = self.and_expr()?;
            let span = Span { start: left.span.start, end: right.span.end };
            left = AstNode {
                expr: Expr::BinaryOp {
                    op: BinOp::Or,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> crate::error::Result<AstNode> {
        let mut left = self.equality()?;
        while matches!(self.peek(), TokenKind::And) {
            self.advance();
            let right = self.equality()?;
            let span = Span { start: left.span.start, end: right.span.end };
            left = AstNode {
                expr: Expr::BinaryOp {
                    op: BinOp::And,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn equality(&mut self) -> crate::error::Result<AstNode> {
        let mut left = self.comparison()?;
        loop {
            let op = match self.peek() {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::BangEq => BinOp::Neq,
                _ => break,
            };
            self.advance();
            let right = self.comparison()?;
            let span = Span { start: left.span.start, end: right.span.end };
            left = AstNode {
                expr: Expr::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn comparison(&mut self) -> crate::error::Result<AstNode> {
        let mut left = self.addition()?;
        loop {
            let op = match self.peek() {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::LtEq => BinOp::Lte,
                TokenKind::GtEq => BinOp::Gte,
                _ => break,
            };
            self.advance();
            let right = self.addition()?;
            let span = Span { start: left.span.start, end: right.span.end };
            left = AstNode {
                expr: Expr::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn addition(&mut self) -> crate::error::Result<AstNode> {
        let mut left = self.multiplication()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.multiplication()?;
            let span = Span { start: left.span.start, end: right.span.end };
            left = AstNode {
                expr: Expr::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn multiplication(&mut self) -> crate::error::Result<AstNode> {
        let mut left = self.power()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.power()?;
            let span = Span { start: left.span.start, end: right.span.end };
            left = AstNode {
                expr: Expr::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                span,
            };
        }
        Ok(left)
    }

    fn power(&mut self) -> crate::error::Result<AstNode> {
        let base = self.unary()?;
        if matches!(self.peek(), TokenKind::Caret) {
            self.advance();
            // Right-associative: recurse into power
            let exp = self.power()?;
            let span = Span { start: base.span.start, end: exp.span.end };
            Ok(AstNode {
                expr: Expr::BinaryOp {
                    op: BinOp::Pow,
                    left: Box::new(base),
                    right: Box::new(exp),
                },
                span,
            })
        } else {
            Ok(base)
        }
    }

    fn unary(&mut self) -> crate::error::Result<AstNode> {
        match self.peek() {
            TokenKind::Not => {
                let start = self.peek_span().start;
                self.advance();
                let operand = self.unary()?;
                let span = Span { start, end: operand.span.end };
                Ok(AstNode {
                    expr: Expr::UnaryOp {
                        op: UnaryOp::Not,
                        operand: Box::new(operand),
                    },
                    span,
                })
            }
            TokenKind::Minus => {
                let start = self.peek_span().start;
                self.advance();
                let operand = self.unary()?;
                let span = Span { start, end: operand.span.end };
                Ok(AstNode {
                    expr: Expr::UnaryOp {
                        op: UnaryOp::Negate,
                        operand: Box::new(operand),
                    },
                    span,
                })
            }
            _ => self.call(),
        }
    }

    fn call(&mut self) -> crate::error::Result<AstNode> {
        let mut node = self.primary()?;

        // Check for function call: ident(...)
        while matches!(self.peek(), TokenKind::LParen) {
            if let Expr::Identifier(name) = &node.expr {
                let name = name.clone();
                let start = node.span.start;
                self.advance(); // consume '('

                // Special handling for "lets" — parses `lets(x = expr, y = expr, body)`
                if name == "lets" {
                    return self.parse_lets(start);
                }

                let args = self.parse_args()?;
                let end = self.peek_span().end;
                self.expect(&TokenKind::RParen)?;

                node = AstNode {
                    expr: Expr::FunctionCall { name, args },
                    span: Span { start, end },
                };
            } else {
                break;
            }
        }

        Ok(node)
    }

    fn primary(&mut self) -> crate::error::Result<AstNode> {
        let span = self.peek_span();

        match self.peek().clone() {
            TokenKind::Number(n) => {
                self.advance();
                Ok(AstNode { expr: Expr::Number(n), span })
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(AstNode { expr: Expr::Text(s), span })
            }
            TokenKind::True => {
                self.advance();
                Ok(AstNode { expr: Expr::Boolean(true), span })
            }
            TokenKind::False => {
                self.advance();
                Ok(AstNode { expr: Expr::Boolean(false), span })
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();

                // Check for lambda: ident => expr
                if matches!(self.peek(), TokenKind::Arrow) {
                    self.advance(); // consume =>
                    let body = self.expr()?;
                    let end = body.span.end;
                    return Ok(AstNode {
                        expr: Expr::Lambda {
                            params: vec![name],
                            body: Box::new(body),
                        },
                        span: Span { start: span.start, end },
                    });
                }

                // Check if the identifier is "empty" used as a value literal
                if name == "empty" && !matches!(self.peek(), TokenKind::LParen) {
                    return Ok(AstNode { expr: Expr::Empty, span });
                }

                Ok(AstNode { expr: Expr::Identifier(name), span })
            }
            // Keywords used as function calls: and(...), or(...), not(...)
            // These are normally infix/unary operators, but Notion also allows
            // them as function-call syntax with multiple arguments.
            TokenKind::And | TokenKind::Or | TokenKind::Not => {
                let name = match self.peek() {
                    TokenKind::And => "and",
                    TokenKind::Or => "or",
                    TokenKind::Not => "not",
                    _ => unreachable!(),
                }.to_string();
                self.advance();
                Ok(AstNode { expr: Expr::Identifier(name), span })
            }
            TokenKind::LParen => {
                self.advance();

                // Check for lambda: (a, b) => expr
                if self.could_be_lambda_params() {
                    if let Some(lambda) = self.try_parse_lambda(span.start)? {
                        return Ok(lambda);
                    }
                }

                let inner = self.expr()?;
                let end_span = self.peek_span();
                self.expect(&TokenKind::RParen)?;
                Ok(AstNode {
                    expr: inner.expr,
                    span: Span { start: span.start, end: end_span.end },
                })
            }
            TokenKind::LBracket => {
                // Property access shorthand: [field name]
                self.advance();
                if let TokenKind::String(name) | TokenKind::Ident(name) = self.peek().clone() {
                    let name = name.clone();
                    self.advance();
                    let end = self.peek_span().end;
                    self.expect(&TokenKind::RBracket)?;
                    Ok(AstNode {
                        expr: Expr::PropertyAccess(name),
                        span: Span { start: span.start, end },
                    })
                } else {
                    // Array literal [a, b, c]
                    let mut elements = Vec::new();
                    if !matches!(self.peek(), TokenKind::RBracket) {
                        elements.push(self.expr()?);
                        while self.match_token(&TokenKind::Comma) {
                            if matches!(self.peek(), TokenKind::RBracket) { break; }
                            elements.push(self.expr()?);
                        }
                    }
                    let end = self.peek_span().end;
                    self.expect(&TokenKind::RBracket)?;
                    Ok(AstNode {
                        expr: Expr::Array(elements),
                        span: Span { start: span.start, end },
                    })
                }
            }
            _ => {
                Err(FormulaError::new(
                    ErrorKind::UnexpectedToken(format!("{:?}", self.peek())),
                    Some(span),
                ))
            }
        }
    }

    // ─── Helpers for complex parsing ──────────────────────────

    fn parse_args(&mut self) -> crate::error::Result<Vec<AstNode>> {
        let mut args = Vec::new();
        if !matches!(self.peek(), TokenKind::RParen) {
            args.push(self.expr()?);
            while self.match_token(&TokenKind::Comma) {
                if matches!(self.peek(), TokenKind::RParen) { break; }
                args.push(self.expr()?);
            }
        }
        Ok(args)
    }

    fn parse_lets(&mut self, start: usize) -> crate::error::Result<AstNode> {
        // lets(x = expr1, y = expr2, body)
        // We parse as a FunctionCall with special arg structure:
        // args = [Identifier("x"), expr1, Identifier("y"), expr2, body_expr]
        let mut args = Vec::new();

        loop {
            // Try to parse "name = value" binding
            let save_pos = self.pos;
            if let TokenKind::Ident(name) = self.peek().clone() {
                let name_span = self.peek_span();
                self.advance();
                if self.match_token(&TokenKind::Eq) {
                    // It's a binding: name = value
                    args.push(AstNode {
                        expr: Expr::Identifier(name),
                        span: name_span,
                    });
                    args.push(self.expr()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                    continue;
                } else {
                    // Not a binding — rewind and parse as final body expression
                    self.pos = save_pos;
                }
            }

            // Final argument is the body expression
            args.push(self.expr()?);
            break;
        }

        let end = self.peek_span().end;
        self.expect(&TokenKind::RParen)?;

        Ok(AstNode {
            expr: Expr::FunctionCall {
                name: "lets".into(),
                args,
            },
            span: Span { start, end },
        })
    }

    fn could_be_lambda_params(&self) -> bool {
        // Quick lookahead: check if we see "ident, ident, ...) =>"
        let mut i = self.pos;
        loop {
            if i >= self.tokens.len() { return false; }
            match &self.tokens[i].kind {
                TokenKind::Ident(_) => { i += 1; }
                TokenKind::Comma => { i += 1; }
                TokenKind::RParen => {
                    i += 1;
                    return i < self.tokens.len() && matches!(self.tokens[i].kind, TokenKind::Arrow);
                }
                _ => return false,
            }
        }
    }

    fn try_parse_lambda(&mut self, start: usize) -> crate::error::Result<Option<AstNode>> {
        let save = self.pos;
        let mut params = Vec::new();

        loop {
            match self.peek() {
                TokenKind::Ident(name) => {
                    params.push(name.clone());
                    self.advance();
                    if self.match_token(&TokenKind::Comma) {
                        continue;
                    }
                }
                _ => {}
            }
            break;
        }

        if self.match_token(&TokenKind::RParen) && self.match_token(&TokenKind::Arrow) {
            let body = self.expr()?;
            let end = body.span.end;
            Ok(Some(AstNode {
                expr: Expr::Lambda { params, body: Box::new(body) },
                span: Span { start, end },
            }))
        } else {
            // Not a lambda — rewind
            self.pos = save;
            Ok(None)
        }
    }
}
