//! DateTime functions
//!
//! PHP datetime functions implementation

use crate::engine::operators::{zval_get_bool, zval_get_long, zval_get_string};
use crate::engine::types::{PhpType, PhpValue, Val};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn time_now(_args: &[Val]) -> Result<Val, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?;
    Ok(Val::new(
        PhpValue::Long(now.as_secs() as i64),
        PhpType::Long,
    ))
}

pub fn microtime(args: &[Val]) -> Result<Val, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?;

    let get_as_float = args.first().map(zval_get_bool).unwrap_or(false);

    if get_as_float {
        let secs = now.as_secs() as f64;
        let micros = now.subsec_micros() as f64 / 1_000_000.0;
        Ok(Val::new(PhpValue::Double(secs + micros), PhpType::Double))
    } else {
        let secs = now.as_secs();
        let micros = now.subsec_micros();
        Ok(Val::new(
            PhpValue::String(Box::new(crate::engine::string::string_init(
                &format!("0.{:06} {}", micros, secs),
                false,
            ))),
            PhpType::String,
        ))
    }
}

pub fn date_format(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("date() expects at least 1 parameter".to_string());
    }

    let format = zval_get_string(&args[0]).as_str().to_string();
    let timestamp = if args.len() > 1 {
        zval_get_long(&args[1]) as u64
    } else {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("System time error: {}", e))?
            .as_secs()
    };

    let datetime = timestamp_to_datetime(timestamp);
    let formatted = format_datetime(&format, &datetime);

    Ok(Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(
            &formatted, false,
        ))),
        PhpType::String,
    ))
}

pub fn mktime(args: &[Val]) -> Result<Val, String> {
    if args.len() < 6 {
        return Err("mktime() expects at least 6 parameters".to_string());
    }

    let hour = zval_get_long(&args[0]);
    let minute = zval_get_long(&args[1]);
    let second = zval_get_long(&args[2]);
    let month = zval_get_long(&args[3]);
    let day = zval_get_long(&args[4]);
    let year = zval_get_long(&args[5]);

    let timestamp = datetime_to_timestamp(year, month, day, hour, minute, second);
    Ok(Val::new(PhpValue::Long(timestamp), PhpType::Long))
}

pub fn strtotime(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("strtotime() expects at least 1 parameter".to_string());
    }

    let time_str = zval_get_string(&args[0]).as_str().to_string();
    let base_time = if args.len() > 1 {
        zval_get_long(&args[1]) as u64
    } else {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("System time error: {}", e))?
            .as_secs()
    };

    let timestamp = parse_time_string(&time_str, base_time)?;
    Ok(Val::new(PhpValue::Long(timestamp as i64), PhpType::Long))
}

#[derive(Debug)]
pub struct DateTime {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    weekday: i64,
}

fn timestamp_to_datetime(timestamp: u64) -> DateTime {
    let days_since_epoch = timestamp / 86400;
    let seconds_today = timestamp % 86400;

    let hour = (seconds_today / 3600) as i64;
    let minute = ((seconds_today % 3600) / 60) as i64;
    let second = (seconds_today % 60) as i64;

    let mut year = 1970i64;
    let mut days_remaining = days_since_epoch as i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days_remaining < days_in_year {
            break;
        }
        days_remaining -= days_in_year;
        year += 1;
    }

    let mut month = 1i64;
    loop {
        let days_in_month = get_days_in_month(year, month);
        if days_remaining < days_in_month {
            break;
        }
        days_remaining -= days_in_month;
        month += 1;
    }

    let day = days_remaining + 1;
    let weekday = ((days_since_epoch + 4) % 7) as i64;

    DateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
        weekday,
    }
}

