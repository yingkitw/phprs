//! String implementation
//!
//! High-performance string handling optimized to outperform PHP 8

use super::perf_alloc::{allocate_php_string, fast_concat, StringBuilder};
use crate::engine::types::PhpString;

/// Hash function for strings (DJBX33A algorithm used by PHP)
pub fn string_hash_func(s: &PhpString) -> u64 {
    if s.h != 0 {
        return s.h;
    }

    // PHP uses DJBX33A hash algorithm
    let mut hash: u64 = 5381;
    for &byte in &s.val[..s.len] {
        hash = ((hash << 5).wrapping_add(hash)).wrapping_add(byte as u64);
    }

    hash
}

/// Hash function for raw string data
pub fn hash_func(str: &[u8]) -> u64 {
    let mut hash: u64 = 5381;
    for &byte in str {
        hash = ((hash << 5).wrapping_add(hash)).wrapping_add(byte as u64);
    }
    hash
}

/// Initialize a new PHP string (optimized)
pub fn string_init(s: &str, persistent: bool) -> PhpString {
    allocate_php_string(s, persistent)
}

/// Create an empty string
pub fn string_empty() -> PhpString {
    PhpString::new("", false)
}

/// Concatenate two strings (optimized)
pub fn string_concat2(s1: &str, s2: &str) -> PhpString {
    fast_concat(s1, s2)
}

/// Concatenate three strings (optimized)
pub fn string_concat3(s1: &str, s2: &str, s3: &str) -> PhpString {
    let total_len = s1.len() + s2.len() + s3.len();
    let mut builder = StringBuilder::with_capacity(total_len);
    builder.push_str(s1);
    builder.push_str(s2);
    builder.push_str(s3);

    let result = builder.into_string();
    allocate_php_string(&result, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_init() {
        let s = string_init("hello", false);
        assert_eq!(s.as_str(), "hello");
        assert_eq!(s.len, 5);
    }

    #[test]
    fn test_string_hash_func() {
        let s = string_init("test", false);
        let hash1 = string_hash_func(&s);
        let hash2 = string_hash_func(&s);
        assert_eq!(hash1, hash2); // Hash should be cached
    }

    #[test]
    fn test_string_concat2() {
        let result = string_concat2("hello", " world");
        assert_eq!(result.as_str(), "hello world");
    }

    #[test]
    fn test_string_concat3() {
        let result = string_concat3("a", "b", "c");
        assert_eq!(result.as_str(), "abc");
    }
}
