//! CSV handling functions
//!
//! PHP CSV functions: str_getcsv, fgetcsv, fputcsv

use crate::engine::operators::zval_get_string;
use crate::engine::string::string_init;
use crate::engine::types::{PhpType, PhpValue, Val};

fn string_val(s: &str) -> Val {
    Val::new(
        PhpValue::String(Box::new(string_init(s, false))),
        PhpType::String,
    )
}

fn make_array(values: Vec<Val>) -> Val {
    let mut arr = crate::engine::types::PhpArray::new();
    for (i, val) in values.into_iter().enumerate() {
        let _ = crate::engine::hash::hash_add_or_update(&mut arr, None, i as u64, val, 0);
    }
    Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array)
}

/// Parse a CSV line into fields.
/// Handles enclosures, escapes, and empty fields.
fn parse_csv_line(
    line: &str,
    delimiter: char,
    enclosure: char,
    escape: char,
) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_enclosure = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_enclosure {
            if ch == escape {
                // Escaped character
                if let Some(&next) = chars.peek() {
                    if next == enclosure || next == escape || next == delimiter {
                        chars.next();
                        current.push(next);
                    } else {
                        current.push(ch);
                    }
                } else {
                    current.push(ch);
                }
            } else if ch == enclosure {
                // Check for doubled enclosure (escaped enclosure)
                if chars.peek() == Some(&enclosure) {
                    chars.next();
                    current.push(enclosure);
                } else {
                    in_enclosure = false;
                }
            } else {
                current.push(ch);
            }
        } else {
            if ch == delimiter {
                fields.push(std::mem::take(&mut current));
            } else if ch == enclosure {
                in_enclosure = true;
            } else {
                current.push(ch);
            }
        }
    }

    fields.push(current);
    fields
}

/// Format fields into a CSV line.
fn format_csv_line(
    fields: &[String],
    delimiter: char,
    enclosure: char,
    escape: char,
) -> String {
    let mut result = String::new();
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            result.push(delimiter);
        }
        let needs_enclosure = field.contains(delimiter)
            || field.contains(enclosure)
            || field.contains('\n')
            || field.contains('\r');
        if needs_enclosure {
            result.push(enclosure);
            for ch in field.chars() {
                if ch == enclosure {
                    result.push(escape);
                }
                result.push(ch);
            }
            result.push(enclosure);
        } else {
            result.push_str(field);
        }
    }
    result
}

/// str_getcsv($string, $separator = ",", $enclosure = "\"", $escape = "\\")
///
/// Parses a CSV string into an array.
pub fn str_getcsv(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("str_getcsv() expects at least 1 argument".to_string());
    }

    let input = zval_get_string(&args[0]).as_str().to_string();
    let delimiter = if args.len() > 1 {
        let s = zval_get_string(&args[1]).as_str().to_string();
        s.chars().next().unwrap_or(',')
    } else {
        ','
    };
    let enclosure = if args.len() > 2 {
        let s = zval_get_string(&args[2]).as_str().to_string();
        s.chars().next().unwrap_or('"')
    } else {
        '"'
    };
    let escape = if args.len() > 3 {
        let s = zval_get_string(&args[3]).as_str().to_string();
        s.chars().next().unwrap_or('\\')
    } else {
        '\\'
    };

    // PHP's str_getcsv only parses the first line
    let line = input.lines().next().unwrap_or(&input);
    let fields = parse_csv_line(line, delimiter, enclosure, escape);
    Ok(make_array(fields.into_iter().map(|s| string_val(&s)).collect()))
}

/// fgetcsv($handle, $length = 0, $separator = ",", $enclosure = "\"", $escape = "\\")
///
/// Reads a line from a file and parses it as CSV.
/// In this implementation, the handle is a file path string.
pub fn fgetcsv(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("fgetcsv() expects at least 1 argument".to_string());
    }

    let path = zval_get_string(&args[0]).as_str().to_string();
    let delimiter = if args.len() > 2 {
        let s = zval_get_string(&args[2]).as_str().to_string();
        s.chars().next().unwrap_or(',')
    } else {
        ','
    };
    let enclosure = if args.len() > 3 {
        let s = zval_get_string(&args[3]).as_str().to_string();
        s.chars().next().unwrap_or('"')
    } else {
        '"'
    };
    let escape = if args.len() > 4 {
        let s = zval_get_string(&args[4]).as_str().to_string();
        s.chars().next().unwrap_or('\\')
    } else {
        '\\'
    };

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Ok(Val::new(PhpValue::Long(0), PhpType::False)),
    };

    let line = content.lines().next().unwrap_or("");
    let fields = parse_csv_line(line, delimiter, enclosure, escape);
    Ok(make_array(fields.into_iter().map(|s| string_val(&s)).collect()))
}

