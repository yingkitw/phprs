//! Multibyte string functions
//!
//! PHP mbstring functions for handling multibyte character encodings

use crate::engine::operators::zval_get_string;
use crate::engine::types::{PhpType, PhpValue, Val};
use unicode_segmentation::UnicodeSegmentation;

fn string_val(s: &str) -> Val {
    Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(s, false))),
        PhpType::String,
    )
}

/// mb_strlen($str, $encoding = null) - Get string length with multibyte support
pub fn mb_strlen(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("mb_strlen() expects at least 1 argument".to_string());
    }

    let s = zval_get_string(&args[0]);
    let s_str = s.as_str();
    
    // Count Unicode grapheme clusters
    let len = s_str.graphemes(true).count();
    
    Ok(Val::new(PhpValue::Long(len as i64), PhpType::Long))
}

/// mb_substr($str, $start, $length = null, $encoding = null) - Get part of string
pub fn mb_substr(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("mb_substr() expects at least 2 arguments".to_string());
    }

    let s = zval_get_string(&args[0]);
    let s_str = s.as_str();
    let start = crate::engine::operators::zval_get_long(&args[1]) as isize;
    
    let graphemes: Vec<&str> = s_str.graphemes(true).collect();
    let total = graphemes.len();
    
    // Handle negative start
    let start = if start < 0 {
        let abs_start = start.abs() as usize;
        if abs_start >= total {
            return Ok(string_val(""));
        }
        total - abs_start
    } else {
        start as usize
    };
    
    if start >= total {
        return Ok(string_val(""));
    }
    
    // Handle length
    let end = if args.len() > 2 {
        let length = crate::engine::operators::zval_get_long(&args[2]) as isize;
        if length < 0 {
            let abs_length = length.abs() as usize;
            if abs_length >= total - start {
                return Ok(string_val(""));
            }
            total - abs_length
        } else {
            let length = length as usize;
            std::cmp::min(start + length, total)
        }
    } else {
        total
    };
    
    let result: String = graphemes[start..end].concat();
    Ok(string_val(&result))
}

/// mb_strtolower($str, $encoding = null) - Make string lowercase
pub fn mb_strtolower(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("mb_strtolower() expects at least 1 argument".to_string());
    }

    let s = zval_get_string(&args[0]);
    let s_str = s.as_str();
    
    Ok(string_val(&s_str.to_lowercase()))
}

/// mb_strtoupper($str, $encoding = null) - Make string uppercase
pub fn mb_strtoupper(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("mb_strtoupper() expects at least 1 argument".to_string());
    }

    let s = zval_get_string(&args[0]);
    let s_str = s.as_str();
    
    Ok(string_val(&s_str.to_uppercase()))
}

/// mb_strpos($haystack, $needle, $offset = 0, $encoding = null) - Find position of first occurrence
pub fn mb_strpos(args: &[Val]) -> Result<Val, String> {
    if args.len() < 2 {
        return Err("mb_strpos() expects at least 2 arguments".to_string());
    }

    let haystack = zval_get_string(&args[0]);
    let haystack_str = haystack.as_str();
    let needle = zval_get_string(&args[1]);
    let needle_str = needle.as_str();
    
    if needle_str.is_empty() {
        return Err("mb_strpos(): Empty delimiter".to_string());
    }
    
    let offset = if args.len() > 2 {
        let off = crate::engine::operators::zval_get_long(&args[2]) as usize;
        if off > 0 {
            let graphemes: Vec<&str> = haystack_str.graphemes(true).collect();
            if off >= graphemes.len() {
                return Ok(Val::new(PhpValue::Long(0), PhpType::False));
            }
            graphemes[off..].concat()
        } else {
            haystack_str.to_string()
        }
    } else {
        haystack_str.to_string()
    };
    
    match offset.find(needle_str) {
        Some(pos) => {
            let graphemes_before = offset[..pos].graphemes(true).count();
            let base_offset = if args.len() > 2 {
                crate::engine::operators::zval_get_long(&args[2]).max(0) as usize
            } else {
                0
            };
            Ok(Val::new(PhpValue::Long((graphemes_before + base_offset) as i64), PhpType::Long))
        }
        None => Ok(Val::new(PhpValue::Long(0), PhpType::False)),
    }
}