fn datetime_to_timestamp(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
) -> i64 {
    let mut days = 0i64;

    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    for m in 1..month {
        days += get_days_in_month(year, m);
    }

    days += day - 1;

    days * 86400 + hour * 3600 + minute * 60 + second
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn get_days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn format_datetime(format: &str, dt: &DateTime) -> String {
    format_datetime_tz(format, dt, 0, "UTC")
}

/// Format a DateTime with timezone offset (seconds) and name.
pub fn format_datetime_tz(format: &str, dt: &DateTime, tz_offset: i64, tz_name: &str) -> String {
    let mut result = String::new();

    for c in format.chars() {
        match c {
            'Y' => result.push_str(&format!("{:04}", dt.year)),
            'y' => result.push_str(&format!("{:02}", dt.year % 100)),
            'm' => result.push_str(&format!("{:02}", dt.month)),
            'n' => result.push_str(&dt.month.to_string()),
            'd' => result.push_str(&format!("{:02}", dt.day)),
            'j' => result.push_str(&dt.day.to_string()),
            'H' => result.push_str(&format!("{:02}", dt.hour)),
            'i' => result.push_str(&format!("{:02}", dt.minute)),
            's' => result.push_str(&format!("{:02}", dt.second)),
            'w' => result.push_str(&dt.weekday.to_string()),
            'D' => {
                let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                result.push_str(days[dt.weekday as usize]);
            }
            'M' => {
                let months = [
                    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
                    "Dec",
                ];
                result.push_str(months[(dt.month - 1) as usize]);
            }
            // Timezone format specifiers
            'e' => result.push_str(tz_name),
            'T' => result.push_str(tz_name),
            'O' => {
                let sign = if tz_offset < 0 { '-' } else { '+' };
                let off = tz_offset.abs();
                let hh = off / 3600;
                let mm = (off % 3600) / 60;
                result.push_str(&format!("{sign}{hh:02}{mm:02}"));
            }
            'P' => {
                let sign = if tz_offset < 0 { '-' } else { '+' };
                let off = tz_offset.abs();
                let hh = off / 3600;
                let mm = (off % 3600) / 60;
                result.push_str(&format!("{sign}{hh:02}:{mm:02}"));
            }
            'Z' => result.push_str(&tz_offset.to_string()),
            _ => result.push(c),
        }
    }

    result
}

/// Public wrapper for VM dispatch (avoids name clash with the struct).
pub fn timestamp_to_datetime_struct(timestamp: u64) -> DateTime {
    timestamp_to_datetime(timestamp)
}

/// Resolve a timezone identifier to its standard offset in seconds.
/// Handles "UTC", "Z", "+HH:MM"/"-HH:MM" offsets, and common named zones.
/// Returns 0 (UTC) for unknown zones.
pub fn timezone_offset(name: &str) -> i64 {
    if name.is_empty() || name == "UTC" || name == "Z" || name == "GMT" {
        return 0;
    }
    // Numeric offset: +0530, -08:00, +05:30, etc.
    if let Some(rest) = name.strip_prefix('+').or_else(|| name.strip_prefix('-')) {
        let sign = if name.starts_with('-') { -1 } else { 1 };
        let clean = rest.replace([':'], "");
        if clean.len() == 4 && clean.chars().all(|c| c.is_ascii_digit()) {
            let hh = clean[..2].parse::<i64>().unwrap_or(0);
            let mm = clean[2..].parse::<i64>().unwrap_or(0);
            return sign * (hh * 3600 + mm * 60);
        }
    }
    // Named zones — standard (non-DST) offsets
    match name {
        "America/New_York" | "America/Detroit" => -5 * 3600,
        "America/Chicago" | "America/Winnipeg" => -6 * 3600,
        "America/Denver" | "America/Phoenix" => -7 * 3600,
        "America/Los_Angeles" | "America/Vancouver" => -8 * 3600,
        "America/Anchorage" => -9 * 3600,
        "America/Sao_Paulo" | "America/Argentina/Buenos_Aires" => -3 * 3600,
        "Europe/London" | "Europe/Lisbon" | "Atlantic/Reykjavik" => 0,
        "Europe/Paris" | "Europe/Berlin" | "Europe/Madrid" | "Europe/Rome" | "Europe/Amsterdam" => 3600,
        "Europe/Athens" | "Europe/Helsinki" | "Europe/Bucharest" => 2 * 3600,
        "Europe/Moscow" => 3 * 3600,
        "Asia/Dubai" => 4 * 3600,
        "Asia/Karachi" => 5 * 3600,
        "Asia/Kolkata" | "Asia/Calcutta" => 5 * 3600 + 1800,
        "Asia/Dhaka" | "Asia/Almaty" => 6 * 3600,
        "Asia/Bangkok" | "Asia/Jakarta" | "Asia/Ho_Chi_Minh" => 7 * 3600,
        "Asia/Shanghai" | "Asia/Hong_Kong" | "Asia/Singapore" | "Asia/Taipei" | "Asia/Manila" => 8 * 3600,
        "Asia/Tokyo" | "Asia/Seoul" => 9 * 3600,
        "Australia/Sydney" => 10 * 3600,
        "Pacific/Auckland" => 12 * 3600,
        "Pacific/Honolulu" => -10 * 3600,
        _ => 0,
    }
}

/// Check if a timestamp falls within US DST (second Sunday of March to
/// first Sunday of November). Uses UTC time of the transition point.
fn is_us_dst(timestamp: i64) -> bool {
    let dt = timestamp_to_datetime_struct(timestamp as u64);
    let year = dt.year;
    // Second Sunday of March, 2:00 AM EST = 7:00 UTC
    let march_sunday = nth_sunday_of_month(year, 3, 2);
    let dst_start = ymd_to_timestamp(year, 3, march_sunday, 7, 0, 0);
    // First Sunday of November, 2:00 AM EDT = 6:00 UTC
    let nov_sunday = nth_sunday_of_month(year, 11, 1);
    let dst_end = ymd_to_timestamp(year, 11, nov_sunday, 6, 0, 0);
    timestamp >= dst_start && timestamp < dst_end
}

/// Check if a timestamp falls within EU DST (last Sunday of March to
/// last Sunday of October). Uses UTC time.
fn is_eu_dst(timestamp: i64) -> bool {
    let dt = timestamp_to_datetime_struct(timestamp as u64);
    let year = dt.year;
    // Last Sunday of March, 1:00 UTC
    let march_sunday = last_sunday_of_month(year, 3);
    let dst_start = ymd_to_timestamp(year, 3, march_sunday, 1, 0, 0);
    // Last Sunday of October, 1:00 UTC
    let oct_sunday = last_sunday_of_month(year, 10);
    let dst_end = ymd_to_timestamp(year, 10, oct_sunday, 1, 0, 0);
    timestamp >= dst_start && timestamp < dst_end
}

/// Convert year/month/day/hour/min/sec to Unix timestamp (UTC).
fn ymd_to_timestamp(year: i64, month: u32, day: i64, hour: i64, min: i64, sec: i64) -> i64 {
    // Days from epoch (1970-01-01) to the given date
    let mut days = 0i64;
    // Count full years
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    // Count days in months of the current year
    for m in 1..month {
        days += days_in_month(year, m) as i64;
    }
    // Add the day (day is 1-based, so subtract 1)
    days += day - 1;
    days * 86400 + hour * 3600 + min * 60 + sec
}

/// Compute the Unix timestamp for the nth Sunday of a given month/year.
/// Returns the day number (1-31).
fn nth_sunday_of_month(year: i64, month: u32, n: u32) -> i64 {
    // Find the day of week for the 1st of the month
    // Using Zeller's congruence or a simple calculation
    let day1_dow = day_of_week(year, month, 1); // 0=Sunday
    let first_sunday = if day1_dow == 0 { 1 } else { 8 - day1_dow };
    first_sunday + (n - 1) as i64 * 7
}

/// Compute the day number of the last Sunday of a given month/year.
fn last_sunday_of_month(year: i64, month: u32) -> i64 {
    let days_in_month = days_in_month(year, month);
    let last_day_dow = day_of_week(year, month, days_in_month as u32);
    let last_sunday = days_in_month - last_day_dow as i64;
    last_sunday
}

/// Day of week: 0=Sunday, 1=Monday, ..., 6=Saturday
/// Uses Zeller's congruence (Gregorian calendar).
fn day_of_week(year: i64, month: u32, day: u32) -> i64 {
    let (y, m) = if month < 3 { (year - 1, month + 12) } else { (year, month) };
    let k = y % 100;
    let j = y / 100;
    let h = (day as i64 + (13 * (m as i64 + 1)) / 5 + k + k / 4 + j / 4 - 2 * j) % 7;
    // Zeller's: 0=Saturday, so adjust to 0=Sunday
    ((h + 6) % 7 + 7) % 7
}

/// Number of days in a given month.
fn days_in_month(year: i64, month: u32) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 30,
    }
}

