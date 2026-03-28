// ═══════════════════════════════════════════════════════════════════════════════
// FORMULA ENGINE — VALUE TYPE SYSTEM
// ═══════════════════════════════════════════════════════════════════════════════
// Tagged union representing all possible runtime values in formulas.
// Optimized for the common case (numbers) while supporting rich types.

use serde::{Deserialize, Serialize};
use std::fmt;

// Closure support — closures are encoded as a special Text value
const CLOSURE_PREFIX: &str = "__closure:";

/// Closure data (matches vm::Closure but defined here to avoid circular deps)
#[derive(Debug, Clone)]
pub struct ClosureData {
    pub body_idx: usize,
    pub captured_locals: Vec<(usize, Value)>,
}

/// Rich text style info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RichTextSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub code: bool,
    pub color: Option<String>,
    pub url: Option<String>,
}

/// The core Value type — every formula evaluates to one of these.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Value {
    /// IEEE 754 double — the number type (like JS)
    Number(f64),
    /// UTF-8 string
    Text(String),
    /// Boolean
    Boolean(bool),
    /// Date as milliseconds since Unix epoch (UTC)
    Date(i64),
    /// Date range (start_ms, end_ms)
    DateRange(i64, i64),
    /// Ordered list of values
    Array(Vec<Value>),
    /// Rich text with formatting
    RichText(Vec<RichTextSpan>),
    /// Null / empty / no value
    Empty,
}

impl Value {
    // ─── Type coercion helpers ─────────────────────────────────

    /// Coerce to f64 (returns None if not convertible)
    pub fn to_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
            Value::Text(s) => s.parse::<f64>().ok(),
            Value::Date(ms) => Some(*ms as f64),
            Value::Empty => None,
            _ => None,
        }
    }

    /// Coerce to string
    pub fn to_text(&self) -> String {
        match self {
            Value::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Text(s) => s.clone(),
            Value::Boolean(b) => if *b { "true".into() } else { "false".into() },
            Value::Date(ms) => format_date_ms(*ms),
            Value::DateRange(s, e) => format!("{} → {}", format_date_ms(*s), format_date_ms(*e)),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_text()).collect();
                items.join(", ")
            }
            Value::RichText(spans) => spans.iter().map(|s| s.text.as_str()).collect(),
            Value::Empty => String::new(),
        }
    }

    /// Coerce to bool (Notion-like: 0/empty/false→false, everything else→true)
    pub fn to_bool(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::Text(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Empty => false,
            _ => true,
        }
    }

    /// Check if value is empty/null
    pub fn is_empty(&self) -> bool {
        match self {
            Value::Empty => true,
            Value::Text(s) => s.is_empty(),
            Value::Array(a) => a.is_empty(),
            _ => false,
        }
    }

    /// Get type name for error messages
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::Text(_) => "text",
            Value::Boolean(_) => "boolean",
            Value::Date(_) => "date",
            Value::DateRange(_, _) => "dateRange",
            Value::Array(_) => "array",
            Value::RichText(_) => "richText",
            Value::Empty => "empty",
        }
    }

    /// Create a closure marker value (encodes closure info as a Text value)
    pub fn closure_marker(closure: ClosureData) -> Value {
        let mut s = format!("{}{}", CLOSURE_PREFIX, closure.body_idx);
        for (slot, _val) in &closure.captured_locals {
            s.push_str(&format!(":{}", slot));
        }
        CLOSURE_STORE.with(|store| {
            store.borrow_mut().insert(s.clone(), closure);
        });
        Value::Text(s)
    }

    /// Try to extract a ClosureData from a value
    pub fn as_closure(val: &Value) -> Option<ClosureData> {
        if let Value::Text(s) = val {
            if s.starts_with(CLOSURE_PREFIX) {
                return CLOSURE_STORE.with(|store| {
                    store.borrow().get(s).cloned()
                });
            }
        }
        None
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => {
                if a.is_nan() && b.is_nan() { return true; }
                (a - b).abs() < f64::EPSILON
            }
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::DateRange(a1, a2), Value::DateRange(b1, b2)) => a1 == b1 && a2 == b2,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Empty, Value::Empty) => true,
            _ => false,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a.partial_cmp(b),
            (Value::Text(a), Value::Text(b)) => Some(a.cmp(b)),
            (Value::Boolean(a), Value::Boolean(b)) => Some(a.cmp(b)),
            (Value::Date(a), Value::Date(b)) => Some(a.cmp(b)),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_text())
    }
}

// ─── Thread-local closure storage ─────────────────────────────

use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static CLOSURE_STORE: RefCell<HashMap<String, ClosureData>> = RefCell::new(HashMap::new());
}

/// Clear closure storage (call between evaluations to prevent leaks)
pub fn clear_closures() {
    CLOSURE_STORE.with(|store| store.borrow_mut().clear());
}

// ─── Date formatting helper ──────────────────────────────────

pub fn format_date_ms(ms: i64) -> String {
    let secs = ms / 1000;
    let days_since_epoch = secs / 86400;
    // Simple date calculation (no external crate needed)
    let (y, m, d) = days_to_ymd(days_since_epoch);
    let hh = ((secs % 86400) / 3600) as u32;
    let mm = ((secs % 3600) / 60) as u32;
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun",
        "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month_name = if m >= 1 && m <= 12 {
        months[(m - 1) as usize]
    } else {
        "???"
    };
    if hh == 0 && mm == 0 {
        format!("{} {}, {}", month_name, d, y)
    } else {
        format!("{} {}, {} {:02}:{:02}", month_name, d, y, hh, mm)
    }
}

/// Convert days since Unix epoch to (year, month, day)
pub fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64 + era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Convert (year, month, day) to days since Unix epoch
pub fn ymd_to_days(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y as i64 - 1 } else { y as i64 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let m_adj = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * m_adj + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe as i64 - 719468
}

/// Break ms timestamp into date components (year, month, day)
pub fn ms_to_components(ms: i64) -> (i32, u32, u32) {
    let total_secs = if ms >= 0 { ms / 1000 } else { (ms - 999) / 1000 };
    let day_secs = ((total_secs % 86400) + 86400) % 86400;
    let days = (total_secs - day_secs) / 86400;
    days_to_ymd(days)
}

/// Break ms timestamp into full components (year, month, day, hour, min, sec, millis)
pub fn ms_to_full_components(ms: i64) -> (i32, u32, u32, u32, u32, u32, u32) {
    let total_secs = if ms >= 0 { ms / 1000 } else { (ms - 999) / 1000 };
    let millis = ((ms % 1000) + 1000) as u32 % 1000;
    let day_secs = ((total_secs % 86400) + 86400) % 86400;
    let days = (total_secs - day_secs) / 86400;
    let (y, mo, d) = days_to_ymd(days);
    let h = (day_secs / 3600) as u32;
    let m = ((day_secs % 3600) / 60) as u32;
    let s = (day_secs % 60) as u32;
    (y, mo, d, h, m, s, millis)
}

/// Build ms timestamp from components
pub fn components_to_ms(y: i32, mo: u32, d: u32, h: u32, m: u32, s: u32) -> i64 {
    let days = ymd_to_days(y, mo, d);
    days * 86400_000 + (h as i64) * 3600_000 + (m as i64) * 60_000 + (s as i64) * 1000
}
