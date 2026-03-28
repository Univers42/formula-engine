// ═══════════════════════════════════════════════════════════════════════════════
// DATE FUNCTIONS (19 functions)
// ═══════════════════════════════════════════════════════════════════════════════
// All dates are represented as i64 milliseconds since Unix epoch.
// Pure Rust implementation — no chrono crate needed.

use crate::error::{ErrorKind, FormulaError};
use crate::types::{self, Value};

const MS_PER_SECOND: i64 = 1_000;
const MS_PER_MINUTE: i64 = 60_000;
const MS_PER_HOUR: i64 = 3_600_000;
const MS_PER_DAY: i64 = 86_400_000;

fn expect_date(args: &[Value], idx: usize, fname: &str) -> crate::error::Result<i64> {
    match args.get(idx) {
        Some(Value::Date(ms)) => Ok(*ms),
        Some(Value::Number(n)) => Ok(*n as i64),
        Some(Value::Text(s)) => {
            // Auto-coerce ISO date strings (e.g. "2025-01-15" or "2025-01-15T10:30:00")
            let s = s.trim();
            if s.len() >= 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
                let year: i32 = s[0..4].parse().unwrap_or(2000);
                let month: u32 = s[5..7].parse().unwrap_or(1);
                let day: u32 = s[8..10].parse().unwrap_or(1);
                let mut hour = 0u32;
                let mut minute = 0u32;
                let mut second = 0u32;
                if s.len() >= 19 && (s.as_bytes()[10] == b'T' || s.as_bytes()[10] == b' ') {
                    hour = s[11..13].parse().unwrap_or(0);
                    minute = s[14..16].parse().unwrap_or(0);
                    second = s[17..19].parse().unwrap_or(0);
                }
                Ok(types::components_to_ms(year, month, day, hour, minute, second))
            } else if let Ok(n) = s.parse::<f64>() {
                Ok(n as i64)
            } else {
                Err(FormulaError::new(
                    ErrorKind::TypeError(format!("{}: expected date at argument {}, got unparseable text", fname, idx + 1)),
                    None,
                ))
            }
        }
        Some(Value::Empty) => Err(FormulaError::new(
            ErrorKind::TypeError(format!("{}: expected date at argument {}", fname, idx + 1)),
            None,
        )),
        _ => Err(FormulaError::new(
            ErrorKind::TypeError(format!("{}: expected date at argument {}", fname, idx + 1)),
            None,
        )),
    }
}

/// now() → current timestamp in ms (returns a fixed value in WASM since no real clock)
pub fn now(_args: Vec<Value>) -> crate::error::Result<Value> {
    // In WASM we'll get the real time from JS via js_sys::Date::now()
    #[cfg(target_arch = "wasm32")]
    {
        Ok(Value::Date(js_sys::Date::now() as i64))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(Value::Date(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
        ))
    }
}

/// today() → midnight of current day
pub fn today(_args: Vec<Value>) -> crate::error::Result<Value> {
    let now_ms;
    #[cfg(target_arch = "wasm32")]
    {
        now_ms = js_sys::Date::now() as i64;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
    }
    // Truncate to start of day
    let today_ms = (now_ms / MS_PER_DAY) * MS_PER_DAY;
    Ok(Value::Date(today_ms))
}

/// timestamp(date) → ms since epoch as number
pub fn timestamp(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let ms = expect_date(&args, 0, "timestamp")?;
    Ok(Value::Number(ms as f64))
}

/// fromTimestamp(number) → date
pub fn from_timestamp(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let n = args[0].to_number().ok_or_else(|| {
        FormulaError::new(
            ErrorKind::TypeError("fromTimestamp: expected number".into()),
            None,
        )
    })?;
    Ok(Value::Date(n as i64))
}

/// dateAdd(date, amount, unit) → date
/// units: "years", "months", "weeks", "days", "hours", "minutes", "seconds"
pub fn date_add(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 3 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("dateAdd".into(), "3".into(), args.len()),
            None,
        ));
    }
    if args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let ms = expect_date(&args, 0, "dateAdd")?;
    let amount = args[1].to_number().unwrap_or(0.0) as i64;
    let unit = args[2].to_text().to_lowercase();

    Ok(Value::Date(add_to_date(ms, amount, &unit)))
}

