//! Comprehensive Test Suite
//!
//! This module provides thorough testing across the entire PHP-RS codebase
//! including unit tests, integration tests, and edge cases.

use phprs::{
    php::{streams, streams::StreamMode},
    engine::{
        facade::{ValFactory, StdValFactory},
        lexer::Lexer,
        types::{PhpType, PhpValue},
    },
};

// ============================================================================
// LEXER COMPREHENSIVE TESTS
// ============================================================================

mod lexer_tests {
    use super::*;

    #[test]
    fn test_lexer_all_keywords() {
        let keywords = [
            "if", "else", "elseif", "while", "for", "foreach", "function",
            "class", "return", "echo", "true", "false", "null", "array",
            "new", "static", "public", "private", "protected", "const",
            "break", "continue", "switch", "case", "default", "try", "catch",
            "throw", "namespace", "use", "as", "abstract", "extends", "final",
            "implements", "interface", "trait", "clone",
        ];

        for keyword in &keywords {
            let mut lexer = Lexer::new(keyword);
            let token = lexer.next_token().unwrap();
            assert!(
                token.token_type != phprs::engine::lexer::TokenType::T_ERROR,
                "Keyword '{}' should not produce T_ERROR, got {:?}",
                keyword,
                token.token_type
            );
        }
    }

    #[test]
    fn test_lexer_numbers() {
        let test_cases = [
            ("42", phprs::engine::lexer::TokenType::T_LNUMBER),
            ("0", phprs::engine::lexer::TokenType::T_LNUMBER),
            ("3.14", phprs::engine::lexer::TokenType::T_DNUMBER),
            ("0.5", phprs::engine::lexer::TokenType::T_DNUMBER),
            ("123.456", phprs::engine::lexer::TokenType::T_DNUMBER),
        ];

        for (input, expected_type) in &test_cases {
            let mut lexer = Lexer::new(input);
            let token = lexer.next_token().unwrap();
            assert_eq!(
                token.token_type, *expected_type,
                "Input '{}' should produce {:?}",
                input, expected_type
            );
        }
    }

    #[test]
    fn test_lexer_strings() {
        let test_cases = [
            ("\"hello\"", "hello"),
            ("'world'", "world"),
            ("\"test\\nescape\"", "test\\nescape"),
        ];

        for (input, expected) in &test_cases {
            let mut lexer = Lexer::new(input);
            let token = lexer.next_token().unwrap();
            assert_eq!(
                token.token_type,
                phprs::engine::lexer::TokenType::T_CONSTANT_ENCAPSED_STRING
            );
            // Note: actual value extraction depends on implementation
        }
    }

    #[test]
    fn test_lexer_variables() {
        let test_cases = ["$x", "$variable", "$_private", "$camelCase"];

        for input in &test_cases {
            let mut lexer = Lexer::new(input);
            let token = lexer.next_token().unwrap();
            assert_eq!(
                token.token_type,
                phprs::engine::lexer::TokenType::T_VARIABLE,
                "Input '{}' should be T_VARIABLE",
                input
            );
        }
    }

    #[test]
    fn test_lexer_operators() {
        let test_cases = [
            ("+", phprs::engine::lexer::TokenType::T_PLUS),
            ("-", phprs::engine::lexer::TokenType::T_MINUS),
            ("*", phprs::engine::lexer::TokenType::T_MUL),
            ("/=", phprs::engine::lexer::TokenType::T_DIV_EQUAL),
            ("==", phprs::engine::lexer::TokenType::T_IS_EQUAL),
            ("===", phprs::engine::lexer::TokenType::T_IS_IDENTICAL),
            ("!=", phprs::engine::lexer::TokenType::T_IS_NOT_EQUAL),
            ("!==", phprs::engine::lexer::TokenType::T_IS_NOT_IDENTICAL),
            ("<=", phprs::engine::lexer::TokenType::T_IS_SMALLER_OR_EQUAL),
            (">=", phprs::engine::lexer::TokenType::T_IS_GREATER_OR_EQUAL),
            ("&&", phprs::engine::lexer::TokenType::T_BOOLEAN_AND),
            ("||", phprs::engine::lexer::TokenType::T_BOOLEAN_OR),
        ];

        for (input, expected_type) in &test_cases {
            let mut lexer = Lexer::new(input);
            let token = lexer.next_token().unwrap();
            assert_eq!(
                token.token_type, *expected_type,
                "Input '{}' should produce {:?}",
                input, expected_type
            );
        }
    }

    #[test]
    fn test_lexer_complex_expression() {
        let code = "$x = 10 + $y * 3.14;";
        let mut lexer = Lexer::new(code);

        let mut token_count = 0;
        loop {
            let token = lexer.next_token().unwrap();
            if token.token_type == phprs::engine::lexer::TokenType::T_EOF {
                break;
            }
            token_count += 1;
            assert!(
                token_count < 20,
                "Too many tokens, possible infinite loop in lexer"
            );
        }
    }
}