/// DST-aware timezone offset for a given timestamp.
/// Returns the offset in seconds from UTC.
pub fn timezone_offset_at(name: &str, timestamp: i64) -> i64 {
    // Non-DST zones: return standard offset
    let non_dst_zones = [
        "UTC", "Z", "GMT", "Asia/Tokyo", "Asia/Seoul", "Asia/Shanghai",
        "Asia/Hong_Kong", "Asia/Singapore", "Asia/Taipei", "Asia/Manila",
        "Asia/Karachi", "Asia/Kolkata", "Asia/Calcutta", "Asia/Dhaka",
        "Asia/Almaty", "Asia/Bangkok", "Asia/Jakarta", "Asia/Ho_Chi_Minh",
        "Asia/Dubai", "Australia/Brisbane", "Pacific/Honolulu",
        "America/Phoenix", "America/Argentina/Buenos_Aires", "Atlantic/Reykjavik",
        "Europe/Moscow",
    ];
    if non_dst_zones.contains(&name) || name.is_empty() {
        return timezone_offset(name);
    }
    // Numeric offsets: always static
    if name.starts_with('+') || name.starts_with('-') {
        return timezone_offset(name);
    }
    // US DST zones
    if matches!(name, "America/New_York" | "America/Detroit") {
        return if is_us_dst(timestamp) { -4 * 3600 } else { -5 * 3600 };
    }
    if matches!(name, "America/Chicago" | "America/Winnipeg") {
        return if is_us_dst(timestamp) { -5 * 3600 } else { -6 * 3600 };
    }
    if matches!(name, "America/Denver") {
        return if is_us_dst(timestamp) { -6 * 3600 } else { -7 * 3600 };
    }
    if matches!(name, "America/Los_Angeles" | "America/Vancouver") {
        return if is_us_dst(timestamp) { -7 * 3600 } else { -8 * 3600 };
    }
    if matches!(name, "America/Anchorage") {
        return if is_us_dst(timestamp) { -8 * 3600 } else { -9 * 3600 };
    }
    // EU DST zones
    if matches!(name, "Europe/London" | "Europe/Lisbon") {
        return if is_eu_dst(timestamp) { 3600 } else { 0 };
    }
    if matches!(name, "Europe/Paris" | "Europe/Berlin" | "Europe/Madrid" | "Europe/Rome" | "Europe/Amsterdam") {
        return if is_eu_dst(timestamp) { 2 * 3600 } else { 3600 };
    }
    if matches!(name, "Europe/Athens" | "Europe/Helsinki" | "Europe/Bucharest") {
        return if is_eu_dst(timestamp) { 3 * 3600 } else { 2 * 3600 };
    }
    // Australia/Sydney: Southern Hemisphere DST (October to April)
    if matches!(name, "Australia/Sydney") {
        return if is_aus_dst(timestamp) { 11 * 3600 } else { 10 * 3600 };
    }
    // Fallback: standard offset
    timezone_offset(name)
}

