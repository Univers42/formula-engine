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
