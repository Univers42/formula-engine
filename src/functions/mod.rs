// ═══════════════════════════════════════════════════════════════════════════════
// FORMULA ENGINE — BUILT-IN FUNCTIONS (90+ Notion-compatible functions)
// ═══════════════════════════════════════════════════════════════════════════════

pub mod math;
pub mod text;
pub mod date;
pub mod array;
pub mod logic;

use crate::compiler::Chunk;
use crate::error::{ErrorKind, FormulaError};
use crate::types::Value;
use crate::vm::{EvalContext, VM};

/// Dispatch a built-in function call by name
pub fn call_builtin(
    name: &str,
    args: Vec<Value>,
    chunk: &Chunk,
    ctx: &EvalContext,
    vm: &mut VM,
    call_depth: usize,
) -> crate::error::Result<Value> {
    match name {
        // ─── Math (26 functions) ──────────────────────────────
        "abs" => math::abs(args),
        "ceil" => math::ceil(args),
        "floor" => math::floor(args),
        "round" => math::round(args),
        "max" => math::max(args),
        "min" => math::min(args),
        "sqrt" => math::sqrt(args),
        "cbrt" => math::cbrt(args),
        "pow" => math::pow(args),
        "exp" => math::exp(args),
        "ln" => math::ln(args),
        "log10" => math::log10(args),
        "log2" => math::log2(args),
        "sign" => math::sign(args),
        "pi" => math::pi(args),
        "e" => math::euler(args),
        "mod" => math::modulo(args),
        "unaryMinus" => math::unary_minus(args),
        "unaryPlus" => math::unary_plus(args),
        "toNumber" => math::to_number(args),
        "sin" => math::sin(args),
        "cos" => math::cos(args),
        "tan" => math::tan(args),
        "asin" => math::asin(args),
        "acos" => math::acos(args),
        "atan" => math::atan(args),

        // ─── Text (17 functions) ──────────────────────────────
        "concat" => text::concat(args),
        "join" => text::join(args),
        "substring" | "slice" => text::substring(args),
        "length" => text::length(args),
        "contains" => text::contains(args),
        "replace" => text::replace(args),
        "replaceAll" => text::replace_all(args),
        "lower" => text::lower(args),
        "upper" => text::upper(args),
        "trim" => text::trim(args),
        "test" => text::test_regex(args),
        "match" => text::match_regex(args),
        "format" => text::format_val(args),
        "split" => text::split(args),
        "padStart" | "lpad" => text::pad_start(args),
        "padEnd" | "rpad" => text::pad_end(args),
        "repeat" | "repeat_text" => text::repeat(args),
        "link" => text::link(args),
        "style" => text::style(args),
        "unstyle" => text::unstyle(args),
        "encode" => text::encode(args),
        "id" => text::id_func(args),

        // ─── Date (19 functions) ──────────────────────────────
        "now" => date::now(args),
        "today" => date::today(args),
        "timestamp" => date::timestamp(args),
        "fromTimestamp" => date::from_timestamp(args),
        "dateAdd" => date::date_add(args),
        "dateSubtract" => date::date_subtract(args),
        "dateBetween" => date::date_between(args),
        "dateRange" => date::date_range(args),
        "dateStart" => date::date_start(args),
        "dateEnd" => date::date_end(args),
        "date" => date::date_construct(args),
        "year" => date::year(args),
        "month" => date::month(args),
        "day" => date::day(args),
        "hour" => date::hour(args),
        "minute" => date::minute(args),
        "formatDate" => date::format_date(args),
        "parseDate" => date::parse_date(args),
        "weekday" | "dayOfWeek" => date::weekday(args),

        // ─── Array (17 functions) ─────────────────────────────
        "map" => array::map(args, chunk, ctx, vm, call_depth),
        "filter" => array::filter(args, chunk, ctx, vm, call_depth),
        "find" => array::find(args, chunk, ctx, vm, call_depth),
        "findIndex" => array::find_index(args, chunk, ctx, vm, call_depth),
        "some" => array::some(args, chunk, ctx, vm, call_depth),
        "every" => array::every(args, chunk, ctx, vm, call_depth),
        "reduce" | "fold" => array::reduce(args, chunk, ctx, vm, call_depth),
        "flat" => array::flat(args),
        "sort" => array::sort(args),
        "reverse" => array::reverse(args),
        "unique" => array::unique(args),
        "at" | "index" => array::at(args),
        "first" => array::first(args),
        "last" => array::last(args),
        "sum" => array::sum(args),
        "count" | "size" => array::count(args),
        "average" | "mean" => array::average(args),
        "median" => array::median(args),
        "__array_construct" => array::construct(args),
        "includes" => array::includes(args),

        // ─── Logic / General (12+ functions) ──────────────────
        "empty" => logic::empty(args),
        "not" => logic::not(args),
        "and" => logic::and(args),
        "or" => logic::or(args),
        "equal" => logic::equal(args),
        "unequal" => logic::unequal(args),
        "larger" => logic::larger(args),
        "smaller" => logic::smaller(args),
        "largerEq" => logic::larger_eq(args),
        "smallerEq" => logic::smaller_eq(args),
        "toBoolean" => logic::to_boolean(args),
        "toString" => logic::to_string(args),
        "typeof" => logic::type_of(args),

        // Dynamic prop resolution (runtime)
        "prop" => {
            if args.len() == 1 {
                let name = args[0].to_text();
                Ok(ctx.properties.get(&name).cloned().unwrap_or(Value::Empty))
            } else {
                Err(FormulaError::new(
                    ErrorKind::ArityMismatch("prop".into(), "1".into(), args.len()),
                    None,
                ))
            }
        }

        _ => Err(FormulaError::new(
            ErrorKind::FunctionNotFound(name.to_string()),
            None,
        )),
    }
}
