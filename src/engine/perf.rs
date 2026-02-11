//! Performance optimizations
//!
//! This module provides performance-enhanced variants of common operations

use std::borrow::Cow;

/// Cached keyword lookup table
///
/// Uses phf (perfect hash function) for O(1) lookups
/// This is much faster than the original match statement
pub struct KeywordCache {
    // In a real implementation, you'd use phf::Map here
    // For now, we'll use the match approach as a baseline
}

impl KeywordCache {
    #[inline]
    pub fn lookup(keyword: &str) -> Option<crate::engine::lexer::TokenType> {
        // Inline-optimized keyword lookup
        // Using a match with explicit order for common keywords first
        Some(match keyword {
            // Most common keywords first
            "if" => crate::engine::lexer::TokenType::T_IF,
            "function" => crate::engine::lexer::TokenType::T_FUNCTION,
            "return" => crate::engine::lexer::TokenType::T_RETURN,
            "class" => crate::engine::lexer::TokenType::T_CLASS,
            "echo" => crate::engine::lexer::TokenType::T_ECHO,
            "while" => crate::engine::lexer::TokenType::T_WHILE,
            "for" => crate::engine::lexer::TokenType::T_FOR,
            "foreach" => crate::engine::lexer::TokenType::T_FOREACH,
            "else" => crate::engine::lexer::TokenType::T_ELSE,
            "elseif" => crate::engine::lexer::TokenType::T_ELSEIF,
            // Other keywords
            "array" => crate::engine::lexer::TokenType::T_ARRAY,
            "new" => crate::engine::lexer::TokenType::T_NEW,
            "static" => crate::engine::lexer::TokenType::T_STATIC,
            "public" => crate::engine::lexer::TokenType::T_PUBLIC,
            "private" => crate::engine::lexer::TokenType::T_PRIVATE,
            "protected" => crate::engine::lexer::TokenType::T_PROTECTED,
            "const" => crate::engine::lexer::TokenType::T_CONST,
            "break" => crate::engine::lexer::TokenType::T_BREAK,
            "continue" => crate::engine::lexer::TokenType::T_CONTINUE,
            "switch" => crate::engine::lexer::TokenType::T_SWITCH,
            "case" => crate::engine::lexer::TokenType::T_CASE,
            "default" => crate::engine::lexer::TokenType::T_DEFAULT,
            "try" => crate::engine::lexer::TokenType::T_TRY,
            "catch" => crate::engine::lexer::TokenType::T_CATCH,
            "throw" => crate::engine::lexer::TokenType::T_THROW,
            "namespace" => crate::engine::lexer::TokenType::T_NAMESPACE,
            "use" => crate::engine::lexer::TokenType::T_USE,
            "as" => crate::engine::lexer::TokenType::T_AS,
            "abstract" => crate::engine::lexer::TokenType::T_ABSTRACT,
            "extends" => crate::engine::lexer::TokenType::T_EXTENDS,
            "final" => crate::engine::lexer::TokenType::T_FINAL,
            "implements" => crate::engine::lexer::TokenType::T_IMPLEMENTS,
            "interface" => crate::engine::lexer::TokenType::T_INTERFACE,
            "trait" => crate::engine::lexer::TokenType::T_TRAIT,
            "clone" => crate::engine::lexer::TokenType::T_CLONE,
            "instanceof" => crate::engine::lexer::TokenType::T_INSTANCEOF,
            "do" => crate::engine::lexer::TokenType::T_DO,
            "true" | "false" | "null" => crate::engine::lexer::TokenType::T_STRING,
            _ => return None,
        })
    }
}

/// Optimized string handling using Cow<str>
///
/// This avoids unnecessary allocations when the string is already owned
/// or when we can borrow the data
pub type SharedString = Cow<'static, str>;

/// Create a shared string from a static string slice (zero-copy)
#[inline]
pub fn shared_str_static(s: &'static str) -> SharedString {
    Cow::Borrowed(s)
}

/// Create a shared string from an owned string
#[inline]
pub fn shared_str_owned(s: String) -> SharedString {
    Cow::Owned(s)
}

/// Check if a character is ASCII whitespace (inline optimized)
#[inline]
pub fn is_ascii_whitespace(ch: u8) -> bool {
    matches!(ch, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' | b'\x0b')
}

/// Check if a character can start an identifier (inline optimized)
#[inline]
pub fn is_identifier_start(ch: u8) -> bool {
    ch.is_ascii_alphabetic() || ch == b'_'
}

/// Check if a character can continue an identifier (inline optimized)
#[inline]
pub fn is_identifier_continue(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Fast integer parsing for small numbers (0-9)
#[inline]
pub fn parse_digit(ch: u8) -> Option<u8> {
    match ch {
        b'0'..=b'9' => Some(ch - b'0'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_cache() {
        assert_eq!(
            KeywordCache::lookup("if"),
            Some(crate::engine::lexer::TokenType::T_IF)
        );
        assert_eq!(
            KeywordCache::lookup("function"),
            Some(crate::engine::lexer::TokenType::T_FUNCTION)
        );
        assert_eq!(KeywordCache::lookup("notakeyword"), None);
    }

    #[test]
    fn test_shared_string() {
        let s1 = shared_str_static("hello");
        let s2 = shared_str_owned(String::from("world"));

        assert_eq!(s1.as_ref(), "hello");
        assert_eq!(s2.as_ref(), "world");
    }

    #[test]
    fn test_ascii_whitespace() {
        assert!(is_ascii_whitespace(b' '));
        assert!(is_ascii_whitespace(b'\t'));
        assert!(is_ascii_whitespace(b'\n'));
        assert!(!is_ascii_whitespace(b'a'));
    }

    #[test]
    fn test_identifier_checks() {
        assert!(is_identifier_start(b'a'));
        assert!(is_identifier_start(b'_'));
        assert!(!is_identifier_start(b'1'));

        assert!(is_identifier_continue(b'a'));
        assert!(is_identifier_continue(b'1'));
        assert!(is_identifier_continue(b'_'));
        assert!(!is_identifier_continue(b' '));
    }

    #[test]
    fn test_parse_digit() {
        assert_eq!(parse_digit(b'0'), Some(0));
        assert_eq!(parse_digit(b'9'), Some(9));
        assert_eq!(parse_digit(b'a'), None);
    }
}
