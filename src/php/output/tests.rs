//! Unit tests for PHP Output Buffering

use crate::php::output::{
    php_output_current_callback, php_output_end, php_output_end_clean, php_output_start,
    php_output_start_with_callback, php_output_take, php_output_take_clean, php_output_write,
    php_output_write_to_active,
};

#[test]
fn test_php_output_start() {
    let result = php_output_start();
    assert!(result.is_ok());
}

#[test]
fn test_php_output_write() {
    let _ = php_output_start();

    let result = php_output_write("Hello".as_bytes());
    assert!(result.is_ok());

    let result = php_output_write(" World".as_bytes());
    assert!(result.is_ok());
}

#[test]
fn test_php_output_get_contents() {
    let _ = php_output_start();
    let _ = php_output_write("Test output".as_bytes());

    // Get contents by ending the buffer
    let contents = php_output_end();
    assert!(contents.is_ok());
    assert_eq!(contents.unwrap(), "Test output");
}

#[test]
fn test_php_output_end() {
    let _ = php_output_start();
    let _ = php_output_write("Test".as_bytes());

    let result = php_output_end();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Test");

    // After end, no buffer should exist
    let result2 = php_output_end();
    assert!(result2.is_err());
}

#[test]
fn test_php_output_nested() {
    // Test nested output buffers
    let _ = php_output_start();
    let _ = php_output_write("Outer".as_bytes());

    let _ = php_output_start();
    let _ = php_output_write("Inner".as_bytes());

    // End inner buffer
    let inner = php_output_end();
    assert!(inner.is_ok());
    assert_eq!(inner.unwrap(), "Inner");

    // End outer buffer
    let outer = php_output_end();
    assert!(outer.is_ok());
    assert_eq!(outer.unwrap(), "Outer");
}

#[test]
fn test_php_output_callback() {
    // Clean any leftover buffers first
    let _ = php_output_end_clean();

    let _ = php_output_start_with_callback("my_callback".to_string());
    let _ = php_output_write("Hello".as_bytes());

    assert_eq!(php_output_current_callback(), Some("my_callback".to_string()));

    let (contents, callback) = php_output_take().unwrap();
    assert_eq!(contents, "Hello");
    assert_eq!(callback, Some("my_callback".to_string()));

    // No buffer left
    assert!(php_output_take().is_err());
}

#[test]
fn test_php_output_take_clean() {
    let _ = php_output_end_clean();

    let _ = php_output_start_with_callback("cb".to_string());
    let _ = php_output_write("World".as_bytes());

    let (contents, callback) = php_output_take_clean().unwrap();
    assert_eq!(contents, "World");
    assert_eq!(callback, Some("cb".to_string()));

    // Buffer still exists but is clean
    let (contents2, callback2) = php_output_take_clean().unwrap();
    assert_eq!(contents2, "");
    assert_eq!(callback2, Some("cb".to_string()));

    let _ = php_output_end_clean();
}