/// mb_strrpos($haystack, $needle, $offset = 0, $encoding = null) - Find position of last occurrence
pub fn mb_strrpos(args: &[Val]) -> Result<Val, String> {
    if args.len() < 2 {
        return Err("mb_strrpos() expects at least 2 arguments".to_string());
    }

    let haystack = zval_get_string(&args[0]);
    let haystack_str = haystack.as_str();
    let needle = zval_get_string(&args[1]);
    let needle_str = needle.as_str();
    
    if needle_str.is_empty() {
        return Err("mb_strrpos(): Empty delimiter".to_string());
    }
    
    let graphemes: Vec<&str> = haystack_str.graphemes(true).collect();
    
    // Handle offset
    let end = if args.len() > 2 {
        let off = crate::engine::operators::zval_get_long(&args[2]) as i64;
        if off < 0 {
            let abs_off = off.abs() as usize;
            if abs_off >= graphemes.len() {
                return Ok(Val::new(PhpValue::Long(0), PhpType::False));
            }
            graphemes.len() - abs_off
        } else {
            let off = off as usize;
            if off >= graphemes.len() {
                graphemes.len()
            } else {
                off + 1
            }
        }
    } else {
        graphemes.len()
    };
    
    // Search from the end
    for i in (0..end).rev() {
        if graphemes[i..end].concat().starts_with(needle_str) {
            return Ok(Val::new(PhpValue::Long(i as i64), PhpType::Long));
        }
    }
    
    Ok(Val::new(PhpValue::Long(0), PhpType::False))
}

/// mb_convert_encoding($str, $to_encoding, $from_encoding = null) - Convert character encoding
pub fn mb_convert_encoding(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("mb_convert_encoding() expects at least 1 argument".to_string());
    }

    let s = zval_get_string(&args[0]);
    let s_str = s.as_str();
    
    // For now, we only handle UTF-8 to UTF-8 (no conversion needed)
    // Full implementation would use encoding_rs or similar crate
    Ok(string_val(s_str))
}

/// mb_substr_count($haystack, $needle, $encoding = null) - Count number of substring occurrences
pub fn mb_substr_count(args: &[Val]) -> Result<Val, String> {
    if args.len() < 2 {
        return Err("mb_substr_count() expects at least 2 arguments".to_string());
    }

    let haystack = zval_get_string(&args[0]);
    let haystack_str = haystack.as_str();
    let needle = zval_get_string(&args[1]);
    let needle_str = needle.as_str();
    
    if needle_str.is_empty() {
        return Err("mb_substr_count(): Empty substring".to_string());
    }
    
    let count = haystack_str.match_indices(needle_str).count();
    
    Ok(Val::new(PhpValue::Long(count as i64), PhpType::Long))
}

/// mb_strwidth($str, $encoding = null) - Return width of string
pub fn mb_strwidth(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("mb_strwidth() expects at least 1 argument".to_string());
    }

    let s = zval_get_string(&args[0]);
    let s_str = s.as_str();
    
    // Calculate display width (East Asian Width)
    let width = s_str
        .chars()
        .map(|c| {
            // Simplified width calculation
            if is_wide_char(c) {
                2
            } else {
                1
            }
        })
        .sum::<usize>();
    
    Ok(Val::new(PhpValue::Long(width as i64), PhpType::Long))
}

fn is_wide_char(c: char) -> bool {
    // Simplified check for wide characters (CJK, etc.)
    // Full implementation would use unicode-width crate
    let cp = c as u32;
    (cp >= 0x1100 && cp <= 0x115F) ||
    (cp >= 0x2E80 && cp <= 0xA4CF) ||
    (cp >= 0xAC00 && cp <= 0xD7A3) ||
    (cp >= 0xF900 && cp <= 0xFAFF) ||
    (cp >= 0xFE10 && cp <= 0xFE19) ||
    (cp >= 0xFE30 && cp <= 0xFE6F) ||
    (cp >= 0xFF00 && cp <= 0xFF60) ||
    (cp >= 0xFFE0 && cp <= 0xFFE6) ||
    (cp >= 0x20000 && cp <= 0x2FFFD) ||
    (cp >= 0x30000 && cp <= 0x3FFFD)
}

