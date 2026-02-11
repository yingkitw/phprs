//! Unit tests for PHP Streams

use crate::php::streams::{
    create_filter, php_stream_open, FileStream, FilterChain, StreamFilter, StreamMode,
    StringRot13Filter, StringToLowerFilter, StringToUpperFilter,
};

#[test]
fn test_php_stream_open_read() {
    if std::path::Path::new("Cargo.toml").exists() {
        let result = php_stream_open("Cargo.toml", StreamMode::Read);
        assert!(result.is_ok());
    }
}

#[test]
fn test_php_stream_open_nonexistent() {
    let result = php_stream_open("/nonexistent/file.txt", StreamMode::Read);
    assert!(result.is_err());
}

#[test]
fn test_stream_mode() {
    assert_eq!(StreamMode::Read as u8, 0);
    assert_eq!(StreamMode::Write as u8, 1);
    assert_eq!(StreamMode::Append as u8, 2);
    assert_eq!(StreamMode::ReadWrite as u8, 3);
}

#[test]
fn test_file_stream_open() {
    if std::path::Path::new("Cargo.toml").exists() {
        let result = FileStream::open("Cargo.toml", StreamMode::Read);
        assert!(result.is_ok());
    }
}

// Stream filter tests

#[test]
fn test_string_toupper_filter() {
    let filter = StringToUpperFilter;
    assert_eq!(filter.name(), "string.toupper");
    assert_eq!(filter.filter_read(b"hello world"), b"HELLO WORLD");
    assert_eq!(filter.filter_write(b"abc 123"), b"ABC 123");
}

#[test]
fn test_string_tolower_filter() {
    let filter = StringToLowerFilter;
    assert_eq!(filter.name(), "string.tolower");
    assert_eq!(filter.filter_read(b"HELLO WORLD"), b"hello world");
    assert_eq!(filter.filter_write(b"ABC 123"), b"abc 123");
}

#[test]
fn test_string_rot13_filter() {
    let filter = StringRot13Filter;
    assert_eq!(filter.name(), "string.rot13");
    let encoded = filter.filter_read(b"Hello");
    assert_eq!(encoded, b"Uryyb");
    // ROT13 is its own inverse
    let decoded = filter.filter_read(&encoded);
    assert_eq!(decoded, b"Hello");
}

#[test]
fn test_base64_encode_decode_filter() {
    let encode = create_filter("convert.base64-encode").unwrap();
    let decode = create_filter("convert.base64-decode").unwrap();

    let original = b"Hello, World!";
    let encoded = encode.filter_read(original);
    let decoded = decode.filter_read(&encoded);
    assert_eq!(decoded, original);
}

#[test]
fn test_filter_chain() {
    let mut chain = FilterChain::new();
    assert!(chain.is_empty());

    chain.append(Box::new(StringToUpperFilter));
    assert_eq!(chain.len(), 1);

    let result = chain.apply_read(b"hello");
    assert_eq!(result, b"HELLO");
}

#[test]
fn test_filter_chain_multiple() {
    let mut chain = FilterChain::new();
    chain.append(Box::new(StringToUpperFilter));
    chain.append(Box::new(StringRot13Filter));

    // First toupper, then rot13
    let result = chain.apply_read(b"hello");
    // "hello" -> "HELLO" -> "URYYB"
    assert_eq!(result, b"URYYB");
}

#[test]
fn test_filter_chain_prepend() {
    let mut chain = FilterChain::new();
    chain.append(Box::new(StringToUpperFilter));
    chain.prepend(Box::new(StringRot13Filter));

    // First rot13, then toupper
    let result = chain.apply_read(b"hello");
    // "hello" -> "uryyb" -> "URYYB"
    assert_eq!(result, b"URYYB");
}

#[test]
fn test_filter_chain_remove() {
    let mut chain = FilterChain::new();
    chain.append(Box::new(StringToUpperFilter));
    chain.append(Box::new(StringToLowerFilter));
    assert_eq!(chain.len(), 2);

    let removed = chain.remove("string.toupper");
    assert!(removed);
    assert_eq!(chain.len(), 1);

    // Only tolower remains
    let result = chain.apply_read(b"HELLO");
    assert_eq!(result, b"hello");
}

#[test]
fn test_create_filter_by_name() {
    assert!(create_filter("string.toupper").is_ok());
    assert!(create_filter("string.tolower").is_ok());
    assert!(create_filter("string.rot13").is_ok());
    assert!(create_filter("convert.base64-encode").is_ok());
    assert!(create_filter("convert.base64-decode").is_ok());
    assert!(create_filter("nonexistent.filter").is_err());
}
