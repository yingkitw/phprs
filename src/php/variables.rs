//! PHP variable handling
//!
//! Migrated from main/php_variables.h and main/php_variables.c

#[cfg(test)]
mod tests;

use crate::engine::hash::hash_add_or_update;
use crate::engine::string::string_init;
use crate::engine::types::{PhpArray, PhpType, PhpValue, Val};

/// Register a variable in the symbol table
///
/// This function registers a variable with the given name and value
/// in the PHP symbol table (typically $_GET, $_POST, $_COOKIE, etc.)
pub fn php_register_variable(
    name: &str,
    value: &str,
    symbol_table: &mut PhpArray,
) -> Result<(), String> {
    // Normalize variable name (replace spaces and dots with underscores)
    let normalized_name = normalize_variable_name(name);

    // Create zval for the value
    let str_val = string_init(value, false);
    let zval = Val::new(PhpValue::String(Box::new(str_val)), PhpType::String);

    // Create key string
    let key = string_init(&normalized_name, false);

    // Add to hash table
    let result = hash_add_or_update(
        symbol_table,
        Some(&key),
        0,
        zval,
        0, // HASH_UPDATE
    );

    match result {
        crate::engine::types::PhpResult::Success => Ok(()),
        crate::engine::types::PhpResult::Failure => Err("Failed to register variable".to_string()),
    }
}

/// Normalize variable name
///
/// Replaces spaces and dots with underscores, handles array notation
fn normalize_variable_name(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut chars = name.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            ' ' | '.' => result.push('_'),
            '[' => {
                result.push('[');
                break; // Rest is array index
            }
            _ => result.push(ch),
        }
    }

    // Copy remaining characters (array index)
    while let Some(ch) = chars.next() {
        result.push(ch);
    }

    result
}

/// Register a variable with safe string (binary-safe)
pub fn php_register_variable_safe(
    name: &str,
    value: &[u8],
    symbol_table: &mut PhpArray,
) -> Result<(), String> {
    let value_str = String::from_utf8_lossy(value);
    php_register_variable(name, &value_str, symbol_table)
}