/// Check if a timestamp falls within Australian DST (first Sunday of October
/// to first Sunday of April).
fn is_aus_dst(timestamp: i64) -> bool {
    let dt = timestamp_to_datetime_struct(timestamp as u64);
    let year = dt.year;
    // First Sunday of October, 2:00 AM local (16:00 UTC previous day)
    let oct_sunday = nth_sunday_of_month(year, 10, 1);
    let dst_start = ymd_to_timestamp(year, 10, oct_sunday, 16, 0, 0);
    // First Sunday of April, 3:00 AM local (17:00 UTC previous day)
    let apr_sunday = nth_sunday_of_month(year, 4, 1);
    let dst_end = ymd_to_timestamp(year, 4, apr_sunday, 17, 0, 0);
    // Southern Hemisphere: DST is October to April
    timestamp >= dst_start || timestamp < dst_end
}

/// Public wrapper for VM dispatch.
pub fn format_datetime_struct(format: &str, dt: &DateTime) -> String {
    format_datetime(format, dt)
}

/// Compute the difference between two timestamps, returning DateInterval fields.
/// Returns (y, m, d, h, i, s, total_days, invert).
pub fn compute_diff(ts1: i64, ts2: i64) -> (i64, i64, i64, i64, i64, i64, i64, bool) {
    let invert = ts2 < ts1;
    let diff_secs = (ts2 - ts1).abs();

    let dt1 = timestamp_to_datetime_struct(ts1 as u64);
    let dt2 = timestamp_to_datetime_struct(ts2 as u64);

    let total_days = diff_secs / 86400;

    // Calendar-aware difference: compute year/month/day/hour/minute/second diffs
    // with borrowing, similar to PHP's DateInterval.
    let mut year = dt2.year - dt1.year;
    let mut month = dt2.month - dt1.month;
    let mut day = dt2.day - dt1.day;
    let mut hour = dt2.hour - dt1.hour;
    let mut minute = dt2.minute - dt1.minute;
    let mut second = dt2.second - dt1.second;

    // Normalize: borrow from higher units when lower is negative
    if second < 0 {
        second += 60;
        minute -= 1;
    }
    if minute < 0 {
        minute += 60;
        hour -= 1;
    }
    if hour < 0 {
        hour += 24;
        day -= 1;
    }
    if day < 0 {
        // Borrow from the previous month of dt2
        let prev_month = if dt2.month == 1 { 12 } else { dt2.month - 1 };
        let prev_year = if dt2.month == 1 {
            dt2.year - 1
        } else {
            dt2.year
        };
        day += get_days_in_month(prev_year, prev_month);
        month -= 1;
    }
    if month < 0 {
        month += 12;
        year -= 1;
    }

    // If inverted, the calendar diff should be computed from dt2 to dt1
    if invert {
        // Recompute with swapped order
        year = dt1.year - dt2.year;
        month = dt1.month - dt2.month;
        day = dt1.day - dt2.day;
        hour = dt1.hour - dt2.hour;
        minute = dt1.minute - dt2.minute;
        second = dt1.second - dt2.second;
        if second < 0 {
            second += 60;
            minute -= 1;
        }
        if minute < 0 {
            minute += 60;
            hour -= 1;
        }
        if hour < 0 {
            hour += 24;
            day -= 1;
        }
        if day < 0 {
            let prev_month = if dt1.month == 1 { 12 } else { dt1.month - 1 };
            let prev_year = if dt1.month == 1 {
                dt1.year - 1
            } else {
                dt1.year
            };
            day += get_days_in_month(prev_year, prev_month);
            month -= 1;
        }
        if month < 0 {
            month += 12;
            year -= 1;
        }
    }

    (year, month, day, hour, minute, second, total_days, invert)
}

