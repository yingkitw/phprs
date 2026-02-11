//! Tests for the lexer module

use super::*;

#[test]
fn test_lexer_basic() {
    let mut lexer = Lexer::new("echo 'hello';");
    let token = lexer.next_token().unwrap();
    assert_eq!(token.token_type, TokenType::T_ECHO);
}

#[test]
fn test_lexer_string() {
    let mut lexer = Lexer::new("\"hello\"");
    let token = lexer.next_token().unwrap();
    assert_eq!(token.token_type, TokenType::T_CONSTANT_ENCAPSED_STRING);
    assert!(token.value.is_some());
}

#[test]
fn test_lexer_variable() {
    let mut lexer = Lexer::new("$x");
    let token = lexer.next_token().unwrap();
    assert_eq!(token.token_type, TokenType::T_VARIABLE);
}

#[test]
fn test_lexer_number() {
    let mut lexer = Lexer::new("42");
    let token = lexer.next_token().unwrap();
    assert_eq!(token.token_type, TokenType::T_LNUMBER);
}

#[test]
fn test_lexer_float() {
    let mut lexer = Lexer::new("3.14");
    let token = lexer.next_token().unwrap();
    assert_eq!(token.token_type, TokenType::T_DNUMBER);
}

#[test]
fn test_lexer_operators() {
    let mut lexer = Lexer::new("+ - * / %");
    let operators = vec![
        TokenType::T_PLUS,
        TokenType::T_MINUS,
        TokenType::T_MUL,
        TokenType::T_DIV,
        TokenType::T_MOD,
    ];

    for expected in operators {
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, expected);
    }
}

#[test]
fn test_lexer_comparison() {
    let mut lexer = Lexer::new("== != === !== < > <= >=");
    let comparisons = vec![
        TokenType::T_IS_EQUAL,
        TokenType::T_IS_NOT_EQUAL,
        TokenType::T_IS_IDENTICAL,
        TokenType::T_IS_NOT_IDENTICAL,
        TokenType::T_STRING,          // <
        TokenType::T_STRING,          // >
        TokenType::T_IS_SMALLER_OR_EQUAL,  // <=
        TokenType::T_IS_GREATER_OR_EQUAL,  // >=
    ];

    for expected in comparisons {
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, expected);
    }
}

#[test]
fn test_lexer_assignment() {
    let mut lexer = Lexer::new("= += -= *= /= .= %=");
    let assignments = vec![
        TokenType::T_EQUAL,
        TokenType::T_PLUS_EQUAL,
        TokenType::T_MINUS_EQUAL,
        TokenType::T_MUL_EQUAL,
        TokenType::T_DIV_EQUAL,
        TokenType::T_CONCAT_EQUAL,
        TokenType::T_MOD_EQUAL,
    ];

    for expected in assignments {
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, expected);
    }
}

#[test]
fn test_lexer_keywords() {
    let keywords = vec![
        ("if", TokenType::T_IF),
        ("else", TokenType::T_ELSE),
        ("while", TokenType::T_WHILE),
        ("for", TokenType::T_FOR),
        ("foreach", TokenType::T_FOREACH),
        ("function", TokenType::T_FUNCTION),
        ("return", TokenType::T_RETURN),
        ("class", TokenType::T_CLASS),
        ("echo", TokenType::T_ECHO),
    ];

    for (keyword, expected) in keywords {
        let mut lexer = Lexer::new(keyword);
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, expected);
    }
}

#[test]
fn test_lexer_line_tracking() {
    let mut lexer = Lexer::new("echo\n'test'\n;");
    let token1 = lexer.next_token().unwrap();
    assert_eq!(token1.lineno, 1);

    let token2 = lexer.next_token().unwrap();
    assert_eq!(token2.lineno, 2);

    let token3 = lexer.next_token().unwrap();
    assert_eq!(token3.lineno, 3);
}

#[test]
fn test_lexer_offset_tracking() {
    let mut lexer = Lexer::new("echo test");
    let token1 = lexer.next_token().unwrap();
    assert_eq!(token1.offset, 0);

    let token2 = lexer.next_token().unwrap();
    assert_eq!(token2.offset, 5); // Start of "test" (after "echo ")
}
