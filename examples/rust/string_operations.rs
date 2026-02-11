//! String Operations Example
//!
//! Demonstrates PHP string handling and operations

use phprs::string::{string_concat2, string_concat3, string_init};

fn main() {
    println!("=== phprs String Operations Example ===\n");

    // Create strings
    let str1 = string_init("Hello", false);
    let str2 = string_init("World", false);
    let str3 = string_init("!", false);

    println!("String 1: {}", str1.as_str());
    println!("String 2: {}", str2.as_str());
    println!("String 3: {}", str3.as_str());
    println!("String 1 hash: {}", str1.h);

    // Concatenate two strings (using &str)
    let concat2 = string_concat2("Hello", "World");
    println!("\nConcatenation (2 strings): {}", concat2.as_str());

    // Concatenate three strings (using &str)
    let concat3 = string_concat3("Hello", "World", "!");
    println!("Concatenation (3 strings): {}", concat3.as_str());

    // Demonstrate string hashing
    println!("\nString hashes:");
    println!("  'Hello': {}", str1.h);
    println!("  'World': {}", str2.h);
    println!("  'HelloWorld': {}", concat2.h);
}
