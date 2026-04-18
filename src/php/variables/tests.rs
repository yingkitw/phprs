//! Unit tests for PHP Variable Handling

use crate::php::variables::{normalize_variable_name, php_register_variable};
use crate::engine::hash::hash_find;
use crate::engine::string::string_init;
use crate::engine::types::PhpArray;

#[test]
fn test_normalize_variable_name() {
    // normalize_variable_name only replaces spaces and dots with underscores
    // It does NOT convert to lowercase
    assert_eq!(normalize_variable_name("test"), "test");
    assert_eq!(normalize_variable_name("TEST"), "TEST");
    assert_eq!(normalize_variable_name("Test_Var"), "Test_Var");
    assert_eq!(normalize_variable_name("test-var"), "test-var");
    assert_eq!(normalize_variable_name("test var"), "test_var");
    assert_eq!(normalize_variable_name("test.var"), "test_var");
}

#[test]
fn test_php_register_variable() {
    let mut symbol_table = PhpArray::new();
    crate::engine::hash::hash_init(&mut symbol_table, 8);

    let name = "test_var";
    let value = "42";

    let result = php_register_variable(name, value, &mut symbol_table);
    assert!(result.is_ok());

    let key = string_init("test_var", false);
    let found = hash_find(&symbol_table, &key);
    assert!(found.is_some());

    if let Some(zval) = found {
        let str_result = crate::engine::operators::zval_get_string(zval);
        assert_eq!(str_result.as_str(), "42");
    }
}

#[test]
fn test_php_register_variable_normalization() {
    let mut symbol_table = PhpArray::new();
    crate::engine::hash::hash_init(&mut symbol_table, 8);

    // Register with name containing spaces (should be normalized to underscores)
    let result = php_register_variable("test var", "100", &mut symbol_table);
    assert!(result.is_ok());

    // Should be findable with normalized name (underscores)
    let key = string_init("test_var", false);
    let found = hash_find(&symbol_table, &key);
    assert!(found.is_some());

    if let Some(zval) = found {
        let str_result = crate::engine::operators::zval_get_string(zval);
        assert_eq!(str_result.as_str(), "100");
    }
}

#[test]
fn test_php_register_variable_string() {
    let mut symbol_table = PhpArray::new();
    crate::engine::hash::hash_init(&mut symbol_table, 8);

    let result = php_register_variable("message", "hello", &mut symbol_table);
    assert!(result.is_ok());

    let key = string_init("message", false);
    let found = hash_find(&symbol_table, &key);
    assert!(found.is_some());

    if let Some(zval) = found {
        let str_result = crate::engine::operators::zval_get_string(zval);
        assert_eq!(str_result.as_str(), "hello");
    }
}