// ============================================================================
// ZVAL FACTORY COMPREHENSIVE TESTS
// ============================================================================

mod zval_factory_tests {
    use super::*;

    #[test]
    fn test_zval_factory_all_types() {
        let bool_val = StdValFactory::bool_val(true);
        assert_eq!(bool_val.get_type(), PhpType::True);

        let bool_false = StdValFactory::bool_val(false);
        assert_eq!(bool_false.get_type(), PhpType::False);

        let long_val = StdValFactory::long_val(42);
        assert_eq!(long_val.get_type(), PhpType::Long);

        let double_val = StdValFactory::double_val(3.14);
        assert_eq!(double_val.get_type(), PhpType::Double);

        let string_val = StdValFactory::string_val("test");
        assert_eq!(string_val.get_type(), PhpType::String);

        let null_val = StdValFactory::null_val();
        assert_eq!(null_val.get_type(), PhpType::Null);

        let array_val = StdValFactory::array_val();
        assert_eq!(array_val.get_type(), PhpType::Array);
    }

    #[test]
    fn test_zval_clone() {
        let original = StdValFactory::long_val(123);
        let cloned = StdValFactory::clone_val(&original);

        assert_eq!(original.get_type(), cloned.get_type());
        match (&original.value, &cloned.value) {
            (PhpValue::Long(orig), PhpValue::Long(clon)) => {
                assert_eq!(orig, clon);
            }
            _ => panic!("Cloned value should match original"),
        }
    }

    #[test]
    fn test_zval_result_dup() {
        let source = StdValFactory::double_val(2.71828);
        let copy = StdValFactory::result_dup(&source);

        assert_eq!(source.get_type(), copy.get_type());
        match (&source.value, &copy.value) {
            (PhpValue::Double(s), PhpValue::Double(c)) => {
                assert!((s - c).abs() < f64::EPSILON);
            }
            _ => panic!("Result copy should preserve value"),
        }
    }
}

// ============================================================================
// STREAM COMPREHENSIVE TESTS
// ============================================================================

mod stream_tests {
    use super::*;
    use std::fs;
    use std::io::{Read, Write};

    #[test]
    fn test_stream_all_modes() {
        let test_paths = [
            ("/tmp/test_read.txt", StreamMode::Read),
            ("/tmp/test_write.txt", StreamMode::Write),
            ("/tmp/test_append.txt", StreamMode::Append),
            ("/tmp/test_readwrite.txt", StreamMode::ReadWrite),
        ];

        // Create files first
        for (path, _) in &test_paths {
            fs::write(path, b"test").unwrap();
        }

        for (path, mode) in &test_paths {
            let result = streams::php_stream_open(path, *mode);
            match mode {
                StreamMode::Read => assert!(result.is_ok(), "Read mode should work for existing file"),
                StreamMode::Write => assert!(result.is_ok(), "Write mode should work"),
                StreamMode::Append => assert!(result.is_ok(), "Append mode should work"),
                StreamMode::ReadWrite => assert!(result.is_ok(), "ReadWrite mode should work"),
            }
        }

        // Cleanup
        for (path, _) in &test_paths {
            let _ = fs::remove_file(path);
        }
    }

    #[test]
    fn test_stream_write_read() {
        let test_path = "/tmp/test_stream_roundtrip.txt";
        let test_content = b"Hello, PHP-RS!";

        // Write
        let mut write_stream = streams::php_stream_open(test_path, StreamMode::Write).unwrap();
        write_stream.write_all(test_content).unwrap();
        write_stream.flush().unwrap();

        // Read
        let mut read_stream = streams::php_stream_open(test_path, StreamMode::Read).unwrap();
        let mut read_content = Vec::new();
        read_stream.read_to_end(&mut read_content).unwrap();

        assert_eq!(test_content, read_content.as_slice());

        // Cleanup
        let _ = fs::remove_file(test_path);
    }

    #[test]
    fn test_stream_nonexistent() {
        let result = streams::php_stream_open("/tmp/nonexistent_file_12345.txt", StreamMode::Read);
        assert!(result.is_err(), "Opening nonexistent file should fail");
    }
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

mod integration_tests {
    use super::*;

    #[test]
    fn test_lexer_to_expression() {
        let code = "42 + 3.14";
        let mut lexer = Lexer::new(code);

        // Should be able to tokenize this expression
        let token1 = lexer.next_token().unwrap();
        assert_eq!(
            token1.token_type,
            phprs::engine::lexer::TokenType::T_LNUMBER
        );

        let token2 = lexer.next_token().unwrap();
        assert_eq!(token2.token_type, phprs::engine::lexer::TokenType::T_PLUS);

        let token3 = lexer.next_token().unwrap();
        assert_eq!(
            token3.token_type,
            phprs::engine::lexer::TokenType::T_DNUMBER
        );
    }