/// dateSubtract(date, amount, unit) → date
pub fn date_subtract(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 3 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("dateSubtract".into(), "3".into(), args.len()),
            None,
        ));
    }
    if args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let ms = expect_date(&args, 0, "dateSubtract")?;
    let amount = args[1].to_number().unwrap_or(0.0) as i64;
    let unit = args[2].to_text().to_lowercase();

    Ok(Value::Date(add_to_date(ms, -amount, &unit)))
}

/// dateBetween(date1, date2, unit) → number
pub fn date_between(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 3 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("dateBetween".into(), "3".into(), args.len()),
            None,
        ));
    }
    if args[0].is_empty() || args[1].is_empty() {
        return Ok(Value::Empty);
    }
    let d1 = expect_date(&args, 0, "dateBetween")?;
    let d2 = expect_date(&args, 1, "dateBetween")?;
    let unit = args[2].to_text().to_lowercase();

    let diff = d1 - d2;
    let result = match unit.as_str() {
        "years" => {
            let (y1, _, _) = types::ms_to_components(d1);
            let (y2, _, _) = types::ms_to_components(d2);
            (y1 - y2) as f64
        }
        "months" => {
            let (y1, m1, _) = types::ms_to_components(d1);
            let (y2, m2, _) = types::ms_to_components(d2);
            ((y1 - y2) * 12 + (m1 as i32 - m2 as i32)) as f64
        }
        "weeks" => (diff / (MS_PER_DAY * 7)) as f64,
        "days" => (diff / MS_PER_DAY) as f64,
        "hours" => (diff / MS_PER_HOUR) as f64,
        "minutes" => (diff / MS_PER_MINUTE) as f64,
        "seconds" => (diff / MS_PER_SECOND) as f64,
        "milliseconds" => diff as f64,
        _ => (diff / MS_PER_DAY) as f64,
    };

    Ok(Value::Number(result))
}

/// dateRange(start, end) → DateRange
pub fn date_range(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 2 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("dateRange".into(), "2".into(), args.len()),
            None,
        ));
    }
    let start = expect_date(&args, 0, "dateRange")?;
    let end = expect_date(&args, 1, "dateRange")?;
    Ok(Value::DateRange(start, end))
}

/// dateStart(dateRange) → date
pub fn date_start(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Empty);
    }
    match &args[0] {
        Value::DateRange(start, _) => Ok(Value::Date(*start)),
        Value::Date(d) => Ok(Value::Date(*d)),
        _ => Ok(Value::Empty),
    }
}

/// dateEnd(dateRange) → date
pub fn date_end(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() {
        return Ok(Value::Empty);
    }
    match &args[0] {
        Value::DateRange(_, end) => Ok(Value::Date(*end)),
        Value::Date(d) => Ok(Value::Date(*d)),
        _ => Ok(Value::Empty),
    }
}

/// date(year, month, day) → date
pub fn date_construct(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.len() < 3 {
        return Err(FormulaError::new(
            ErrorKind::ArityMismatch("date".into(), "3-6".into(), args.len()),
            None,
        ));
    }
    let year = args[0].to_number().unwrap_or(2000.0) as i32;
    let month = args[1].to_number().unwrap_or(1.0) as u32;
    let day = args[2].to_number().unwrap_or(1.0) as u32;
    let hour = args.get(3).and_then(|v| v.to_number()).unwrap_or(0.0) as u32;
    let min = args.get(4).and_then(|v| v.to_number()).unwrap_or(0.0) as u32;
    let sec = args.get(5).and_then(|v| v.to_number()).unwrap_or(0.0) as u32;

    let ms = types::components_to_ms(year, month, day, hour, min, sec);
    Ok(Value::Date(ms))
}

/// year(date) → number
pub fn year(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let ms = expect_date(&args, 0, "year")?;
    let (y, _, _) = types::ms_to_components(ms);
    Ok(Value::Number(y as f64))
}

/// month(date) → number (1-12)
pub fn month(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let ms = expect_date(&args, 0, "month")?;
    let (_, m, _) = types::ms_to_components(ms);
    Ok(Value::Number(m as f64))
}

/// day(date) → number (1-31)
pub fn day(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let ms = expect_date(&args, 0, "day")?;
    let (_, _, d) = types::ms_to_components(ms);
    Ok(Value::Number(d as f64))
}

/// hour(date) → number (0-23)
pub fn hour(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let ms = expect_date(&args, 0, "hour")?;
    // Extract hour from ms
    let day_ms = ms.rem_euclid(MS_PER_DAY);
    let h = day_ms / MS_PER_HOUR;
    Ok(Value::Number(h as f64))
}

