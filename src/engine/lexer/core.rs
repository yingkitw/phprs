//! Core lexer implementation

use super::tokens::{Token, TokenType};
use super::keywords::keyword_to_token;
use super::readers::{read_number, read_string, read_identifier, read_variable, skip_whitespace_with_lineno};
use crate::engine::string::string_init;

/// Lexer state
pub struct Lexer {
    input: Vec<u8>,
    position: usize,
    lineno: u32,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.as_bytes().to_vec(),
            position: 0,
            lineno: 1,
        }
    }

    pub fn from_bytes(input: Vec<u8>) -> Self {
        Self {
            input,
            position: 0,
            lineno: 1,
        }
    }

    /// Get current character
    pub fn current_char(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }

    /// Advance position
    pub fn advance(&mut self) {
        if let Some(ch) = self.current_char() {
            if ch == b'\n' {
                self.lineno += 1;
            }
        }
        self.position += 1;
    }

    /// Get current line number
    pub fn lineno(&self) -> u32 {
        self.lineno
    }

    /// Get current position
    pub fn position(&self) -> usize {
        self.position
    }

    /// Check if at end of input
    pub fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }

    /// Get next token
    pub fn next_token(&mut self) -> Result<Token, String> {
        loop {
            skip_whitespace_with_lineno(&self.input, &mut self.position, &mut self.lineno);

            let offset = self.position;
            let lineno = self.lineno;

            if self.position >= self.input.len() {
                return Ok(Token::eof(lineno, offset));
            }

            let ch = self.current_char().ok_or("Unexpected EOF")?;

            // Check for comments before calling read_token
            if ch == b'/' {
                self.advance();
                if self.current_char() == Some(b'/') {
                    // Single line comment - skip to end of line
                    while let Some(ch) = self.current_char() {
                        if ch == b'\n' {
                            break;
                        }
                        self.advance();
                    }
                    continue; // Loop to get next token after comment
                } else if self.current_char() == Some(b'*') {
                    // Multi-line comment - skip to */
                    self.advance();
                    while let Some(ch) = self.current_char() {
                        if ch == b'*' && self.input.get(self.position + 1) == Some(&b'/') {
                            self.advance(); // Skip *
                            self.advance(); // Skip /
                            break;
                        }
                        self.advance();
                    }
                    continue; // Loop to get next token after comment
                }
                // Not a comment, put back the /
                self.position = offset;
            }

            let (token_type, value_str) = self.read_token(self.current_char().ok_or("Unexpected EOF")?)?;

            let value = if value_str.is_empty() {
                None
            } else {
                Some(string_init(&value_str, false))
            };

            return Ok(Token::new(token_type, value, lineno, offset));
        }
    }

    /// Read a token based on the current character
    fn read_token(&mut self, ch: u8) -> Result<(TokenType, String), String> {
        match ch {
            b'#' => {
                self.advance();
                if self.current_char() == Some(b'[') {
                    self.advance();
                    Ok((TokenType::T_ATTRIBUTE, "#[".to_string()))
                } else {
                    Ok((TokenType::T_STRING, "#".to_string()))
                }
            }
            b'$' => {
                let _start = self.position;
                self.read_with(|lexer| read_variable(&lexer.input, &mut lexer.position))
            }
            b'"' | b'\'' => {
                self.read_with(|lexer| read_string(&lexer.input, &mut lexer.position))
            }
            b'0'..=b'9' => {
                self.read_with(|lexer| read_number(&lexer.input, &mut lexer.position))
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                self.read_with(|lexer| {
                    let (_token_type, value) = read_identifier(&lexer.input, &mut lexer.position)?;
                    let kw_token = keyword_to_token(&value);
                    Ok((kw_token, value))
                })
            }
            b'+' => {
                self.advance();
                if self.current_char() == Some(b'=') {
                    self.advance();
                    Ok((TokenType::T_PLUS_EQUAL, "+=".to_string()))
                } else if self.current_char() == Some(b'+') {
                    self.advance();
                    Ok((TokenType::T_INC, "++".to_string()))
                } else {
                    Ok((TokenType::T_PLUS, "+".to_string()))
                }
            }
            b'-' => {
                self.advance();
                if self.current_char() == Some(b'=') {
                    self.advance();
                    Ok((TokenType::T_MINUS_EQUAL, "-=".to_string()))
                } else if self.current_char() == Some(b'>') {
                    self.advance();
                    Ok((TokenType::T_OBJECT_OPERATOR, "->".to_string()))
                } else if self.current_char() == Some(b'-') {
                    self.advance();
                    Ok((TokenType::T_DEC, "--".to_string()))
                } else {
                    Ok((TokenType::T_MINUS, "-".to_string()))
                }
            }
            b'*' => {
                self.advance();
                if self.current_char() == Some(b'=') {
                    self.advance();
                    Ok((TokenType::T_MUL_EQUAL, "*=".to_string()))
                } else {
                    Ok((TokenType::T_MUL, "*".to_string()))
                }
            }
            b'/' => {
                self.advance();
                if self.current_char() == Some(b'=') {
                    self.advance();
                    Ok((TokenType::T_DIV_EQUAL, "/=".to_string()))
                } else {
                    Ok((TokenType::T_DIV, "/".to_string()))
                }
            }
            b'%' => {
                self.advance();
                if self.current_char() == Some(b'=') {
                    self.advance();
                    Ok((TokenType::T_MOD_EQUAL, "%=".to_string()))
                } else {
                    Ok((TokenType::T_MOD, "%".to_string()))
                }
            }
            b'=' => {
                self.advance();
                if self.current_char() == Some(b'=') {
                    self.advance();
                    // Check for === (three equals)
                    if self.current_char() == Some(b'=') {
                        self.advance();
                        Ok((TokenType::T_IS_IDENTICAL, "===" .to_string()))
                    } else {
                        Ok((TokenType::T_IS_EQUAL, "==".to_string()))
                    }
                } else if self.current_char() == Some(b'>') {
                    self.advance();
                    Ok((TokenType::T_DOUBLE_ARROW, "=>".to_string()))
                } else {
                    Ok((TokenType::T_EQUAL, "=".to_string()))
                }
            }
            b'<' => {
                self.advance();
                // Check for PHP opening tag: <?php or <?
                if self.current_char() == Some(b'?') {
                    self.advance();
                    // Check if it's <?php
                    if self.position + 2 < self.input.len()
                        && &self.input[self.position..self.position + 3] == b"php"
                    {
                        self.position += 3;
                        Ok((TokenType::T_OPEN_TAG, "<?php".to_string()))
                    } else {
                        // Just <?
                        Ok((TokenType::T_OPEN_TAG_WITH_ECHO, "<?".to_string()))
                    }
                } else if self.current_char() == Some(b'=') {
                    self.advance();
                    Ok((TokenType::T_IS_SMALLER_OR_EQUAL, "<=".to_string()))
                } else if self.current_char() == Some(b'<') {
                    self.advance();
                    Ok((TokenType::T_SL, "<<".to_string()))
                } else {
                    // Plain < character - not a token in PHP
                    Ok((TokenType::T_STRING, "<".to_string()))
                }
            }
            b'>' => {
                self.advance();
                if self.current_char() == Some(b'=') {
                    self.advance();
                    Ok((TokenType::T_IS_GREATER_OR_EQUAL, ">=".to_string()))
                } else if self.current_char() == Some(b'>') {
                    self.advance();
                    Ok((TokenType::T_SR, ">>".to_string()))
                } else {
                    // Plain > character - not a token in PHP
                    Ok((TokenType::T_STRING, ">".to_string()))
                }
            }
            b'!' => {
                self.advance();
                if self.current_char() == Some(b'=') {
                    self.advance();
                    // Check for !== (not identical)
                    if self.current_char() == Some(b'=') {
                        self.advance();
                        Ok((TokenType::T_IS_NOT_IDENTICAL, "!==".to_string()))
                    } else {
                        Ok((TokenType::T_IS_NOT_EQUAL, "!=".to_string()))
                    }
                } else {
                    Ok((TokenType::T_BOOLEAN_NOT, "!".to_string()))
                }
            }
            b'&' => {
                self.advance();
                if self.current_char() == Some(b'&') {
                    self.advance();
                    Ok((TokenType::T_BOOLEAN_AND, "&&".to_string()))
                } else {
                    Ok((
                        TokenType::T_AMPERSAND_FOLLOWED_BY_VAR_OR_VARARG,
                        "&".to_string(),
                    ))
                }
            }
            b'|' => {
                self.advance();
                if self.current_char() == Some(b'|') {
                    self.advance();
                    Ok((TokenType::T_BOOLEAN_OR, "||".to_string()))
                } else {
                    Ok((TokenType::T_OR_EQUAL, "|".to_string()))
                }
            }
            b'.' => {
                self.advance();
                if self.current_char() == Some(b'=') {
                    self.advance();
                    Ok((TokenType::T_CONCAT_EQUAL, ".=".to_string()))
                } else {
                    Ok((TokenType::T_CONCAT, ".".to_string()))
                }
            }
            b'?' => {
                self.advance();
                if self.current_char() == Some(b'?') {
                    self.advance();
                    if self.current_char() == Some(b'=') {
                        self.advance();
                        Ok((TokenType::T_COALESCE_EQUAL, "??=".to_string()))
                    } else {
                        Ok((TokenType::T_COALESCE, "??".to_string()))
                    }
                } else if self.current_char() == Some(b'-') && self.input.get(self.position + 1) == Some(&b'>') {
                    self.advance(); // skip -
                    self.advance(); // skip >
                    Ok((TokenType::T_NULLSAFE_OBJECT_OPERATOR, "?->".to_string()))
                } else {
                    // Bare ? used for ternary
                    Ok((TokenType::T_STRING, "?".to_string()))
                }
            }
            b':' => {
                self.advance();
                if self.current_char() == Some(b':') {
                    self.advance();
                    Ok((TokenType::T_PAAMAYIM_NEKUDOTAYIM, "::".to_string()))
                } else {
                    Ok((TokenType::T_STRING, ":".to_string()))
                }
            }
            b'\\' => {
                self.advance();
                Ok((TokenType::T_NS_SEPARATOR, "\\".to_string()))
            }
            b'(' | b')' | b'{' | b'}' | b';' | b',' | b'[' | b']' => {
                self.advance();
                let ch_str = String::from_utf8_lossy(&[ch]).to_string();
                Ok((TokenType::T_STRING, ch_str))
            }
            _ => {
                self.advance();
                Err(format!("Unknown char: {}", ch as char))
            }
        }
    }

    /// Helper to execute a read function that modifies position
    fn read_with<F>(&mut self, mut f: F) -> Result<(TokenType, String), String>
    where
        F: FnMut(&mut Self) -> Result<(TokenType, String), String>,
    {
        f(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic() {
        let mut lexer = Lexer::new("echo 'hello';");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::T_ECHO);

        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::T_CONSTANT_ENCAPSED_STRING);
    }

    #[test]
    fn test_lexer_expression() {
        let mut lexer = Lexer::new("$x + 10");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::T_VARIABLE);

        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::T_PLUS);

        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, TokenType::T_LNUMBER);
    }
}
