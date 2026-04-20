//! Error Handling Example
//!
//! Demonstrates PHP error handling system

use phprs::errors::{error_at_line, php_error, set_error_handler, ErrorType, PhpError};

fn main() {
    println!("=== phprs Error Handling Example ===\n");

    // Set a custom error handler
    set_error_handler(|error: &PhpError| {
        println!(
            "[Custom Handler] {}: {}",
            match error.r#type {
                ErrorType::Error => "ERROR",
                ErrorType::Warning => "WARNING",
                ErrorType::Notice => "NOTICE",
                _ => "OTHER",
            },
            error.message
        );
    });

    // Report different types of errors
    println!("Reporting errors:\n");

    php_error(ErrorType::Notice, "This is a notice");
    php_error(ErrorType::Warning, "This is a warning");
    php_error(ErrorType::Error, "This is an error");

    // Report error with file and line
    println!("\nError with location:");
    error_at_line(
        ErrorType::Parse,
        "example.php",
        42,
        "Parse error: unexpected token",
    );

    // Reset to default handler (by setting None)
    // Note: In a real implementation, we'd have a way to reset handler
    println!("\nUsing default error handler:");
    // The default handler will be used if we don't set a custom one
}