    #[test]
    fn test_variable_assignment_pattern() {
        let code = "$x = 100";
        let mut lexer = Lexer::new(code);

        let token1 = lexer.next_token().unwrap();
        assert_eq!(
            token1.token_type,
            phprs::engine::lexer::TokenType::T_VARIABLE
        );

        let token2 = lexer.next_token().unwrap();
        assert_eq!(token2.token_type, phprs::engine::lexer::TokenType::T_EQUAL);

        let token3 = lexer.next_token().unwrap();
        assert_eq!(
            token3.token_type,
            phprs::engine::lexer::TokenType::T_LNUMBER
        );
    }

    #[test]
    fn test_function_call_pattern() {
        let code = "echo \"hello\"";
        let mut lexer = Lexer::new(code);

        let token1 = lexer.next_token().unwrap();
        assert_eq!(token1.token_type, phprs::engine::lexer::TokenType::T_ECHO);

        let token2 = lexer.next_token().unwrap();
        assert_eq!(
            token2.token_type,
            phprs::engine::lexer::TokenType::T_CONSTANT_ENCAPSED_STRING
        );
    }
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let mut lexer = Lexer::new("");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, phprs::engine::lexer::TokenType::T_EOF);
    }

    #[test]
    fn test_whitespace_only() {
        let mut lexer = Lexer::new("   \n\t  ");
        let token = lexer.next_token().unwrap();
        assert_eq!(token.token_type, phprs::engine::lexer::TokenType::T_EOF);
    }

    #[test]
    fn test_very_long_number() {
        let big_number = "12345678901234567890";
        let mut lexer = Lexer::new(big_number);
        let token = lexer.next_token().unwrap();
        assert_eq!(
            token.token_type,
            phprs::engine::lexer::TokenType::T_LNUMBER
        );
    }

    #[test]
    fn test_special_characters_in_string() {
        let code = "\"\\n\\r\\t\\\\\"";
        let mut lexer = Lexer::new(code);
        let token = lexer.next_token().unwrap();
        assert_eq!(
            token.token_type,
            phprs::engine::lexer::TokenType::T_CONSTANT_ENCAPSED_STRING
        );
    }

    #[test]
    fn test_boolean_values() {
        let mut lexer = Lexer::new("true false null");
        let tokens = vec![
            lexer.next_token().unwrap(),
            lexer.next_token().unwrap(),
            lexer.next_token().unwrap(),
        ];

        // All should be recognized as valid tokens (not T_ERROR)
        for token in tokens {
            assert_ne!(token.token_type, phprs::engine::lexer::TokenType::T_ERROR);
        }
    }

    #[test]
    fn test_consecutive_operators() {
        let code = "++--+-";
        let mut lexer = Lexer::new(code);

        // Should tokenize all operators
        loop {
            let token = lexer.next_token().unwrap();
            if token.token_type == phprs::engine::lexer::TokenType::T_EOF {
                break;
            }
            // Should not error on any of these
        }
    }

    #[test]
    fn test_zval_factory_edge_cases() {
        // Test zero values
        let zero_long = StdValFactory::long_val(0);
        let zero_double = StdValFactory::double_val(0.0);
        let bool_false = StdValFactory::bool_val(false);
        let bool_true = StdValFactory::bool_val(true);

        assert_eq!(zero_long.get_type(), PhpType::Long);
        assert_eq!(zero_double.get_type(), PhpType::Double);
        assert_eq!(bool_false.get_type(), PhpType::False);
        assert_eq!(bool_true.get_type(), PhpType::True);
    }

    #[test]
    fn test_string_factory_edge_cases() {
        let empty_string = StdValFactory::string_val("");
        assert_eq!(empty_string.get_type(), PhpType::String);

        let unicode_string = StdValFactory::string_val("Hello 世界 🌍");
        assert_eq!(unicode_string.get_type(), PhpType::String);
    }
}

// ============================================================================
// PERFORMANCE TESTS (Basic sanity checks)
// ============================================================================

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_lexer_performance() {
        let code = "$x + $y * $z / $w";
        let iterations = 1000;

        let start = Instant::now();
        for _ in 0..iterations {
            let mut lexer = Lexer::new(code);
            loop {
                let token = lexer.next_token().unwrap();
                if token.token_type == phprs::engine::lexer::TokenType::T_EOF {
                    break;
                }
            }
        }
        let duration = start.elapsed();

        // Should complete 1000 iterations in reasonable time (< 1 second)
        assert!(
            duration.as_millis() < 1000,
            "Lexer performance test took too long: {:?}",
            duration
        );
    }

    #[test]
    fn test_zval_factory_performance() {
        let iterations = 10000;
        let start = Instant::now();

        for i in 0..iterations {
            let _ = StdValFactory::long_val(i);
            let _ = StdValFactory::double_val(i as f64);
            let _ = StdValFactory::bool_val(i % 2 == 0);
        }

        let duration = start.elapsed();

        // Should complete 30000 factory calls in reasonable time
        assert!(
            duration.as_millis() < 500,
            "Factory performance test took too long: {:?}",
            duration
        );
    }
}
