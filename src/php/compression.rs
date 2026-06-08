//! Compression functions
//!
//! PHP compression functions: gzcompress, gzuncompress, gzencode, gzdecode,
//! gzdeflate, gzinflate

use crate::engine::operators::zval_get_long;
use crate::engine::operators::zval_get_string;
use crate::engine::string::string_init;
use crate::engine::types::{PhpType, PhpValue, Val};
use std::io::{Read, Write};

#[allow(dead_code)]
fn string_val(s: &str) -> Val {
    Val::new(
        PhpValue::String(Box::new(string_init(s, false))),
        PhpType::String,
    )
}

fn bytes_val(bytes: &[u8]) -> Val {
    Val::new(
        PhpValue::String(Box::new(crate::engine::types::PhpString::from_bytes(
            bytes, false,
        ))),
        PhpType::String,
    )
}

/// Extract raw bytes from a Val, supporting binary data
fn zval_get_bytes(val: &Val) -> Vec<u8> {
    match &val.value {
        PhpValue::String(s) => s.as_bytes().to_vec(),
        _ => zval_get_string(val).as_str().as_bytes().to_vec(),
    }
}

/// Parse compression level from argument, defaulting to flate2::Compression::default()
fn parse_level(args: &[Val], idx: usize) -> flate2::Compression {
    let level = if args.len() > idx {
        zval_get_long(&args[idx]) as u32
    } else {
        6
    };
    if level == 0 {
        flate2::Compression::none()
    } else if level <= 9 {
        flate2::Compression::new(level)
    } else {
        flate2::Compression::default()
    }
}

/// gzcompress($data, $level = -1) - Compress a string using zlib
pub fn gzcompress(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("gzcompress() expects at least 1 argument".to_string());
    }

    let input_bytes = zval_get_bytes(&args[0]);
    let compression = parse_level(args, 1);

    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), compression);
    encoder
        .write_all(&input_bytes)
        .map_err(|e| format!("gzcompress(): {}", e))?;
    let compressed = encoder.finish().map_err(|e| format!("gzcompress(): {}", e))?;

    Ok(bytes_val(&compressed))
}

/// gzuncompress($data, $length = 0) - Decompress a zlib-compressed string
pub fn gzuncompress(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("gzuncompress() expects at least 1 argument".to_string());
    }

    let input_bytes = zval_get_bytes(&args[0]);

    let mut decoder = flate2::write::ZlibDecoder::new(Vec::new());
    decoder
        .write_all(&input_bytes)
        .map_err(|e| format!("gzuncompress(): {}", e))?;
    let decompressed = decoder
        .finish()
        .map_err(|e| format!("gzuncompress(): {}", e))?;

    Ok(bytes_val(&decompressed))
}

/// gzencode($data, $level = -1, $encoding = ZLIB_ENCODING_GZIP) - Create a gzip-compressed string
pub fn gzencode(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("gzencode() expects at least 1 argument".to_string());
    }

    let input_bytes = zval_get_bytes(&args[0]);
    let compression = parse_level(args, 1);

    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), compression);
    encoder
        .write_all(&input_bytes)
        .map_err(|e| format!("gzencode(): {}", e))?;
    let compressed = encoder.finish().map_err(|e| format!("gzencode(): {}", e))?;

    Ok(bytes_val(&compressed))
}

/// gzdecode($data) - Decode a gzip-compressed string
pub fn gzdecode(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("gzdecode() expects at least 1 argument".to_string());
    }

    let input_bytes = zval_get_bytes(&args[0]);

    let mut decoder = flate2::read::GzDecoder::new(&input_bytes[..]);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("gzdecode(): {}", e))?;

    Ok(bytes_val(&decompressed))
}

/// gzdeflate($data, $level = -1) - Deflate a string
pub fn gzdeflate(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("gzdeflate() expects at least 1 argument".to_string());
    }

    let input_bytes = zval_get_bytes(&args[0]);
    let compression = parse_level(args, 1);

    let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), compression);
    encoder
        .write_all(&input_bytes)
        .map_err(|e| format!("gzdeflate(): {}", e))?;
    let compressed = encoder.finish().map_err(|e| format!("gzdeflate(): {}", e))?;

    Ok(bytes_val(&compressed))
}

/// gzinflate($data, $length = 0) - Inflate a deflated string
pub fn gzinflate(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("gzinflate() expects at least 1 argument".to_string());
    }

    let input_bytes = zval_get_bytes(&args[0]);

    let mut decoder = flate2::read::DeflateDecoder::new(&input_bytes[..]);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| format!("gzinflate(): {}", e))?;

    Ok(bytes_val(&decompressed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gzcompress_roundtrip() {
        let input = string_val("hello world");
        let compressed = gzcompress(&[input.clone()]).unwrap();
        let decompressed = gzuncompress(&[compressed]).unwrap();
        assert_eq!(zval_get_string(&decompressed).as_str(), "hello world");
    }

    #[test]
    fn test_gzencode_roundtrip() {
        let input = string_val("hello world");
        let encoded = gzencode(&[input.clone()]).unwrap();
        let decoded = gzdecode(&[encoded]).unwrap();
        assert_eq!(zval_get_string(&decoded).as_str(), "hello world");
    }

    #[test]
    fn test_gzdeflate_roundtrip() {
        let input = string_val("hello world");
        let deflated = gzdeflate(&[input.clone()]).unwrap();
        let inflated = gzinflate(&[deflated]).unwrap();
        assert_eq!(zval_get_string(&inflated).as_str(), "hello world");
    }

    #[test]
    fn test_gzcompress_with_level() {
        let input = string_val("hello world");
        let level = Val::new(PhpValue::Long(9), PhpType::Long);
        let compressed = gzcompress(&[input, level]).unwrap();
        // Should succeed and return non-empty
        assert!(!zval_get_string(&compressed).as_str().is_empty());
    }

    #[test]
    fn test_gzencode_with_level() {
        let input = string_val("hello world");
        let level = Val::new(PhpValue::Long(1), PhpType::Long);
        let encoded = gzencode(&[input, level]).unwrap();
        let decoded = gzdecode(&[encoded]).unwrap();
        assert_eq!(zval_get_string(&decoded).as_str(), "hello world");
    }
}