fn parse_time_string(time_str: &str, base_time: u64) -> Result<u64, String> {
    let time_str = time_str.trim().to_lowercase();

    if time_str == "now" {
        return Ok(base_time);
    }

    if time_str.starts_with('+') || time_str.starts_with('-') {
        let parts: Vec<&str> = time_str.split_whitespace().collect();
        if parts.len() >= 2 {
            let amount: i64 = parts[0].parse().map_err(|_| "Invalid time string")?;
            let unit = parts[1];

            let seconds = match unit {
                "second" | "seconds" => amount,
                "minute" | "minutes" => amount * 60,
                "hour" | "hours" => amount * 3600,
                "day" | "days" => amount * 86400,
                "week" | "weeks" => amount * 604800,
                "month" | "months" => amount * 2592000,
                "year" | "years" => amount * 31536000,
                _ => return Err(format!("Unknown time unit: {}", unit)),
            };

            return Ok((base_time as i64 + seconds) as u64);
        }
    }

    Err("Unable to parse time string".to_string())
}

/// Parse a datetime string like "2024-01-15 10:30:00" or "2024-01-15T10:30:00"
/// into a Unix timestamp. Returns None if the string doesn't match common formats.
pub fn parse_datetime_string(s: &str) -> Option<i64> {
    let s = s.trim();
    if s == "now" {
        return Some(SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64);
    }

    // Try ISO 8601: YYYY-MM-DD HH:MM:SS or YYYY-MM-DDTHH:MM:SS
    // Also handle YYYY-MM-DD (date only, time = 00:00:00)
    let s = s.replace('T', " ");
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let date_part = parts[0];
    let date_fields: Vec<&str> = date_part.split('-').collect();
    if date_fields.len() != 3 {
        return None;
    }
    let year: i64 = date_fields[0].parse().ok()?;
    let month: i64 = date_fields[1].parse().ok()?;
    let day: i64 = date_fields[2].parse().ok()?;

    let (hour, minute, second) = if parts.len() >= 2 {
        let time_fields: Vec<&str> = parts[1].split(':').collect();
        let h = time_fields
            .first()
            .and_then(|t| t.parse().ok())
            .unwrap_or(0);
        let m = time_fields.get(1).and_then(|t| t.parse().ok()).unwrap_or(0);
        let s = time_fields.get(2).and_then(|t| t.parse().ok()).unwrap_or(0);
        (h, m, s)
    } else {
        (0, 0, 0)
    };

    Some(datetime_to_timestamp(
        year, month, day, hour, minute, second,
    ))
}

