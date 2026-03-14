//! DateTime functions
//!
//! PHP datetime functions implementation

use crate::engine::types::{PhpType, PhpValue, Val};
use crate::engine::operators::{zval_get_long, zval_get_string, zval_get_bool};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn time_now(_args: &[Val]) -> Result<Val, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?;
    Ok(Val::new(PhpValue::Long(now.as_secs() as i64), PhpType::Long))
}

pub fn microtime(args: &[Val]) -> Result<Val, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?;
    
    let get_as_float = args.first().map(|v| zval_get_bool(v)).unwrap_or(false);
    
    if get_as_float {
        let secs = now.as_secs() as f64;
        let micros = now.subsec_micros() as f64 / 1_000_000.0;
        Ok(Val::new(PhpValue::Double(secs + micros), PhpType::Double))
    } else {
        let secs = now.as_secs();
        let micros = now.subsec_micros();
        Ok(Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&format!("0.{:06} {}", micros, secs), false))), PhpType::String))
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
    
    Ok(Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&formatted, false))), PhpType::String))
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
struct DateTime {
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

fn datetime_to_timestamp(year: i64, month: i64, day: i64, hour: i64, minute: i64, second: i64) -> i64 {
    let mut days = 0i64;
    
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    
    for m in 1..month {
        days += get_days_in_month(year, m);
    }
    
    days += day - 1;
    
    let seconds = days * 86400 + hour * 3600 + minute * 60 + second;
    seconds
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn get_days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if is_leap_year(year) { 29 } else { 28 },
        _ => 30,
    }
}

fn format_datetime(format: &str, dt: &DateTime) -> String {
    let mut result = String::new();
    let mut chars = format.chars().peekable();
    
    while let Some(c) = chars.next() {
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
                let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
                result.push_str(months[(dt.month - 1) as usize]);
            }
            _ => result.push(c),
        }
    }
    
    result
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
        let result = date_format(&[Val::new(PhpValue::String(Box::new(crate::engine::string::string_init("Y-m-d", false))), PhpType::String), Val::new(PhpValue::Long(0), PhpType::Long)]).unwrap();
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
        ]).unwrap();
        assert_eq!(zval_get_long(&result), 0);
    }

    #[test]
    fn test_strtotime() {
        let result = strtotime(&[Val::new(PhpValue::String(Box::new(crate::engine::string::string_init("now", false))), PhpType::String)]).unwrap();
        assert!(zval_get_long(&result) > 0);
        
        let result = strtotime(&[Val::new(PhpValue::String(Box::new(crate::engine::string::string_init("+1 day", false))), PhpType::String), Val::new(PhpValue::Long(0), PhpType::Long)]).unwrap();
        assert_eq!(zval_get_long(&result), 86400);
    }
}
