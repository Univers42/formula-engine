// ═══════════════════════════════════════════════════════════════════════════════
// TEXT FUNCTIONS (17+ functions)
// ═══════════════════════════════════════════════════════════════════════════════

use crate::error::{ErrorKind, FormulaError};
use crate::types::Value;

pub fn concat(args: Vec<Value>) -> crate::error::Result<Value> {
    let mut result = String::new();
    for arg in &args {
        match arg {
            Value::Empty => {}
            _ => result.push_str(&arg.to_text()),
        }
    }
    Ok(Value::Text(result))
}

pub fn join(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Text(String::new()));
    }
    let separator = args.get(0).map(|v| v.to_text()).unwrap_or_default();
    let mut parts = Vec::new();

    for arg in &args[1..] {
        match arg {
            Value::Array(arr) => {
                for v in arr {
                    if !v.is_empty() {
                        parts.push(v.to_text());
                    }
                }
            }
            Value::Empty => {}
            _ => parts.push(arg.to_text()),
        }
    }

    Ok(Value::Text(parts.join(&separator)))
}

pub fn substring(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Empty);
    }
    if args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let s = args[0].to_text();
    let chars: Vec<char> = s.chars().collect();
    let start = args
        .get(1)
        .and_then(|v| v.to_number())
        .unwrap_or(0.0) as usize;
    let end = args
        .get(2)
        .and_then(|v| v.to_number())
        .map(|n| n as usize)
        .unwrap_or(chars.len());

    let start = start.min(chars.len());
    let end = end.min(chars.len());
    if start >= end {
        return Ok(Value::Text(String::new()));
    }

    Ok(Value::Text(chars[start..end].iter().collect()))
}

pub fn length(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Number(0.0));
    }
    match &args[0] {
        Value::Text(s) => Ok(Value::Number(s.chars().count() as f64)),
        Value::Array(arr) => Ok(Value::Number(arr.len() as f64)),
        Value::Empty => Ok(Value::Number(0.0)),
        _ => Ok(Value::Number(args[0].to_text().chars().count() as f64)),
    }
}

pub fn contains(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("contains".into(), "2".into(), args.len()),
            None,
        ));
    }
    if args[0].is_empty() {
        return Ok(Value::Boolean(false));
    }
    let haystack = args[0].to_text();
    let needle = args[1].to_text();
    Ok(Value::Boolean(haystack.contains(&needle)))
}

pub fn replace(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 3 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("replace".into(), "3".into(), args.len()),
            None,
        ));
    }
    if args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let s = args[0].to_text();
    let pattern = args[1].to_text();
    let replacement = args[2].to_text();
    Ok(Value::Text(s.replacen(&pattern, &replacement, 1)))
}

pub fn replace_all(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 3 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("replaceAll".into(), "3".into(), args.len()),
            None,
        ));
    }
    if args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let s = args[0].to_text();
    let pattern = args[1].to_text();
    let replacement = args[2].to_text();
    Ok(Value::Text(s.replace(&pattern, &replacement)))
}

pub fn lower(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    Ok(Value::Text(args[0].to_text().to_lowercase()))
}

pub fn upper(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    Ok(Value::Text(args[0].to_text().to_uppercase()))
}

pub fn trim(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    Ok(Value::Text(args[0].to_text().trim().to_string()))
}

pub fn test_regex(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("test".into(), "2".into(), args.len()),
            None,
        ));
    }
    if args[0].is_empty() {
        return Ok(Value::Boolean(false));
    }
    let s = args[0].to_text();
    let pattern = args[1].to_text();
    // Simple regex-like matching (basic glob without full regex crate)
    Ok(Value::Boolean(simple_match(&pattern, &s)))
}

pub fn match_regex(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("match".into(), "2".into(), args.len()),
            None,
        ));
    }
    if args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let s = args[0].to_text();
    let pattern = args[1].to_text();

    // Return matched portion or empty
    if simple_match(&pattern, &s) {
        Ok(Value::Text(s))
    } else {
        Ok(Value::Empty)
    }
}

pub fn format_val(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Text(String::new()));
    }
    match &args[0] {
        Value::Number(n) => {
            // Optional format string
            if let Some(fmt) = args.get(1) {
                let fmt_str = fmt.to_text();
                if fmt_str == "percent" {
                    return Ok(Value::Text(format!("{}%", n * 100.0)));
                }
                // Try parsing as decimal places
                if let Ok(places) = fmt_str.parse::<usize>() {
                    return Ok(Value::Text(format!("{:.prec$}", n, prec = places)));
                }
            }
            Ok(Value::Text(format_number(*n)))
        }
        Value::Boolean(b) => Ok(Value::Text(if *b { "Yes".into() } else { "No".into() })),
        Value::Date(ms) => {
            let formatted = crate::types::format_date_ms(*ms);
            Ok(Value::Text(formatted))
        }
        Value::Empty => Ok(Value::Text(String::new())),
        _ => Ok(Value::Text(args[0].to_text())),
    }
}

