// ═══════════════════════════════════════════════════════════════════════════════
// FORMULA ENGINE — LEXER (Tokenizer)
// ═══════════════════════════════════════════════════════════════════════════════
// Hand-written lexer for maximum speed and error quality.
// Produces a flat Vec<Token> from a formula string.
// Handles: numbers, strings, identifiers, operators, keywords, comments.

use crate::error::{ErrorKind, FormulaError, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    String(String),

    // Identifiers & keywords
    Ident(String),
    True,
    False,
    And,
    Or,
    Not,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,

    // Comparison
    EqEq,      // ==
    BangEq,    // !=
    Lt,        // <
    Gt,        // >
    LtEq,     // <=
    GtEq,     // >=

    // Assignment
    Eq,        // = (used in lets)

    // Delimiters
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Dot,

    // Arrow (for lambdas)
    Arrow,     // =>

    // End
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer<'a> {
    source: &'a [u8],
    pos: usize,
    tokens: Vec<Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            pos: 0,
            tokens: Vec::with_capacity(64),
        }
    }

    pub fn tokenize(mut self) -> crate::error::Result<Vec<Token>> {
        while self.pos < self.source.len() {
            self.skip_whitespace_and_comments();
            if self.pos >= self.source.len() {
                break;
            }

            let start = self.pos;
            let ch = self.source[self.pos] as char;

            match ch {
                // Numbers
                '0'..='9' => self.lex_number()?,

                // Strings
                '"' | '\'' => self.lex_string()?,

                // Identifiers and keywords
                'a'..='z' | 'A'..='Z' | '_' => self.lex_ident(),

                // Operators
                '+' => { self.push_token(TokenKind::Plus, start, start + 1); self.pos += 1; }
                '-' => {
                    // Could be minus or negative number — always treat as operator, parser handles unary
                    self.push_token(TokenKind::Minus, start, start + 1);
                    self.pos += 1;
                }
                '*' => { self.push_token(TokenKind::Star, start, start + 1); self.pos += 1; }
                '/' => { self.push_token(TokenKind::Slash, start, start + 1); self.pos += 1; }
                '%' => { self.push_token(TokenKind::Percent, start, start + 1); self.pos += 1; }
                '^' => { self.push_token(TokenKind::Caret, start, start + 1); self.pos += 1; }

                // Comparison and assignment
                '=' => {
                    if self.peek_char(1) == '>' {
                        self.push_token(TokenKind::Arrow, start, start + 2);
                        self.pos += 2;
                    } else if self.peek_char(1) == '=' {
                        self.push_token(TokenKind::EqEq, start, start + 2);
                        self.pos += 2;
                    } else {
                        self.push_token(TokenKind::Eq, start, start + 1);
                        self.pos += 1;
                    }
                }
                '!' => {
                    if self.peek_char(1) == '=' {
                        self.push_token(TokenKind::BangEq, start, start + 2);
                        self.pos += 2;
                    } else {
                        return Err(FormulaError::new(
                            ErrorKind::UnexpectedCharacter('!'),
                            Some(Span { start, end: start + 1 }),
                        ));
                    }
                }
                '<' => {
                    if self.peek_char(1) == '=' {
                        self.push_token(TokenKind::LtEq, start, start + 2);
                        self.pos += 2;
                    } else {
                        self.push_token(TokenKind::Lt, start, start + 1);
                        self.pos += 1;
                    }
                }
                '>' => {
                    if self.peek_char(1) == '=' {
                        self.push_token(TokenKind::GtEq, start, start + 2);
                        self.pos += 2;
                    } else {
                        self.push_token(TokenKind::Gt, start, start + 1);
                        self.pos += 1;
                    }
                }

                // Delimiters
                '(' => { self.push_token(TokenKind::LParen, start, start + 1); self.pos += 1; }
                ')' => { self.push_token(TokenKind::RParen, start, start + 1); self.pos += 1; }
                '[' => { self.push_token(TokenKind::LBracket, start, start + 1); self.pos += 1; }
                ']' => { self.push_token(TokenKind::RBracket, start, start + 1); self.pos += 1; }
                ',' => { self.push_token(TokenKind::Comma, start, start + 1); self.pos += 1; }
                '.' => { self.push_token(TokenKind::Dot, start, start + 1); self.pos += 1; }

                _ => {
                    return Err(FormulaError::new(
                        ErrorKind::UnexpectedCharacter(ch),
                        Some(Span { start, end: start + 1 }),
                    ));
                }
            }
        }

        let eof_pos = self.source.len();
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span { start: eof_pos, end: eof_pos },
        });

        Ok(self.tokens)
    }

    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.source.len() {
            let ch = self.source[self.pos];
            if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                self.pos += 1;
            } else if ch == b'/' && self.peek_char(1) == '/' {
                // Line comment: skip to end of line
                while self.pos < self.source.len() && self.source[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn peek_char(&self, offset: usize) -> char {
        if self.pos + offset < self.source.len() {
            self.source[self.pos + offset] as char
        } else {
            '\0'
        }
    }

    fn lex_number(&mut self) -> crate::error::Result<()> {
        let start = self.pos;
        // Integer part
        while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        // Fractional part
        if self.pos < self.source.len() && self.source[self.pos] == b'.' {
            // Check it's not a method call like "value.prop"
            if self.pos + 1 < self.source.len() && self.source[self.pos + 1].is_ascii_digit() {
                self.pos += 1; // skip '.'
                while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
                    self.pos += 1;
                }
            }
        }
        // Scientific notation
        if self.pos < self.source.len() && (self.source[self.pos] == b'e' || self.source[self.pos] == b'E') {
            self.pos += 1;
            if self.pos < self.source.len() && (self.source[self.pos] == b'+' || self.source[self.pos] == b'-') {
                self.pos += 1;
            }
            while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        let text = std::str::from_utf8(&self.source[start..self.pos]).unwrap_or("0");
        let value: f64 = text.parse().map_err(|_| {
            FormulaError::new(
                ErrorKind::InvalidNumber(text.to_string()),
                Some(Span { start, end: self.pos }),
            )
        })?;
        self.push_token(TokenKind::Number(value), start, self.pos);
        Ok(())
    }

    fn lex_string(&mut self) -> crate::error::Result<()> {
        let start = self.pos;
        let quote = self.source[self.pos];
        self.pos += 1;

        let mut value = String::new();
        while self.pos < self.source.len() {
            let ch = self.source[self.pos];
            if ch == quote {
                self.pos += 1;
                self.push_token(TokenKind::String(value), start, self.pos);
                return Ok(());
            }
            if ch == b'\\' {
                self.pos += 1;
                if self.pos >= self.source.len() {
                    break;
                }
                let escaped = self.source[self.pos];
                match escaped {
                    b'n' => value.push('\n'),
                    b't' => value.push('\t'),
                    b'r' => value.push('\r'),
                    b'\\' => value.push('\\'),
                    b'\'' => value.push('\''),
                    b'"' => value.push('"'),
                    _ => {
                        value.push('\\');
                        value.push(escaped as char);
                    }
                }
                self.pos += 1;
            } else {
                // Decode UTF-8: multi-byte characters (emojis, accented chars, etc.)
                let remaining = &self.source[self.pos..];
                if let Some(c) = std::str::from_utf8(remaining)
                    .ok()
                    .and_then(|s| s.chars().next())
                {
                    value.push(c);
                    self.pos += c.len_utf8();
                } else {
                    // Invalid UTF-8 byte — push replacement char and advance
                    value.push(ch as char);
                    self.pos += 1;
                }
            }
        }

        Err(FormulaError::new(
            ErrorKind::UnterminatedString,
            Some(Span { start, end: self.pos }),
        ))
    }

    fn lex_ident(&mut self) {
        let start = self.pos;
        while self.pos < self.source.len() {
            let ch = self.source[self.pos];
            if ch.is_ascii_alphanumeric() || ch == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }

        let text = std::str::from_utf8(&self.source[start..self.pos]).unwrap_or("");
        let kind = match text {
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            _ => TokenKind::Ident(text.to_string()),
        };
        self.push_token(kind, start, self.pos);
    }

    fn push_token(&mut self, kind: TokenKind, start: usize, end: usize) {
        self.tokens.push(Token {
            kind,
            span: Span { start, end },
        });
    }
}
