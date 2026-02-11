//! Edge Case Tests
//!
//! Tests for boundary conditions, error cases, and unusual inputs

use phprs::hash::{hash_add_or_update, hash_find, hash_init};
use phprs::operators::{zval_div, zval_get_long, zval_get_string};
use phprs::string::string_init;
use phprs::{PhpArray, PhpType, PhpValue, Val};

#[test]
fn test_empty_string() {
    let empty = string_init("", false);
    assert_eq!(empty.as_str(), "");
    assert_eq!(empty.len, 0);
}

#[test]
fn test_very_long_string() {
    let long_str = "a".repeat(10000);
    let zstr = string_init(&long_str, false);
    assert_eq!(zstr.len, 10000);
    assert_eq!(zstr.as_str(), long_str);
}

#[test]
fn test_hash_table_empty() {
    let mut ht = PhpArray::new();
    hash_init(&mut ht, 8);
    assert_eq!(ht.n_num_of_elements, 0);
}

#[test]
fn test_hash_table_single_element() {
    let mut ht = PhpArray::new();
    hash_init(&mut ht, 8);

    let key = string_init("only", false);
    let val = Val::new(PhpValue::Long(1), PhpType::Long);
    hash_add_or_update(&mut ht, Some(&key), 0, val, 0);

    assert_eq!(ht.n_num_of_elements, 1);
}

#[test]
fn test_hash_table_duplicate_keys() {
    let mut ht = PhpArray::new();
    hash_init(&mut ht, 8);

    let key = string_init("key", false);
    let val1 = Val::new(PhpValue::Long(1), PhpType::Long);
    let val2 = Val::new(PhpValue::Long(2), PhpType::Long);

    // First add
    hash_add_or_update(&mut ht, Some(&key), 0, val1, 0);

    // Update with same key
    hash_add_or_update(&mut ht, Some(&key), 0, val2, 0);

    // Should have only one element
    assert_eq!(ht.n_num_of_elements, 1);

    // Value should be updated
    if let Some(z) = hash_find(&ht, &key) {
        assert_eq!(zval_get_long(z), 2);
    }
}

#[test]
fn test_division_by_zero() {
    let a = Val::new(PhpValue::Long(10), PhpType::Long);
    let b = Val::new(PhpValue::Long(0), PhpType::Long);

    let result = zval_div(&a, &b);
    assert!(result.is_err());
}

#[test]
fn test_division_very_small_numbers() {
    let a = Val::new(PhpValue::Double(0.0001), PhpType::Double);
    let b = Val::new(PhpValue::Double(0.0002), PhpType::Double);

    let result = zval_div(&a, &b);
    assert!(result.is_ok());
    let quotient = result.unwrap();
    assert!(phprs::operators::zval_get_double(&quotient) > 0.0);
}

#[test]
fn test_negative_numbers() {
    let a = Val::new(PhpValue::Long(-10), PhpType::Long);
    let b = Val::new(PhpValue::Long(5), PhpType::Long);

    let result = phprs::operators::zval_add(&a, &b);
    assert_eq!(zval_get_long(&result), -5);
}

#[test]
fn test_unicode_strings() {
    let unicode = string_init("Hello 世界 🌍", false);
    assert!(unicode.len > 0);
    assert_eq!(unicode.as_str(), "Hello 世界 🌍");
}

#[test]
fn test_special_characters_in_string() {
    let special = string_init("test\n\t\r\"'\\", false);
    assert_eq!(special.as_str(), "test\n\t\r\"'\\");
}

#[test]
fn test_hash_table_resize() {
    let mut ht = PhpArray::new();
    hash_init(&mut ht, 4); // Small initial size

    // Add more elements than initial size to trigger resize
    for i in 0..10 {
        let key = string_init(&format!("key{}", i), false);
        let val = Val::new(PhpValue::Long(i), PhpType::Long);
        hash_add_or_update(&mut ht, Some(&key), 0, val, 0);
    }

    assert_eq!(ht.n_num_of_elements, 10);

    // Verify all can still be found
    for i in 0..10 {
        let key = string_init(&format!("key{}", i), false);
        let found = hash_find(&ht, &key);
        assert!(found.is_some());
        if let Some(z) = found {
            assert_eq!(zval_get_long(z), i);
        }
    }
}

#[test]
fn test_null_value() {
    let null_val = Val::new(PhpValue::Long(0), PhpType::Null);
    assert_eq!(null_val.get_type(), PhpType::Null);

    // Null with Long(0) value converts to "0" string
    let str_result = zval_get_string(&null_val);
    // The implementation converts Long(0) to "0" string
    assert_eq!(str_result.as_str(), "0");
}

#[test]
fn test_boolean_values() {
    let true_val = Val::new(PhpValue::Long(1), PhpType::True);
    let false_val = Val::new(PhpValue::Long(0), PhpType::False);

    assert_eq!(true_val.get_type(), PhpType::True);
    assert_eq!(false_val.get_type(), PhpType::False);
}
