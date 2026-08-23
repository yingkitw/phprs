//! Token readers - specialized functions for reading different token types

use super::tokens::TokenType;

/// Result of reading a token
pub type ReadResult = Result<(TokenType, String), String>;

/// Read a number (integer or float)
pub fn read_number(input: &[u8], position: &mut usize) -> ReadResult {
    let start = *position;

    // Prefixed literals: hex (0x), binary (0b), octal (0o)
    if input.get(start) == Some(&b'0')
        && matches!(
            input.get(start + 1),
            Some(b'x' | b'X' | b'b' | b'B' | b'o' | b'O')
        )
    {
        let radix = match input[start + 1] {
            b'x' | b'X' => 16,
            b'b' | b'B' => 2,
            _ => 8,
        };
        *position += 2;
        let digits_start = *position;
        while let Some(&ch) = input.get(*position) {
            let valid = match radix {
                16 => ch.is_ascii_hexdigit(),
                8 => (b'0'..=b'7').contains(&ch),
                _ => ch == b'0' || ch == b'1',
            };
            if valid {
                *position += 1;
            } else {
                break;
            }
        }
        let digits = &input[digits_start..*position];
        if digits.is_empty() {
            return Err("Invalid numeric literal: missing digits".to_string());
        }
        let text = String::from_utf8_lossy(digits).replace('_', "");
        let value = i64::from_str_radix(&text, radix)
            .map_err(|e| format!("Invalid numeric literal: {e}"))?
            .to_string();
        return Ok((TokenType::T_LNUMBER, value));
    }

    // Legacy octal: leading 0 followed by octal digits only
    if input.get(start) == Some(&b'0')
        && input
            .get(start + 1)
            .is_some_and(|&c| (b'0'..=b'7').contains(&c))
    {
        *position += 1;
        while let Some(&ch) = input.get(*position) {
            if (b'0'..=b'7').contains(&ch) {
                *position += 1;
            } else {
                break;
            }
        }
        let text = String::from_utf8_lossy(&input[start..*position])
            .replace('_', "");
        let value =
            i64::from_str_radix(&text, 8).map_err(|e| format!("Invalid octal literal: {e}"))?;
        return Ok((TokenType::T_LNUMBER, value.to_string()));
    }

    let mut has_dot = false;

    while let Some(&ch) = input.get(*position) {
        if ch.is_ascii_digit() || ch == b'_' {
            *position += 1;
        } else if ch == b'.' && !has_dot {
            has_dot = true;
            *position += 1;
        } else {
            break;
        }
    }

    let raw = String::from_utf8_lossy(&input[start..*position]).replace('_', "");
    let token_type = if has_dot {
        TokenType::T_DNUMBER
    } else {
        TokenType::T_LNUMBER
    };

    Ok((token_type, raw))
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
