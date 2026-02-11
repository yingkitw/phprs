//! Filesystem Operations Example
//!
//! Demonstrates PHP filesystem functions

use phprs::filesystem::{
    php_file_exists, php_file_get_contents, php_filesize, php_is_dir, php_is_file, php_scandir,
};

fn main() {
    println!("=== PHP-RS Filesystem Example ===\n");

    // Check if current directory exists
    let current_dir = ".";
    if php_file_exists(current_dir) {
        println!("Current directory exists");

        if php_is_dir(current_dir) {
            println!("Current directory is a directory");
        }
    }

    // Scan directory
    match php_scandir(current_dir) {
        Ok(entries) => {
            println!("\nDirectory entries (first 10):");
            for (i, entry) in entries.iter().take(10).enumerate() {
                println!("  [{}] {}", i, entry);
            }
            println!("  ... ({} total entries)", entries.len());
        }
        Err(e) => {
            println!("Error scanning directory: {}", e);
        }
    }

    // Check for Cargo.toml
    let cargo_toml = "Cargo.toml";
    if php_file_exists(cargo_toml) {
        println!("\n{} exists", cargo_toml);

        if php_is_file(cargo_toml) {
            println!("{} is a file", cargo_toml);

            match php_filesize(cargo_toml) {
                Ok(size) => println!("{} size: {} bytes", cargo_toml, size),
                Err(e) => println!("Error getting file size: {}", e),
            }

            match php_file_get_contents(cargo_toml) {
                Ok(content) => {
                    let preview = content.lines().take(5).collect::<Vec<_>>().join("\n");
                    println!("\n{} content preview:\n{}", cargo_toml, preview);
                }
                Err(e) => println!("Error reading file: {}", e),
            }
        }
    } else {
        println!("\n{} does not exist", cargo_toml);
    }
}
