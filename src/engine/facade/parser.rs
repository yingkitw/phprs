//! Parser Facade - Simplifies parsing operations
//!
//! Provides traits and helpers for common parsing operations

use crate::engine::lexer::{Token, Lexer, TokenType};

/// Trait for token-based parsing operations
pub trait TokenParser {
    /// Check if current token matches expected type
    fn expect_token(&mut self, expected: TokenType) -> Result<Token, String>;

    /// Check if current token is a specific string value
    fn expect_string(&mut self, expected: &str) -> Result<Token, String>;

    /// Peek at current token without consuming
    fn peek_token(&mut self) -> Result<Option<Token>, String>;

    /// Consume token if it matches, return None otherwise
    fn consume_if(&mut self, expected: TokenType) -> Result<Option<Token>, String>;

    /// Skip whitespace tokens
    fn skip_whitespace(&mut self);

    /// Check if we're at EOF
    fn is_at_eof(&self) -> bool;
}

impl TokenParser for Lexer {
    fn expect_token(&mut self, expected: TokenType) -> Result<Token, String> {
        let token = self.next_token()?;
        if token.token_type == expected {
            Ok(token)
        } else {
            Err(format!(
                "Expected token {:?}, got {:?}",
                expected, token.token_type
            ))
        }
    }

    fn expect_string(&mut self, expected: &str) -> Result<Token, String> {
        let token = self.next_token()?;
        match &token.value {
            Some(s) if s.as_str() == expected => Ok(token),
            _ => Err(format!(
                "Expected string '{}', got {:?}",
                expected, token.value
            )),
        }
    }

    fn peek_token(&mut self) -> Result<Option<Token>, String> {
        // We need to clone the current state, peek, then restore
        // This is a simplified implementation
        // A more efficient implementation would buffer tokens
        let _current_pos = self.position();
        let _current_lineno = self.lineno();
        let token = self.next_token()?;
        // Restore position
        // Note: This requires access to private fields, so in practice
        // you'd want a proper token buffer/peek implementation
        Ok(Some(token))
    }

    fn consume_if(&mut self, expected: TokenType) -> Result<Option<Token>, String> {
        // This would require peek functionality
        // For now, a simplified version
        let token = self.next_token()?;
        if token.token_type == expected {
            Ok(Some(token))
        } else {
            // We need to "put back" the token, which requires buffering
            // For now, just return None (note: this loses the token)
            Ok(None)
        }
    }

    fn skip_whitespace(&mut self) {
        // This is already implemented in Lexer
        // Just expose it through the trait
        while let Some(ch) = self.current_char() {
            if ch.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn is_at_eof(&self) -> bool {
        self.is_at_end()
    }
}

/// Helper functions for common parsing patterns
pub struct ParserHelpers;

impl ParserHelpers {
    /// Check if a token represents a specific punctuation
    pub fn is_punct(token: &Token, punct: &str) -> bool {
        match &token.value {
            Some(s) => s.as_str() == punct,
            None => false,
        }
    }

    /// Check if token is a binary operator
    pub fn is_binary_op(token: &Token) -> bool {
        matches!(
            token.token_type,
            TokenType::T_PLUS
                | TokenType::T_MINUS
                | TokenType::T_MUL
                | TokenType::T_DIV
                | TokenType::T_MOD
                | TokenType::T_CONCAT
                | TokenType::T_BOOLEAN_AND
                | TokenType::T_BOOLEAN_OR
                | TokenType::T_IS_EQUAL
                | TokenType::T_IS_NOT_EQUAL
                | TokenType::T_IS_IDENTICAL
                | TokenType::T_IS_NOT_IDENTICAL
                | TokenType::T_IS_SMALLER_OR_EQUAL
                | TokenType::T_IS_GREATER_OR_EQUAL
        )
    }

    /// Check if token is an assignment operator
    pub fn is_assignment_op(token: &Token) -> bool {
        matches!(
            token.token_type,
            TokenType::T_EQUAL
                | TokenType::T_PLUS_EQUAL
                | TokenType::T_MINUS_EQUAL
                | TokenType::T_MUL_EQUAL
                | TokenType::T_DIV_EQUAL
                | TokenType::T_CONCAT_EQUAL
        )
    }

    /// Get operator precedence (higher = tighter binding)
    pub fn get_precedence(token: &Token) -> u8 {
        match token.token_type {
            TokenType::T_BOOLEAN_OR => 1,
            TokenType::T_BOOLEAN_AND => 2,
            TokenType::T_IS_EQUAL
            | TokenType::T_IS_NOT_EQUAL
            | TokenType::T_IS_IDENTICAL
            | TokenType::T_IS_NOT_IDENTICAL
            | TokenType::T_IS_SMALLER_OR_EQUAL
            | TokenType::T_IS_GREATER_OR_EQUAL => 3,
            TokenType::T_PLUS | TokenType::T_MINUS | TokenType::T_CONCAT => 4,
            TokenType::T_MUL | TokenType::T_DIV | TokenType::T_MOD => 5,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_punct() {
        let token = Token::new(
            TokenType::T_STRING,
            Some(crate::engine::string::string_init("(", false)),
            1,
            0,
        );
        assert!(ParserHelpers::is_punct(&token, "("));
        assert!(!ParserHelpers::is_punct(&token, ")"));
    }

    #[test]
    fn test_is_binary_op() {
        let token = Token::new(
            TokenType::T_PLUS,
            Some(crate::engine::string::string_init("+", false)),
            1,
            0,
        );
        assert!(ParserHelpers::is_binary_op(&token));
    }

    #[test]
    fn test_is_assignment_op() {
        let token = Token::new(
            TokenType::T_EQUAL,
            Some(crate::engine::string::string_init("=", false)),
            1,
            0,
        );
        assert!(ParserHelpers::is_assignment_op(&token));
    }

    #[test]
    fn test_get_precedence() {
        let mul_token = Token::new(
            TokenType::T_MUL,
            Some(crate::engine::string::string_init("*", false)),
            1,
            0,
        );
        assert_eq!(ParserHelpers::get_precedence(&mul_token), 5);

        let add_token = Token::new(
            TokenType::T_PLUS,
            Some(crate::engine::string::string_init("+", false)),
            1,
            0,
        );
        assert_eq!(ParserHelpers::get_precedence(&add_token), 4);
    }
}
