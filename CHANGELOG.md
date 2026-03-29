# Changelog

All notable changes are documented here.

## [2026-03-28]

- **09:19** `chore(deps)` add wasm-bindgen, serde and serde_json to Cargo.toml dependencies
- **09:26** `chore(deps)` pin wasm-bindgen to 0.2.92 for stable ABI compatibility
- **09:41** `feat(types)` add Date and DateRange variants to FormulaValue
- **09:49** `feat(types)` add Array(Vec<FormulaValue>) variant for aggregate functions
- **09:57** `feat(types)` add Expr AST node enum (Literal, Ident, BinOp, UnaryOp, Call, Prop)
- **10:05** `feat(types)` add BinOp and UnaryOp enums with all operator variants
- **10:12** `feat(types)` impl Display for FormulaValue with Notion-style rendering
- **10:28** `feat(error)` add LexError, ParseError, CompileError and RuntimeError variants
- **10:36** `feat(error)` impl From<FormulaError> for JsValue for WASM boundary crossing
- **10:51** `feat(lexer)` tokenize integer and floating-point number literals
- **10:59** `feat(lexer)` add double-quoted string literal with backslash escape sequences
- **11:07** `feat(lexer)` tokenize identifiers and map reserved words (and, or, not, if, true, false, null)
- **11:14** `feat(lexer)` add all operator tokens: +, -, *, /, %, ==, !=, <, >, <=, >=
- **11:22** `feat(lexer)` add punctuation tokens: (, ), [, ], ,, . and prop-access bracket
- **11:30** `feat(lexer)` track line and column for every token for precise error spans
- **11:38** `feat(lexer)` skip whitespace and single-line // comments between tokens
- **11:53** `feat(parser)` parse number, string, bool and null literal expressions
- **12:01** `feat(parser)` implement precedence table and parse_expr entry point
- **12:09** `feat(parser)` add binary operator parsing with left-associativity
- **12:16** `feat(parser)` add unary minus and logical not prefix operators
- **12:24** `feat(parser)` parse parenthesised sub-expressions and grouping
- **12:32** `feat(parser)` parse function calls: identifier followed by argument list in ()
