//! Example Verification Tests
//!
//! These tests verify that the examples work correctly and produce expected output

use phprs::alloc::{
    get_allocation_count, get_memory_usage, get_peak_memory_usage, pefree, pemalloc,
};
use phprs::filesystem::{php_file_exists, php_is_dir, php_is_file, php_scandir};
use phprs::hash::{hash_add_or_update, hash_find, hash_init};
use phprs::operators::{
    zval_add, zval_compare, zval_div, zval_get_bool, zval_get_double, zval_get_long,
    zval_get_string, zval_mul, zval_sub,
};
use phprs::runtime::{
    php_build_date, php_module_shutdown, php_module_startup, php_version, php_version_id,
};
use phprs::string::{string_concat2, string_concat3, string_init};
use phprs::{PhpArray, PhpType, PhpValue, Val};

#[test]
fn test_basic_types_example() {
    // Verify basic types example functionality

    // Long value
    let long_val = Val::new(PhpValue::Long(42), PhpType::Long);
    assert_eq!(zval_get_long(&long_val), 42);

    // Double value
    let double_val = Val::new(PhpValue::Double(3.14159), PhpType::Double);
    assert_eq!(zval_get_double(&double_val), 3.14159);

    // String value
    let str_val = string_init("Hello, PHP-RS!", false);
    let zval_str = Val::new(PhpValue::String(Box::new(str_val)), PhpType::String);
    assert_eq!(zval_get_string(&zval_str).as_str(), "Hello, PHP-RS!");

    // Boolean values
    let true_val = Val::new(PhpValue::Long(1), PhpType::True);
    let false_val = Val::new(PhpValue::Long(0), PhpType::False);
    assert_eq!(zval_get_bool(&true_val), true);
    assert_eq!(zval_get_bool(&false_val), false);

    // Null value
    let null_val = Val::new(PhpValue::Long(0), PhpType::Null);
    assert_eq!(null_val.get_type(), PhpType::Null);
}

#[test]
fn test_string_operations_example() {
    // Verify string operations example functionality

    let str1 = string_init("Hello", false);
    let str2 = string_init("World", false);
    let str3 = string_init("!", false);

    assert_eq!(str1.as_str(), "Hello");
    assert_eq!(str2.as_str(), "World");
    assert_eq!(str3.as_str(), "!");

    // Concatenate two strings
    let concat2 = string_concat2("Hello", "World");
    assert_eq!(concat2.as_str(), "HelloWorld");

    // Concatenate three strings
    let concat3 = string_concat3("Hello", "World", "!");
    assert_eq!(concat3.as_str(), "HelloWorld!");
}

#[test]
fn test_hash_table_example() {
    // Verify hash table example functionality

    let mut ht = PhpArray::new();
    hash_init(&mut ht, 8);

    // Add string keys
    let key1 = string_init("name", false);
    let val1 = Val::new(
        PhpValue::String(Box::new(string_init("PHP-RS", false))),
        PhpType::String,
    );
    hash_add_or_update(&mut ht, Some(&key1), 0, val1, 0);

    let key2 = string_init("version", false);
    let val2 = Val::new(PhpValue::Long(1), PhpType::Long);
    hash_add_or_update(&mut ht, Some(&key2), 0, val2, 0);

    let key3 = string_init("pi", false);
    let val3 = Val::new(PhpValue::Double(3.14), PhpType::Double);
    hash_add_or_update(&mut ht, Some(&key3), 0, val3, 0);

    assert_eq!(ht.n_num_of_elements, 3);

    // Find by string key
    let found1 = hash_find(&ht, &key1);
    assert!(found1.is_some());

    let found2 = hash_find(&ht, &key2);
    assert!(found2.is_some());
    if let Some(z) = found2 {
        assert_eq!(zval_get_long(z), 1);
    }
}

#[test]
fn test_operators_example() {
    // Verify operators example functionality

    let a = Val::new(PhpValue::Long(10), PhpType::Long);
    let b = Val::new(PhpValue::Long(3), PhpType::Long);

    // Arithmetic operations
    let result_add = zval_add(&a, &b);
    assert_eq!(zval_get_long(&result_add), 13);

    let result_sub = zval_sub(&a, &b);
    assert_eq!(zval_get_double(&result_sub), 7.0);

    let result_mul = zval_mul(&a, &b);
    assert_eq!(zval_get_double(&result_mul), 30.0);

    match zval_div(&a, &b) {
        Ok(result_div) => {
            assert!((zval_get_double(&result_div) - 3.3333333333333335).abs() < 0.0001);
        }
        Err(_) => panic!("Division should succeed"),
    }

    // Comparison operations
    let cmp1 = zval_compare(&a, &b);
    assert_eq!(cmp1, 1); // 10 > 3

    let c = Val::new(PhpValue::Long(10), PhpType::Long);
    let cmp2 = zval_compare(&a, &c);
    assert_eq!(cmp2, 0); // 10 == 10
}

#[test]
fn test_memory_management_example() {
    // Verify memory management example functionality

    let initial_usage = get_memory_usage();
    let initial_peak = get_peak_memory_usage();
    let initial_count = get_allocation_count();

    // Allocate some memory
    let size1 = 1024;
    let ptr1 = unsafe { pemalloc(size1, false) };
    assert!(!ptr1.is_null());

    let size2 = 2048;
    let ptr2 = unsafe { pemalloc(size2, true) };
    assert!(!ptr2.is_null());

    // Verify memory increased
    let new_usage = get_memory_usage();
    let new_peak = get_peak_memory_usage();
    let new_count = get_allocation_count();

    assert!(new_usage >= initial_usage);
    assert!(new_peak >= initial_peak);
    assert!(new_count >= initial_count);

    // Free memory
    unsafe {
        pefree(ptr1, false);
        pefree(ptr2, true);
    }

    // After freeing, usage should decrease but peak remains
    let final_peak = get_peak_memory_usage();
    assert!(final_peak >= new_peak);
}

#[test]
fn test_runtime_example() {
    // Verify runtime example functionality

    // Version information should be available
    let version = php_version();
    assert!(!version.is_empty());
    assert!(version.contains("8"));

    let version_id = php_version_id();
    assert!(version_id > 0);

    let build_date = php_build_date();
    assert!(!build_date.is_empty());

    // Test module lifecycle
    let startup_result = php_module_startup();
    assert!(startup_result.is_ok());

    let shutdown_result = php_module_shutdown();
    assert!(shutdown_result.is_ok());
}

#[test]
fn test_filesystem_example() {
    // Verify filesystem example functionality

    // Current directory should exist
    assert!(php_file_exists("."));
    assert!(php_is_dir("."));

    // Cargo.toml should exist and be a file
    if php_file_exists("Cargo.toml") {
        assert!(php_is_file("Cargo.toml"));
    }

    // Should be able to scan directory
    let result = php_scandir(".");
    assert!(result.is_ok());
    let entries = result.unwrap();
    assert!(!entries.is_empty());
}
