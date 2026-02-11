//! PHP Global Variables
//!
//! Migrated from main/php_globals.h
//!
//! This module manages PHP's global state

#[cfg(test)]
mod tests;

use crate::engine::types::PhpArray;
use std::sync::{Mutex, OnceLock};

/// PHP core globals
pub struct PhpCoreGlobals {
    /// Superglobals
    pub get: PhpArray,
    pub post: PhpArray,
    pub cookie: PhpArray,
    pub server: PhpArray,
    pub env: PhpArray,
    pub files: PhpArray,
    pub request: PhpArray,

    /// Other globals
    pub error_reporting: u32,
    pub display_errors: bool,
    pub log_errors: bool,
    pub error_log: Option<String>,
}

impl PhpCoreGlobals {
    pub fn new() -> Self {
        Self {
            get: PhpArray::new(),
            post: PhpArray::new(),
            cookie: PhpArray::new(),
            server: PhpArray::new(),
            env: PhpArray::new(),
            files: PhpArray::new(),
            request: PhpArray::new(),
            error_reporting: 0,
            display_errors: true,
            log_errors: false,
            error_log: None,
        }
    }
}

impl Default for PhpCoreGlobals {
    fn default() -> Self {
        Self::new()
    }
}

/// Get PHP core globals (thread-safe wrapper)
fn php_core_globals() -> &'static Mutex<PhpCoreGlobals> {
    static GLOBALS: OnceLock<Mutex<PhpCoreGlobals>> = OnceLock::new();
    GLOBALS.get_or_init(|| Mutex::new(PhpCoreGlobals::new()))
}

/// Initialize PHP globals
pub fn php_init_globals() {
    let _ = php_core_globals();
}

/// Get PHP core globals mutex (for direct access)
pub fn php_get_globals() -> &'static Mutex<PhpCoreGlobals> {
    php_core_globals()
}

/// Set error reporting level
pub fn php_set_error_reporting(level: u32) {
    let mut globals = php_core_globals().lock().unwrap();
    globals.error_reporting = level;
}

/// Get error reporting level
pub fn php_get_error_reporting() -> u32 {
    let globals = php_core_globals().lock().unwrap();
    globals.error_reporting
}