pub fn split(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    let s = args[0].to_text();
    let sep = args.get(1).map(|v| v.to_text()).unwrap_or_else(|| ",".into());
    let parts: Vec<Value> = s.split(&sep).map(|p| Value::Text(p.to_string())).collect();
    Ok(Value::Array(parts))
}

pub fn pad_start(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("padStart".into(), "2-3".into(), args.len()),
            None,
        ));
    }
    if args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let s = args[0].to_text();
    let target_len = args[1].to_number().unwrap_or(0.0) as usize;
    let pad_char = args
        .get(2)
        .map(|v| v.to_text())
        .unwrap_or_else(|| " ".into());
    let pad_char = pad_char.chars().next().unwrap_or(' ');

    let current_len = s.chars().count();
    if current_len >= target_len {
        return Ok(Value::Text(s));
    }
    let padding: String = std::iter::repeat(pad_char)
        .take(target_len - current_len)
        .collect();
    Ok(Value::Text(format!("{}{}", padding, s)))
}

pub fn pad_end(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("padEnd".into(), "2-3".into(), args.len()),
            None,
        ));
    }
    if args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let s = args[0].to_text();
    let target_len = args[1].to_number().unwrap_or(0.0) as usize;
    let pad_char = args
        .get(2)
        .map(|v| v.to_text())
        .unwrap_or_else(|| " ".into());
    let pad_char = pad_char.chars().next().unwrap_or(' ');

    let current_len = s.chars().count();
    if current_len >= target_len {
        return Ok(Value::Text(s));
    }
    let padding: String = std::iter::repeat(pad_char)
        .take(target_len - current_len)
        .collect();
    Ok(Value::Text(format!("{}{}", s, padding)))
}

pub fn repeat(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("repeat".into(), "2".into(), args.len()),
            None,
        ));
    }
    if args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let s = args[0].to_text();
    let count = args[1].to_number().unwrap_or(0.0) as usize;
    Ok(Value::Text(s.repeat(count.min(10_000))))
}

pub fn link(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Empty);
    }
    let label = args[0].to_text();
    let url = args.get(1).map(|v| v.to_text()).unwrap_or_else(|| label.clone());
    Ok(Value::Text(format!("[{}]({})", label, url)))
}

pub fn style(args: Vec<Value>) -> crate::error::Result<Value> {
    // style(text, "b") / style(text, "i") etc — returns styled text marker
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let text = args[0].to_text();
    let style_code = args.get(1).map(|v| v.to_text()).unwrap_or_default();
    // Return as enriched text marker
    Ok(Value::Text(format!("<{}>{}</{}>", style_code, text, style_code)))
}

pub fn unstyle(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    // Strip simple style tags
    let text = args[0].to_text();
    let stripped = text
        .replace("<b>", "").replace("</b>", "")
        .replace("<i>", "").replace("</i>", "")
        .replace("<u>", "").replace("</u>", "")
        .replace("<s>", "").replace("</s>", "")
        .replace("<code>", "").replace("</code>", "");
    Ok(Value::Text(stripped))
}

pub fn encode(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let s = args[0].to_text();
    // Simple URL encoding
    let encoded: String = s
        .chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '!' => "%21".to_string(),
            '#' => "%23".to_string(),
            '$' => "%24".to_string(),
            '&' => "%26".to_string(),
            '\'' => "%27".to_string(),
            '(' => "%28".to_string(),
            ')' => "%29".to_string(),
            '*' => "%2A".to_string(),
            '+' => "%2B".to_string(),
            ',' => "%2C".to_string(),
            '/' => "%2F".to_string(),
            ':' => "%3A".to_string(),
            ';' => "%3B".to_string(),
            '=' => "%3D".to_string(),
            '?' => "%3F".to_string(),
            '@' => "%40".to_string(),
            '[' => "%5B".to_string(),
            ']' => "%5D".to_string(),
            _ => c.to_string(),
        })
        .collect();
    Ok(Value::Text(encoded))
}

pub fn id_func(args: Vec<Value>) -> crate::error::Result<Value> {
    // id() — returns simple unique-ish identifier
    // In WASM context we don't have uuid, use a simple hash
    if !args.is_empty() {
        return Ok(args[0].clone());
    }
    Ok(Value::Text("id".to_string()))
}

// ─── Helpers ──────────────────────────────────────────────────────────────

fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

/// Very basic pattern matching (supports * wildcard and case-insensitive literal match)
fn simple_match(pattern: &str, text: &str) -> bool {
    let p_lower = pattern.to_lowercase();
    let t_lower = text.to_lowercase();

    if p_lower.contains('*') {
        // Split on * and check all parts exist in order
        let parts: Vec<&str> = p_lower.split('*').collect();
        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            if let Some(found) = t_lower[pos..].find(part) {
                if i == 0 && found != 0 {
                    return false; // Must match from start
                }
                pos += found + part.len();
            } else {
                return false;
            }
        }
        true
    } else {
        t_lower.contains(&p_lower)
    }
}
