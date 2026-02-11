//! Unit tests for PHP Runtime

use crate::php::runtime::{
    php_build_date, php_module_shutdown, php_module_startup, php_version, php_version_id,
};

#[test]
fn test_php_version() {
    let version = php_version();
    assert!(!version.is_empty());
    assert!(version.contains("8"));
}

#[test]
fn test_php_version_id() {
    let version_id = php_version_id();
    assert!(version_id >= 80000); // Should be 8.x.x
}

#[test]
fn test_php_build_date() {
    let build_date = php_build_date();
    assert!(!build_date.is_empty());
}

#[test]
fn test_php_module_startup() {
    let result = php_module_startup();
    assert!(result.is_ok());
}

#[test]
fn test_php_module_shutdown() {
    // Startup first
    let _ = php_module_startup();

    let result = php_module_shutdown();
    assert!(result.is_ok());
}

#[test]
fn test_php_module_lifecycle() {
    // Test full lifecycle
    let startup_result = php_module_startup();
    assert!(startup_result.is_ok());

    // Do some work here (simulated)

    let shutdown_result = php_module_shutdown();
    assert!(shutdown_result.is_ok());
}