/// fputcsv($handle, $fields, $separator = ",", $enclosure = "\"", $escape = "\\", $eol = "\n")
///
/// Formats a line as CSV and writes it to a file.
/// In this implementation, the handle is a file path string.
pub fn fputcsv(args: &[Val]) -> Result<Val, String> {
    if args.len() < 2 {
        return Err("fputcsv() expects at least 2 arguments".to_string());
    }

    let path = zval_get_string(&args[0]).as_str().to_string();

    let fields = if let PhpValue::Array(ref arr) = args[1].value {
        arr.ar_data
            .iter()
            .map(|b| zval_get_string(&b.val).as_str().to_string())
            .collect::<Vec<String>>()
    } else {
        return Err("fputcsv() expects array as second argument".to_string());
    };

    let delimiter = if args.len() > 2 {
        let s = zval_get_string(&args[2]).as_str().to_string();
        s.chars().next().unwrap_or(',')
    } else {
        ','
    };
    let enclosure = if args.len() > 3 {
        let s = zval_get_string(&args[3]).as_str().to_string();
        s.chars().next().unwrap_or('"')
    } else {
        '"'
    };
    let escape = if args.len() > 4 {
        let s = zval_get_string(&args[4]).as_str().to_string();
        s.chars().next().unwrap_or('\\')
    } else {
        '\\'
    };
    let eol = if args.len() > 5 {
        zval_get_string(&args[5]).as_str().to_string()
    } else {
        "\n".to_string()
    };

    let line = format_csv_line(&fields, delimiter, enclosure, escape);
    let output = line + &eol;

    // Append to file
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("fputcsv(): {}", e))?;

    let bytes_written = output.len();
    file.write_all(output.as_bytes())
        .map_err(|e| format!("fputcsv(): {}", e))?;

    Ok(Val::new(PhpValue::Long(bytes_written as i64), PhpType::Long))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_getcsv_basic() {
        let input = string_val("foo,bar,baz");
        let result = str_getcsv(&[input]).unwrap();
        if let PhpValue::Array(ref arr) = result.value {
            assert_eq!(arr.ar_data.len(), 3);
            assert_eq!(zval_get_string(&arr.ar_data[0].val).as_str(), "foo");
            assert_eq!(zval_get_string(&arr.ar_data[1].val).as_str(), "bar");
            assert_eq!(zval_get_string(&arr.ar_data[2].val).as_str(), "baz");
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_str_getcsv_enclosure() {
        let input = string_val("\"hello, world\",baz");
        let result = str_getcsv(&[input]).unwrap();
        if let PhpValue::Array(ref arr) = result.value {
            assert_eq!(arr.ar_data.len(), 2);
            assert_eq!(zval_get_string(&arr.ar_data[0].val).as_str(), "hello, world");
            assert_eq!(zval_get_string(&arr.ar_data[1].val).as_str(), "baz");
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_str_getcsv_escaped_enclosure() {
        let input_str = r#""he said ""hello""",baz"#;
        let input = string_val(input_str);
        let result = str_getcsv(&[input]).unwrap();
        if let PhpValue::Array(ref arr) = result.value {
            assert_eq!(arr.ar_data.len(), 2);
            assert_eq!(zval_get_string(&arr.ar_data[0].val).as_str(), "he said \"hello\"");
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_str_getcsv_empty_field() {
        let input = string_val("foo,,baz");
        let result = str_getcsv(&[input]).unwrap();
        if let PhpValue::Array(ref arr) = result.value {
            assert_eq!(arr.ar_data.len(), 3);
            assert_eq!(zval_get_string(&arr.ar_data[0].val).as_str(), "foo");
            assert_eq!(zval_get_string(&arr.ar_data[1].val).as_str(), "");
            assert_eq!(zval_get_string(&arr.ar_data[2].val).as_str(), "baz");
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_str_getcsv_custom_delimiter() {
        let input = string_val("foo;bar;baz");
        let delim = string_val(";");
        let result = str_getcsv(&[input, delim]).unwrap();
        if let PhpValue::Array(ref arr) = result.value {
            assert_eq!(arr.ar_data.len(), 3);
            assert_eq!(zval_get_string(&arr.ar_data[0].val).as_str(), "foo");
            assert_eq!(zval_get_string(&arr.ar_data[1].val).as_str(), "bar");
            assert_eq!(zval_get_string(&arr.ar_data[2].val).as_str(), "baz");
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_fputcsv_and_fgetcsv_roundtrip() {
        let temp_path = std::env::temp_dir().join("phprs_csv_test.txt");
        let path_str = temp_path.to_str().unwrap();

        // Clean up if exists
        let _ = std::fs::remove_file(path_str);

        // Build array
        let mut arr = crate::engine::types::PhpArray::new();
        let ks1 = Box::new(string_init("0", false));
        let _ = crate::engine::hash::hash_add_or_update(&mut arr, Some(&*ks1), 0, string_val("hello"), 0);
        let ks2 = Box::new(string_init("1", false));
        let _ = crate::engine::hash::hash_add_or_update(&mut arr, Some(&*ks2), 0, string_val("world, test"), 0);
        let arr_val = Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array);

        let path_val = string_val(path_str);
        let result = fputcsv(&[path_val.clone(), arr_val]).unwrap();
        assert!(matches!(result.value, PhpValue::Long(_)));

        let read_result = fgetcsv(&[path_val]).unwrap();
        if let PhpValue::Array(ref arr) = read_result.value {
            assert_eq!(arr.ar_data.len(), 2);
            assert_eq!(zval_get_string(&arr.ar_data[0].val).as_str(), "hello");
            assert_eq!(zval_get_string(&arr.ar_data[1].val).as_str(), "world, test");
        } else {
            panic!("Expected array");
        }

        let _ = std::fs::remove_file(path_str);
    }
}