/// Parse a datetime string according to a format specifier (PHP `DateTime::createFromFormat`).
/// Supports a subset of PHP format characters: Y, y, m, n, d, j, H, i, s, a, A, G, g.
/// Returns a Unix timestamp, or None if parsing fails.
pub fn parse_from_format(format: &str, datetime_str: &str) -> Option<i64> {
    let fmt_chars: Vec<char> = format.chars().collect();
    let dt_chars: Vec<char> = datetime_str.chars().collect();
    let mut fi = 0;
    let mut di = 0;

    let mut year = 1970i64;
    let mut month = 1i64;
    let mut day = 1i64;
    let mut hour = 0i64;
    let mut minute = 0i64;
    let mut second = 0i64;

    while fi < fmt_chars.len() {
        let fc = fmt_chars[fi];

        if fc == '\\' && fi + 1 < fmt_chars.len() {
            // Escaped literal character
            fi += 1;
            if di < dt_chars.len() && dt_chars[di] == fmt_chars[fi] {
                di += 1;
            }
            fi += 1;
            continue;
        }

        match fc {
            'Y' => {
                // 4-digit year
                if di + 4 > dt_chars.len() {
                    return None;
                }
                year = dt_chars[di..di + 4]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .ok()?;
                di += 4;
            }
            'y' => {
                // 2-digit year (PHP: 00-69 → 2000-2069, 70-99 → 1970-1999)
                if di + 2 > dt_chars.len() {
                    return None;
                }
                let y2: i64 = dt_chars[di..di + 2]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .ok()?;
                year = if y2 < 70 { 2000 + y2 } else { 1900 + y2 };
                di += 2;
            }
            'm' | 'n' => {
                // Month: numeric (with or without leading zero)
                let (val, consumed) = read_number(&dt_chars, di)?;
                month = val;
                di += consumed;
            }
            'd' | 'j' => {
                // Day: numeric
                let (val, consumed) = read_number(&dt_chars, di)?;
                day = val;
                di += consumed;
            }
            'H' | 'G' => {
                // Hour 24-hour format
                let (val, consumed) = read_number(&dt_chars, di)?;
                hour = val;
                di += consumed;
            }
            'i' => {
                // Minute
                let (val, consumed) = read_number(&dt_chars, di)?;
                minute = val;
                di += consumed;
            }
            's' => {
                // Second
                let (val, consumed) = read_number(&dt_chars, di)?;
                second = val;
                di += consumed;
            }
            'a' | 'A' => {
                // am/pm
                if di + 2 > dt_chars.len() {
                    return None;
                }
                let ampm: String = dt_chars[di..di + 2].iter().collect();
                let ampm_lower = ampm.to_lowercase();
                if ampm_lower == "pm" && hour < 12 {
                    hour += 12;
                } else if ampm_lower == "am" && hour == 12 {
                    hour = 0;
                }
                di += 2;
            }
            'g' => {
                // Hour 12-hour format
                let (val, consumed) = read_number(&dt_chars, di)?;
                hour = if val == 12 { 0 } else { val };
                di += consumed;
            }
            // Literal characters: skip whitespace in format, match in input
            ' ' | '\t' => {
                while di < dt_chars.len() && dt_chars[di].is_whitespace() {
                    di += 1;
                }
            }
            _ => {
                // Literal character must match
                if di < dt_chars.len() && dt_chars[di] == fc {
                    di += 1;
                }
            }
        }
        fi += 1;
    }

    Some(datetime_to_timestamp(
        year, month, day, hour, minute, second,
    ))
}

