//! Unit tests for PHP INI Configuration

use crate::php::ini::IniParser;

#[test]
fn test_ini_parser_new() {
    let parser = IniParser::new();
    assert_eq!(parser.entries.len(), 0);
}

#[test]
fn test_ini_parse_string_simple() {
    let mut parser = IniParser::new();
    let ini_content = "key1 = value1\nkey2 = value2\n";

    parser.parse_string(ini_content).unwrap();

    let entry1 = parser.get("key1");
    assert!(entry1.is_some());
    assert_eq!(entry1.unwrap().value, "value1");

    let entry2 = parser.get("key2");
    assert!(entry2.is_some());
    assert_eq!(entry2.unwrap().value, "value2");
}

#[test]
fn test_ini_parse_string_with_sections() {
    let mut parser = IniParser::new();
    let ini_content = "[section1]\nkey1 = value1\n[section2]\nkey2 = value2\n";

    parser.parse_string(ini_content).unwrap();

    let entry1 = parser.get("section1.key1");
    assert!(entry1.is_some());
    assert_eq!(entry1.unwrap().value, "value1");

    let entry2 = parser.get("section2.key2");
    assert!(entry2.is_some());
    assert_eq!(entry2.unwrap().value, "value2");

    let section1 = parser.get_section("section1");
    assert!(!section1.is_empty());
    let key1_entry = section1.iter().find(|e| e.name == "key1");
    assert!(key1_entry.is_some());
    assert_eq!(key1_entry.unwrap().value, "value1");
}

#[test]
fn test_ini_parse_string_with_comments() {
    let mut parser = IniParser::new();
    let ini_content = "; This is a comment\nkey1 = value1\n# Another comment\nkey2 = value2\n";

    parser.parse_string(ini_content).unwrap();

    let entry1 = parser.get("key1");
    assert!(entry1.is_some());
    assert_eq!(entry1.unwrap().value, "value1");

    let entry2 = parser.get("key2");
    assert!(entry2.is_some());
    assert_eq!(entry2.unwrap().value, "value2");
}

#[test]
fn test_ini_get_all() {
    let mut parser = IniParser::new();
    parser
        .parse_string("key1 = value1\nkey2 = value2\n")
        .unwrap();

    let all = parser.get_all();
    assert_eq!(all.len(), 2);
    let key1 = all.iter().find(|e| e.name == "key1");
    assert!(key1.is_some());
    assert_eq!(key1.unwrap().value, "value1");
}

#[test]
fn test_ini_empty_value() {
    let mut parser = IniParser::new();
    parser.parse_string("key1 =\nkey2 = value2\n").unwrap();

    let entry1 = parser.get("key1");
    assert!(entry1.is_some());
    assert_eq!(entry1.unwrap().value, "");

    let entry2 = parser.get("key2");
    assert!(entry2.is_some());
    assert_eq!(entry2.unwrap().value, "value2");
}

#[test]
fn test_ini_whitespace_handling() {
    let mut parser = IniParser::new();
    parser.parse_string("  key1  =  value1  \n").unwrap();

    let entry = parser.get("key1");
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().value, "value1");
}