/// minute(date) → number (0-59)
pub fn minute(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let ms = expect_date(&args, 0, "minute")?;
    let day_ms = ms.rem_euclid(MS_PER_DAY);
    let m = (day_ms % MS_PER_HOUR) / MS_PER_MINUTE;
    Ok(Value::Number(m as f64))
}

/// formatDate(date, formatStr) → text
pub fn format_date(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let ms = expect_date(&args, 0, "formatDate")?;
    let fmt_str = args.get(1).map(|v| v.to_text()).unwrap_or_default();

    if fmt_str.is_empty() {
        return Ok(Value::Text(types::format_date_ms(ms)));
    }

    let (year, month, day) = types::ms_to_components(ms);
    let day_ms = ms.rem_euclid(MS_PER_DAY);
    let hour = (day_ms / MS_PER_HOUR) as u32;
    let minute = ((day_ms % MS_PER_HOUR) / MS_PER_MINUTE) as u32;
    let second = ((day_ms % MS_PER_MINUTE) / MS_PER_SECOND) as u32;

    let result = fmt_str
        .replace("YYYY", &format!("{:04}", year))
        .replace("YY", &format!("{:02}", year % 100))
        .replace("MM", &format!("{:02}", month))
        .replace("DD", &format!("{:02}", day))
        .replace("HH", &format!("{:02}", hour))
        .replace("mm", &format!("{:02}", minute))
        .replace("ss", &format!("{:02}", second));

    Ok(Value::Text(result))
}

/// parseDate(text) → date
pub fn parse_date(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let s = args[0].to_text();
    let s = s.trim();

    // Try ISO format: YYYY-MM-DD or YYYY-MM-DDTHH:mm:ss
    if s.len() >= 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        let year: i32 = s[0..4].parse().unwrap_or(2000);
        let month: u32 = s[5..7].parse().unwrap_or(1);
        let day: u32 = s[8..10].parse().unwrap_or(1);

        let mut hour = 0u32;
        let mut minute = 0u32;
        let mut second = 0u32;

        if s.len() >= 19 && (s.as_bytes()[10] == b'T' || s.as_bytes()[10] == b' ') {
            hour = s[11..13].parse().unwrap_or(0);
            minute = s[14..16].parse().unwrap_or(0);
            second = s[17..19].parse().unwrap_or(0);
        }

        let ms = types::components_to_ms(year, month, day, hour, minute, second);
        return Ok(Value::Date(ms));
    }

    // Fallback: try as timestamp
    if let Ok(n) = s.parse::<f64>() {
        return Ok(Value::Date(n as i64));
    }

    Ok(Value::Empty)
}

/// weekday(date) → number (0=Sunday, 1=Monday, ..., 6=Saturday)
pub fn weekday(args: Vec<Value>) -> crate::error::Result<Value> {
    if args.is_empty() || args[0].is_empty() {
        return Ok(Value::Empty);
    }
    let ms = expect_date(&args, 0, "weekday")?;
    // Days since epoch (1970-01-01 was a Thursday = 4)
    let days = ms.div_euclid(MS_PER_DAY);
    let dow = ((days + 4) % 7 + 7) % 7; // 0=Sunday
    Ok(Value::Number(dow as f64))
}

// ─── Date arithmetic helper ───────────────────────────────────────────────────

fn add_to_date(ms: i64, amount: i64, unit: &str) -> i64 {
    match unit {
        "years" => {
            let (y, m, d) = types::ms_to_components(ms);
            let day_ms = ms.rem_euclid(MS_PER_DAY);
            let new_y = y + amount as i32;
            types::components_to_ms(new_y, m, d, 0, 0, 0) + day_ms
        }
        "months" => {
            let (y, m, d) = types::ms_to_components(ms);
            let day_ms = ms.rem_euclid(MS_PER_DAY);
            let total_months = y as i64 * 12 + (m as i64 - 1) + amount;
            let new_y = (total_months.div_euclid(12)) as i32;
            let new_m = (total_months.rem_euclid(12) + 1) as u32;
            types::components_to_ms(new_y, new_m, d, 0, 0, 0) + day_ms
        }
        "weeks" => ms + amount * MS_PER_DAY * 7,
        "days" => ms + amount * MS_PER_DAY,
        "hours" => ms + amount * MS_PER_HOUR,
        "minutes" => ms + amount * MS_PER_MINUTE,
        "seconds" => ms + amount * MS_PER_SECOND,
        "milliseconds" => ms + amount,
        _ => ms + amount * MS_PER_DAY,
    }
}
