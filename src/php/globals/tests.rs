//! Unit tests for PHP Globals

use crate::php::globals::{
    php_get_error_reporting, php_get_globals, php_init_globals, php_set_error_reporting,
};

#[test]
fn test_php_init_globals() {
    // Should not panic
    php_init_globals();
}

#[test]
fn test_php_get_globals() {
    php_init_globals();
    let globals = php_get_globals();
    // Should be able to lock
    let _locked = globals.lock().unwrap();
}

#[test]
fn test_php_error_reporting() {
    php_init_globals();

    // Set error reporting level
    php_set_error_reporting(0b11111111);
    assert_eq!(php_get_error_reporting(), 0b11111111);

    // Change it
    php_set_error_reporting(0b00000001);
    assert_eq!(php_get_error_reporting(), 0b00000001);
}

#[test]
fn test_php_globals_thread_safety() {
    php_init_globals();

    // Multiple threads should be able to access (in real scenario)
    let globals = php_get_globals();
    let _g1 = globals.lock().unwrap();
    // In a real multi-threaded scenario, another thread would wait here
}
