//! Unit tests for PHP Output Buffering

use crate::php::output::{php_output_end, php_output_start, php_output_write};

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