/// Read a number from the character slice starting at `start`.
/// Returns (value, chars_consumed).
fn read_number(chars: &[char], start: usize) -> Option<(i64, usize)> {
    let mut consumed = 0;
    let mut s = String::new();
    while start + consumed < chars.len() && chars[start + consumed].is_ascii_digit() {
        s.push(chars[start + consumed]);
        consumed += 1;
        if consumed >= 4 {
            break;
        }
    }
    if s.is_empty() {
        return None;
    }
    Some((s.parse().ok()?, consumed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time() {
        let result = time_now(&[]).unwrap();
        assert!(zval_get_long(&result) > 0);
    }

    #[test]
    fn test_date_format() {
        let result = date_format(&[
            Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init("Y-m-d", false))),
                PhpType::String,
            ),
            Val::new(PhpValue::Long(0), PhpType::Long),
        ])
        .unwrap();
        assert_eq!(zval_get_string(&result).as_str(), "1970-01-01");
    }

    #[test]
    fn test_mktime() {
        let result = mktime(&[
            Val::new(PhpValue::Long(0), PhpType::Long),
            Val::new(PhpValue::Long(0), PhpType::Long),
            Val::new(PhpValue::Long(0), PhpType::Long),
            Val::new(PhpValue::Long(1), PhpType::Long),
            Val::new(PhpValue::Long(1), PhpType::Long),
            Val::new(PhpValue::Long(1970), PhpType::Long),
        ])
        .unwrap();
        assert_eq!(zval_get_long(&result), 0);
    }

    #[test]
    fn test_strtotime() {
        let result = strtotime(&[Val::new(
            PhpValue::String(Box::new(crate::engine::string::string_init("now", false))),
            PhpType::String,
        )])
        .unwrap();
        assert!(zval_get_long(&result) > 0);

        let result = strtotime(&[
            Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(
                    "+1 day", false,
                ))),
                PhpType::String,
            ),
            Val::new(PhpValue::Long(0), PhpType::Long),
        ])
        .unwrap();
        assert_eq!(zval_get_long(&result), 86400);
    }
}
