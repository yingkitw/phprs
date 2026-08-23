//! Numeric-string array key normalization (PHP semantics):
//! `$a["1"]` and `$a[1]` address the same slot; canonical decimal integer
//! strings are stored as integer keys; non-canonical ("01", "-0", "1x",
//! overflow) remain string keys.

use phprs::engine::compile::compile_string_with_functions;
use phprs::engine::types::PhpResult;
use phprs::engine::vm::{ExecuteData, execute_ex};
use std::sync::Arc;

fn run_php(code: &str) -> Result<(PhpResult, String), String> {
    let (op_array, ft) = compile_string_with_functions(code, "test.php")?;
    phprs::php::output::php_output_start().map_err(|e| e.to_string())?;
    let mut ed = ExecuteData::new();
    ed.function_table = Some(Arc::new(ft));
    let result = execute_ex(&mut ed, &op_array);
    let output = phprs::php::output::php_output_end().map_err(|e| e.to_string())?;
    Ok((result, output))
}

fn assert_output(code: &str, expected: &str) {
    let (r, out) = run_php(code).expect("run");
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
    assert_eq!(out.trim(), expected, "output mismatch");
}

#[test]
fn test_numeric_string_key_write_int_read() {
    assert_output(
        r#"<?php
        $a = array();
        $a["1"] = "one";
        echo $a[1];
        "#,
        "one",
    );
}

#[test]
fn test_int_literal_key_string_read() {
    assert_output(
        r#"<?php
        $b = array("10" => "ten");
        echo $b[10], "|", $b["10"];
        "#,
        "ten|ten",
    );
}

#[test]
fn test_non_canonical_keys_stay_strings() {
    assert_output(
        r#"<?php
        $c = array("01" => "a", "1x" => "b", "-0" => "c", "9223372036854775808" => "d");
        echo $c["01"], $c["1x"], $c["-0"], $c["9223372036854775808"];
        "#,
        "abcd",
    );
}

#[test]
fn test_negative_numeric_key() {
    // Unit-level: negative keys round-trip through the hash layer
    use phprs::engine::hash::{hash_add_or_update, hash_index_find, hash_init};
    use phprs::engine::string::string_init;
    use phprs::engine::types::{PhpArray, PhpType, PhpValue, Val};
    let mut ht = PhpArray::new();
    hash_init(&mut ht, 8);
    let v = Val::new(PhpValue::Long(99), PhpType::Long);
    let skey = string_init("-5", false);
    hash_add_or_update(&mut ht, Some(&skey), 0, v, 0);
    assert!(hash_index_find(&ht, (-5i64) as u64).is_some());

    assert_output(
        r#"<?php
        $a = array();
        $a["-5"] = "neg";
        echo $a[-5];
        "#,
        "neg",
    );
}

#[test]
fn test_numeric_string_key_unit() {
    use phprs::engine::hash::numeric_string_key;
    assert_eq!(numeric_string_key("1"), Some(1));
    assert_eq!(numeric_string_key("0"), Some(0));
    assert_eq!(numeric_string_key("42"), Some(42));
    assert_eq!(numeric_string_key("-7"), Some(-7));
    assert_eq!(numeric_string_key("01"), None, "leading zero");
    assert_eq!(numeric_string_key("-0"), None);
    assert_eq!(numeric_string_key(""), None);
    assert_eq!(numeric_string_key("1.5"), None);
    assert_eq!(numeric_string_key("1e3"), None);
    assert_eq!(numeric_string_key(" 1"), None);
    assert_eq!(numeric_string_key("9223372036854775807"), Some(i64::MAX));
    assert_eq!(numeric_string_key("9223372036854775808"), None, "overflow");
}
