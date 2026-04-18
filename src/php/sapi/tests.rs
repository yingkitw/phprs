//! Unit tests for PHP SAPI

use crate::php::sapi::{
    php_cli_shutdown, php_cli_startup, sapi_shutdown, sapi_startup, SapiModule,
};

#[test]
fn test_sapi_module_new() {
    let module = SapiModule::new("test");
    assert_eq!(module.name, "test");
    assert_eq!(module.pretty_name, "test");
}

#[test]
fn test_sapi_module_startup() {
    let mut module = SapiModule::new("test");
    let result = module.startup();
    assert!(result.is_ok());
}

#[test]
fn test_sapi_module_shutdown() {
    let mut module = SapiModule::new("test");
    let _ = module.startup();

    let result = module.shutdown();
    assert!(result.is_ok());
}

#[test]
fn test_sapi_module_add_header() {
    let mut module = SapiModule::new("test");
    let _ = module.startup();

    module.add_header("Content-Type: text/html");
    assert!(!module.headers.headers.is_empty());
}

#[test]
fn test_sapi_module_set_response_code() {
    let mut module = SapiModule::new("test");
    let _ = module.startup();

    module.set_response_code(200);
    assert_eq!(module.headers.http_response_code, 200);

    module.set_response_code(404);
    assert_eq!(module.headers.http_response_code, 404);
}

#[test]
fn test_sapi_startup_shutdown() {
    let module = SapiModule::new("test");
    let result = sapi_startup(module);
    assert!(result.is_ok());

    let result = sapi_shutdown();
    assert!(result.is_ok());
}

#[test]
fn test_php_cli_startup_shutdown() {
    let result = php_cli_startup();
    assert!(result.is_ok());

    let result = php_cli_shutdown();
    assert!(result.is_ok());
}
