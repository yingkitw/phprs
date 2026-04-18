//! Error Handling Tests
//!
//! Tests for error conditions and error handling

use phprs::errors::{
    php_error, error_at_line, set_error_handler, PhpError, ErrorType,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

static ERROR_COUNT: AtomicU32 = AtomicU32::new(0);
/// Serialize tests that touch the process-wide error handler (otherwise parallel `cargo test` races).
static ERROR_HANDLER_TESTS_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn test_error_reporting() {
    let _guard = ERROR_HANDLER_TESTS_LOCK.lock().unwrap();
    // Reset counter
    ERROR_COUNT.store(0, Ordering::Relaxed);

    // Set custom handler
    set_error_handler(|_error: &PhpError| {
        ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
    });

    // Report errors
    php_error(ErrorType::Notice, "Test notice");
    php_error(ErrorType::Warning, "Test warning");
    php_error(ErrorType::Error, "Test error");

    // Handler should have been called for each reported error.
    let count = ERROR_COUNT.load(Ordering::Relaxed);
    assert!(count >= 3);
}

#[test]
fn test_error_with_location() {
    let _guard = ERROR_HANDLER_TESTS_LOCK.lock().unwrap();
    ERROR_COUNT.store(0, Ordering::Relaxed);

    set_error_handler(|error: &PhpError| {
        ERROR_COUNT.fetch_add(1, Ordering::Relaxed);
        // Verify filename and line number if present
        if let Some(ref filename) = error.filename {
            assert_eq!(filename, "test.php");
        }
        if error.lineno > 0 {
            assert_eq!(error.lineno, 42);
        }
    });

    error_at_line(ErrorType::Parse, "test.php", 42, "Parse error");

    let count = ERROR_COUNT.load(Ordering::Relaxed);
    assert!(count >= 1);
}

#[test]
fn test_different_error_types() {
    let _guard = ERROR_HANDLER_TESTS_LOCK.lock().unwrap();
    let error_types = vec![
        ErrorType::Error,
        ErrorType::Warning,
        ErrorType::Notice,
        ErrorType::Parse,
        ErrorType::CompileError,
        ErrorType::UserError,
    ];

    for error_type in error_types {
        // Should not panic
        php_error(error_type, "Test error");
    }
}

#[test]
fn test_error_handler_replacement() {
    let _guard = ERROR_HANDLER_TESTS_LOCK.lock().unwrap();
    // Test that we can set and replace error handlers
    set_error_handler(|_error: &PhpError| {
        // First handler
    });

    php_error(ErrorType::Notice, "Test");

    // Replace handler
    set_error_handler(|_error: &PhpError| {
        // New handler
    });

    php_error(ErrorType::Warning, "Test");

    // Test passes if no panic occurs
}
