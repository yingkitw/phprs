//! Integration Tests
//!
//! Tests that verify interactions between multiple modules

use phprs::hash::{hash_add_or_update, hash_find, hash_init};
use phprs::operators::{zval_add, zval_get_long, zval_get_string};
use phprs::string::string_init;
use phprs::variables::php_register_variable;
use phprs::{PhpArray, PhpType, PhpValue, Val};

#[test]
fn test_string_to_hash_to_operator() {
    // Create a string
    let str_val = string_init("hello", false);
    let zval_str = Val::new(PhpValue::String(Box::new(str_val)), PhpType::String);

    // Store in hash table
    let mut ht = PhpArray::new();
    hash_init(&mut ht, 8);
    let key = string_init("greeting", false);
    hash_add_or_update(&mut ht, Some(&key), 0, zval_str, 0);

    // Retrieve and use in operation
    let found = hash_find(&ht, &key);
    assert!(found.is_some());

    let world_str = string_init("world", false);
    let zval_world = Val::new(PhpValue::String(Box::new(world_str)), PhpType::String);

    if let Some(greeting) = found {
        let result = zval_add(greeting, &zval_world);
        let result_str = zval_get_string(&result);
        assert_eq!(result_str.as_str(), "helloworld");
    }
}

#[test]
fn test_variable_registration_and_retrieval() {
    // Register variable
    let mut symbol_table = PhpArray::new();
    hash_init(&mut symbol_table, 8);

    let result = php_register_variable("counter", "42", &mut symbol_table);
    assert!(result.is_ok());

    // Retrieve and use in arithmetic
    let key = string_init("counter", false);
    let found = hash_find(&symbol_table, &key);
    assert!(found.is_some());

    if let Some(counter) = found {
        // counter is a string "42", need to convert to number first
        let counter_num = zval_get_long(counter);
        let increment = Val::new(PhpValue::Long(1), PhpType::Long);
        let counter_zval = Val::new(PhpValue::Long(counter_num), PhpType::Long);
        let result = zval_add(&counter_zval, &increment);
        assert_eq!(zval_get_long(&result), 43);
    }
}

#[test]
fn test_hash_table_with_multiple_types() {
    let mut ht = PhpArray::new();
    hash_init(&mut ht, 8);

    // Add different types
    let key1 = string_init("int_val", false);
    let val1 = Val::new(PhpValue::Long(10), PhpType::Long);
    hash_add_or_update(&mut ht, Some(&key1), 0, val1, 0);

    let key2 = string_init("double_val", false);
    let val2 = Val::new(PhpValue::Double(3.14), PhpType::Double);
    hash_add_or_update(&mut ht, Some(&key2), 0, val2, 0);

    let key3 = string_init("string_val", false);
    let str_val = string_init("test", false);
    let val3 = Val::new(PhpValue::String(Box::new(str_val)), PhpType::String);
    hash_add_or_update(&mut ht, Some(&key3), 0, val3, 0);

    // Verify all can be retrieved
    assert!(hash_find(&ht, &key1).is_some());
    assert!(hash_find(&ht, &key2).is_some());
    assert!(hash_find(&ht, &key3).is_some());

    // Verify types are correct
    if let Some(z) = hash_find(&ht, &key1) {
        assert_eq!(zval_get_long(z), 10);
    }

    if let Some(z) = hash_find(&ht, &key3) {
        let result_str = zval_get_string(z);
        assert_eq!(result_str.as_str(), "test");
    }
}

#[test]
fn test_operator_chain() {
    // Test chaining multiple operations
    let a = Val::new(PhpValue::Long(10), PhpType::Long);
    let b = Val::new(PhpValue::Long(5), PhpType::Long);
    let c = Val::new(PhpValue::Long(2), PhpType::Long);

    // (a + b) * c
    let sum = zval_add(&a, &b);
    let product = phprs::operators::zval_mul(&sum, &c);

    assert_eq!(zval_get_long(&sum), 15);
    assert_eq!(phprs::operators::zval_get_double(&product), 30.0);
}
