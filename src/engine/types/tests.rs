//! Unit tests for types

use crate::engine::string::string_init;
use crate::engine::types::*;

#[test]
fn test_zval_new() {
    let zval = Val::new(PhpValue::Long(42), PhpType::Long);
    assert_eq!(zval.get_type(), PhpType::Long);
}

#[test]
fn test_zval_get_type() {
    let zval_long = Val::new(PhpValue::Long(10), PhpType::Long);
    assert_eq!(zval_long.get_type(), PhpType::Long);

    let zval_double = Val::new(PhpValue::Double(3.14), PhpType::Double);
    assert_eq!(zval_double.get_type(), PhpType::Double);

    let str_val = string_init("test", false);
    let zval_string = Val::new(PhpValue::String(Box::new(str_val)), PhpType::String);
    assert_eq!(zval_string.get_type(), PhpType::String);
}

#[test]
fn test_string_new() {
    let s = PhpString::new("hello", false);
    assert_eq!(s.as_str(), "hello");
    assert_eq!(s.len, 5);
}

#[test]
fn test_string_hash() {
    let s1 = PhpString::new("test", false);
    let s2 = PhpString::new("test", false);
    assert_eq!(s1.h, s2.h);
}

#[test]
fn test_array_new() {
    let arr = PhpArray::new();
    assert_eq!(arr.n_num_of_elements, 0);
    assert_eq!(arr.n_table_size, 0);
}

#[test]
fn test_refcounted_new() {
    let refcounted = RefcountedH::new(0x00000001); // GC_STRING
    assert_eq!(
        refcounted
            .refcount
            .load(std::sync::atomic::Ordering::Relaxed),
        1
    );
}

#[test]
fn test_php_value_variants() {
    // Test all PhpValue variants can be created
    let _long = PhpValue::Long(42);
    let _double = PhpValue::Double(3.14);
    let _string = PhpValue::String(Box::new(string_init("test", false)));
    let _array = PhpValue::Array(Box::new(PhpArray::new()));
    let _ptr = PhpValue::Ptr(std::ptr::null_mut());
}

#[test]
fn test_php_type_values() {
    // Verify PhpType enum values
    assert_eq!(PhpType::Undef as u8, 0);
    assert_eq!(PhpType::Null as u8, 1);
    assert_eq!(PhpType::False as u8, 2);
    assert_eq!(PhpType::True as u8, 3);
    assert_eq!(PhpType::Long as u8, 4);
    assert_eq!(PhpType::Double as u8, 5);
    assert_eq!(PhpType::String as u8, 6);
    assert_eq!(PhpType::Array as u8, 7);
}