/// mb_strimwidth($str, $start, $width, $trimmarker = "", $encoding = null) - Get truncated string
pub fn mb_strimwidth(args: &[Val]) -> Result<Val, String> {
    if args.len() < 3 {
        return Err("mb_strimwidth() expects at least 3 arguments".to_string());
    }

    let s = zval_get_string(&args[0]);
    let s_str = s.as_str();
    let start = crate::engine::operators::zval_get_long(&args[1]) as usize;
    let width = crate::engine::operators::zval_get_long(&args[2]) as usize;
    let trimmarker = if args.len() > 3 {
        zval_get_string(&args[3]).as_str().to_string()
    } else {
        String::new()
    };
    
    let chars: Vec<char> = s_str.chars().collect();
    let total_width: usize = chars.iter().map(|c| if is_wide_char(*c) { 2 } else { 1 }).sum();
    
    if start >= total_width {
        return Ok(string_val(""));
    }
    
    let mut current_width = 0;
    let mut result = String::new();
    let marker_width = trimmarker.chars().map(|c| if is_wide_char(c) { 2 } else { 1 }).sum::<usize>();
    
    for c in chars {
        if current_width < start {
            current_width += if is_wide_char(c) { 2 } else { 1 };
            continue;
        }
        
        let char_width = if is_wide_char(c) { 2 } else { 1 };
        let remaining = width - (current_width - start);
        
        if remaining < marker_width + char_width {
            result.push_str(&trimmarker);
            break;
        }
        
        result.push(c);
        current_width += char_width;
    }
    
    Ok(string_val(&result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mb_strlen_ascii() {
        let val = string_val("hello");
        let result = mb_strlen(&[val]).unwrap();
        assert_eq!(result.value, PhpValue::Long(5));
    }

    #[test]
    fn test_mb_strlen_unicode() {
        let val = string_val("hello 世界");
        let result = mb_strlen(&[val]).unwrap();
        assert_eq!(result.value, PhpValue::Long(8));
    }

    #[test]
    fn test_mb_substr_ascii() {
        let val = string_val("hello world");
        let result = mb_substr(&[val, Val::new(PhpValue::Long(6), PhpType::Long)]).unwrap();
        assert_eq!(zval_get_string(&result).as_str(), "world");
    }

    #[test]
    fn test_mb_substr_unicode() {
        let val = string_val("hello 世界");
        let result = mb_substr(&[val, Val::new(PhpValue::Long(6), PhpType::Long)]).unwrap();
        assert_eq!(zval_get_string(&result).as_str(), "世界");
    }

    #[test]
    fn test_mb_strtolower() {
        let val = string_val("HELLO WORLD");
        let result = mb_strtolower(&[val]).unwrap();
        assert_eq!(zval_get_string(&result).as_str(), "hello world");
    }

    #[test]
    fn test_mb_strtoupper() {
        let val = string_val("hello world");
        let result = mb_strtoupper(&[val]).unwrap();
        assert_eq!(zval_get_string(&result).as_str(), "HELLO WORLD");
    }

    #[test]
    fn test_mb_strpos() {
        let haystack = string_val("hello world");
        let needle = string_val("world");
        let result = mb_strpos(&[haystack, needle]).unwrap();
        assert_eq!(result.value, PhpValue::Long(6));
    }

    #[test]
    fn test_mb_strpos_not_found() {
        let haystack = string_val("hello world");
        let needle = string_val("xyz");
        let result = mb_strpos(&[haystack, needle]).unwrap();
        assert_eq!(result.get_type(), PhpType::False);
    }

    #[test]
    fn test_mb_substr_count() {
        let haystack = string_val("hello world world");
        let needle = string_val("world");
        let result = mb_substr_count(&[haystack, needle]).unwrap();
        assert_eq!(result.value, PhpValue::Long(2));
    }

    #[test]
    fn test_mb_strwidth() {
        let val = string_val("hello");
        let result = mb_strwidth(&[val]).unwrap();
        assert_eq!(result.value, PhpValue::Long(5));
    }
}
