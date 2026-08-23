//! Compound assignment operators (`+=`, `-=`, `*=`, `/=`, `%=`, `.=`)
//! for plain variables, array dimensions, and object properties,
//! plus comma-separated `echo` arguments.

use phprs::engine::compile::compile_string_with_functions;
use phprs::engine::types::PhpResult;
use phprs::engine::vm::{ExecuteData, execute_ex};
use std::sync::Arc;

/// Compile and run a PHP code string, returning (result, output)
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
fn test_compound_arithmetic_on_variable() {
    assert_output(
        r#"<?php
        $n = 10;
        $n += 5;
        $n -= 3;
        $n *= 4;
        $n /= 8;
        $n %= 4;
        echo $n;
        "#,
        "2",
    );
}

#[test]
fn test_compound_concat_on_variable() {
    assert_output(
        r#"<?php
        $s = "a";
        $s .= "-b";
        $s .= "-c";
        echo $s;
        "#,
        "a-b-c",
    );
}

#[test]
fn test_compound_concat_undefined_variable_reads_null() {
    // PHP: $u .= 'v' treats undefined/null as '' → 'v'
    assert_output(
        r#"<?php
        $u = null;
        $u .= "v";
        echo $u;
        "#,
        "v",
    );
}

#[test]
fn test_compound_assignment_on_array_dim() {
    assert_output(
        r#"<?php
        $a = array("k" => 10);
        $a["k"] += 5;
        echo $a["k"];
        "#,
        "15",
    );
}

#[test]
fn test_compound_assignment_on_chained_dim() {
    assert_output(
        r#"<?php
        $d = array("a" => array("b" => "x"));
        $d["a"]["b"] .= "y";
        echo $d["a"]["b"];
        "#,
        "xy",
    );
}

#[test]
fn test_compound_concat_on_missing_key_reads_null() {
    // PHP auto-vivifies: $u['k'] .= 'v' → 'v' (null key read)
    assert_output(
        r#"<?php
        $u = array();
        $u["k"] .= "v";
        echo $u["k"];
        "#,
        "v",
    );
}

#[test]
fn test_compound_assignment_on_object_property() {
    assert_output(
        r#"<?php
        class C { public $p = 10; }
        $c = new C();
        $c->p += 5;
        $c->p .= "!";
        echo $c->p;
        "#,
        "15!",
    );
}

#[test]
fn test_compound_assignment_in_loop() {
    // Classic PHP string/counter building pattern
    assert_output(
        r#"<?php
        $total = 0;
        $parts = "";
        for ($i = 1; $i <= 5; $i++) {
            $total += $i;
            $parts .= $i;
        }
        echo $total, "|", $parts;
        "#,
        "15|12345",
    );
}

#[test]
fn test_coalesce_equal() {
    assert_output(
        r#"<?php
        $a = null;
        $a ??= "default";
        $b = "set";
        $b ??= "other";
        echo $a, "|", $b;
        "#,
        "default|set",
    );
}

#[test]
fn test_echo_multiple_comma_arguments() {
    assert_output(
        r#"<?php
        $a = "x";
        $b = 1;
        echo $a, "-", $b, "-", "z";
        "#,
        "x-1-z",
    );
}
