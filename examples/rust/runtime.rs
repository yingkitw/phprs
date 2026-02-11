//! PHP Runtime Example
//!
//! Demonstrates PHP runtime functions and initialization

use phprs::globals::{php_get_error_reporting, php_init_globals, php_set_error_reporting};
use phprs::runtime::{
    php_build_date, php_module_shutdown, php_module_startup, php_version, php_version_id,
};

fn main() {
    println!("=== PHP-RS Runtime Example ===\n");

    // Display PHP version information
    println!("PHP Version Information:");
    println!("  Version: {}", php_version());
    println!("  Version ID: {}", php_version_id());
    println!("  Build Date: {}", php_build_date());

    // Initialize PHP globals
    println!("\nInitializing PHP globals...");
    php_init_globals();
    println!("  PHP globals initialized");

    // Configure error reporting
    let error_level = 0b11111111; // E_ALL equivalent
    php_set_error_reporting(error_level);
    println!(
        "  Error reporting level set to: {}",
        php_get_error_reporting()
    );

    // Module startup/shutdown
    println!("\nModule lifecycle:");
    match php_module_startup() {
        Ok(_) => {
            println!("  Module started successfully");

            // Simulate some work here

            match php_module_shutdown() {
                Ok(_) => println!("  Module shut down successfully"),
                Err(e) => println!("  Error during shutdown: {}", e),
            }
        }
        Err(e) => println!("  Error during startup: {}", e),
    }
}
