//! Token readers - specialized functions for reading different token types

use super::tokens::TokenType;

/// Result of reading a token
pub type ReadResult = Result<(TokenType, String), String>;

/// Read a number (integer or float)
pub fn read_number(input: &[u8], position: &mut usize) -> ReadResult {
    let start = *position;
    let mut has_dot = false;

    while let Some(&ch) = input.get(*position) {
        if ch.is_ascii_digit() {
            *position += 1;
        } else if ch == b'.' && !has_dot {
            has_dot = true;
            *position += 1;
        } else {
            break;
        }
    }

    let value = String::from_utf8_lossy(&input[start..*position]).to_string();
    let token_type = if has_dot {
        TokenType::T_DNUMBER
    } else {
        TokenType::T_LNUMBER
    };

    Ok((token_type, value))
}

/// Read a string (single or double quoted)
pub fn read_string(input: &[u8], position: &mut usize) -> ReadResult {
    let quote = input.get(*position).ok_or("Unexpected EOF")?;
    if *quote != b'"' && *quote != b'\'' {
        return Err("Not a string".to_string());
    }

    *position += 1; // Skip opening quote
    let mut value = String::new();
    let mut escaped = false;

    while let Some(&ch) = input.get(*position) {
        if escaped {
            match ch {
                b'n' => value.push('\n'),
                b't' => value.push('\t'),
                b'r' => value.push('\r'),
                b'\\' => value.push('\\'),
                b'"' => value.push('"'),
                b'\'' => value.push('\''),
                _ => value.push(ch as char),
            }
            escaped = false;
            *position += 1;
        } else if ch == b'\\' {
            escaped = true;
            *position += 1;
        } else if ch == *quote {
            *position += 1; // Skip closing quote
            break;
        } else {
            value.push(ch as char);
            *position += 1;
        }
    }

    Ok((TokenType::T_CONSTANT_ENCAPSED_STRING, value))
}

/// Read an identifier or keyword
pub fn read_identifier(input: &[u8], position: &mut usize) -> ReadResult {
    let start = *position;

    while let Some(&ch) = input.get(*position) {
        if ch.is_ascii_alphanumeric() || ch == b'_' {
            *position += 1;
        } else {
            break;
        }
    }

    let value = String::from_utf8_lossy(&input[start..*position]).to_string();
    Ok((TokenType::T_STRING, value))
}

/// Read a variable ($identifier)
pub fn read_variable(input: &[u8], position: &mut usize) -> ReadResult {
    if input.get(*position) != Some(&b'$') {
        return Err("Not a variable".to_string());
    }

    *position += 1; // Skip $
    let start = *position;

    // Read identifier after $
    while let Some(&ch) = input.get(*position) {
        if ch.is_ascii_alphanumeric() || ch == b'_' {
            *position += 1;
        } else {
            break;
        }
    }

    let value = String::from_utf8_lossy(&input[start..*position]).to_string();
    Ok((TokenType::T_VARIABLE, format!("${value}")))
}

/// Skip whitespace
#[allow(dead_code)]
pub fn skip_whitespace(input: &[u8], position: &mut usize) {
    while let Some(&ch) = input.get(*position) {
        if ch.is_ascii_whitespace() {
            *position += 1;
        } else {
            break;
        }
    }
}

/// Skip whitespace and track line numbers
pub fn skip_whitespace_with_lineno(input: &[u8], position: &mut usize, lineno: &mut u32) {
    while let Some(&ch) = input.get(*position) {
        if ch.is_ascii_whitespace() {
            if ch == b'\n' {
                *lineno += 1;
            }
            *position += 1;
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_number() {
        let input = b"123 456";
        let mut pos = 0;
        let (token, value) = read_number(input, &mut pos).unwrap();
        assert_eq!(token, TokenType::T_LNUMBER);
        assert_eq!(value, "123");
        assert_eq!(pos, 3);
    }

    #[test]
    fn test_read_float() {
        let input = b"3.14";
        let mut pos = 0;
        let (token, value) = read_number(input, &mut pos).unwrap();
        assert_eq!(token, TokenType::T_DNUMBER);
        assert_eq!(value, "3.14");
    }

    #[test]
    fn test_read_string() {
        let input = b"\"hello\"";
        let mut pos = 0;
        let (token, value) = read_string(input, &mut pos).unwrap();
        assert_eq!(token, TokenType::T_CONSTANT_ENCAPSED_STRING);
        assert_eq!(value, "hello");
    }

    #[test]
    fn test_read_identifier() {
        let input = b"variable123";
        let mut pos = 0;
        let (token, value) = read_identifier(input, &mut pos).unwrap();
        assert_eq!(token, TokenType::T_STRING);
        assert_eq!(value, "variable123");
    }

    #[test]
    fn test_read_variable() {
        let input = b"$var_name";
        let mut pos = 0;
        let (token, value) = read_variable(input, &mut pos).unwrap();
        assert_eq!(token, TokenType::T_VARIABLE);
        assert_eq!(value, "$var_name");
    }

    #[test]
    fn test_skip_whitespace() {
        let input = b"   \t\n  x";
        let mut pos = 0;
        skip_whitespace(input, &mut pos);
        // After skipping 3 spaces, 1 tab, 1 newline, and 2 spaces = 7 characters
        assert_eq!(pos, 7);
        assert_eq!(input[pos], b'x');
    }
}
