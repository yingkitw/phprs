//! Error Handling Example
//!
//! Demonstrates PHP error handling system

use phprs::errors::{
    php_error, php_error_at_line, php_set_error_handler, PhpError, PhpErrorType,
};

fn main() {
    println!("=== phprs Error Handling Example ===\n");

    // Set a custom error handler
    php_set_error_handler(|error: &PhpError| {
        println!(
            "[Custom Handler] {}: {}",
            match error.r#type {
                PhpErrorType::Error => "ERROR",
                PhpErrorType::Warning => "WARNING",
                PhpErrorType::Notice => "NOTICE",
                _ => "OTHER",
            },
            error.message
        );
    });

    // Report different types of errors
    println!("Reporting errors:\n");

    php_error(PhpErrorType::Notice, "This is a notice");
    php_error(PhpErrorType::Warning, "This is a warning");
    php_error(PhpErrorType::Error, "This is an error");

    // Report error with file and line
    println!("\nError with location:");
    php_error_at_line(
        PhpErrorType::Parse,
        "example.php",
        42,
        "Parse error: unexpected token",
    );

    // Reset to default handler (by setting None)
    // Note: In a real implementation, we'd have a way to reset handler
    println!("\nUsing default error handler:");
    // The default handler will be used if we don't set a custom one
}
