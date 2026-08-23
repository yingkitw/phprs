//! Bitwise operators (&, |, ^, ~, <<, >>), bitwise compound assignment
//! (&=, |=, ^=, <<=, >>=), and integer literal bases (0x, 0b, 0o, legacy
//! octal, underscore separators).

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
fn test_bitwise_binary_operators() {
    assert_output(
        r#"<?php
        $a = 12; $b = 10;
        echo $a & $b, "|", $a | $b, "|", $a ^ $b;
        "#,
        "8|14|6",
    );
}

#[test]
fn test_bitwise_not() {
    // ~12 == -13; -~12 == 13
    assert_output(
        r#"<?php
        echo ~12, "|", -~12;
        "#,
        "-13|13",
    );
}

#[test]
fn test_shift_operators() {
    assert_output(
        r#"<?php
        echo 1 << 4, "|", 256 >> 3, "|", -16 >> 2;
        "#,
        "16|32|-4",
    );
}

#[test]
fn test_shift_count_masked() {
    // PHP (64-bit) masks shift counts to 6 bits: 1 << 64 === 1 << 0
    assert_output(
        r#"<?php
        echo 1 << 64;
        "#,
        "1",
    );
}

#[test]
fn test_bitwise_precedence() {
    // PHP 8: comparison binds tighter than & — (3 & 1) == 1 is true
    assert_output(
        r#"<?php
        $flags = 0b1010;
        $mask = 0b0110;
        echo ($flags & $mask) == 0b0010 ? "yes" : "no";
        "#,
        "yes",
    );
}

#[test]
fn test_bitwise_compound_assignment() {
    assert_output(
        r#"<?php
        $c = 5;
        $c <<= 2;
        $c |= 1;
        $c ^= 3;
        $c &= 0xff;
        $c >>= 1;
        echo $c;
        "#,
        "11",
    );
}

#[test]
fn test_integer_literal_bases() {
    assert_output(
        r#"<?php
        echo 0xff, "|", 0XFF, "|", 0b1010, "|", 0o17, "|", 0777, "|", 1_000_000;
        "#,
        "255|255|10|15|511|1000000",
    );
}

#[test]
fn test_unary_minus() {
    assert_output(
        r#"<?php
        $x = -5;
        $y = -$x + 3;
        echo $x, "|", $y, "|", -~12;
        "#,
        "-5|8|13",
    );
}

#[test]
fn test_logical_xor_keyword() {
    // Note: unlike Zend, `=` binds looser than `xor` here (keyword-operator
    // statements like `$a = true xor true` differ); parenthesize when mixing.
    assert_output(
        r#"<?php
        $r = (true xor true);
        $s = (true xor false);
        echo $r === false ? "f" : "t", "|", $s === true ? "t" : "f";
        "#,
        "f|t",
    );
}

#[test]
fn test_power_operator() {
    assert_output(
        r#"<?php
        var_dump(2 ** 10);
        var_dump(2 ** 3 ** 2);
        var_dump(2 * 3 ** 2);
        var_dump(-2 ** 2);
        "#,
        "int(1024)\nint(512)\nint(18)\nint(-4)",
    );
}

#[test]
fn test_spaceship_operator() {
    assert_output(
        r#"<?php
        echo 5 <=> 3, "|", 3 <=> 5, "|", 4 <=> 4, "|", "a" <=> "b";
        "#,
        "1|-1|0|-1",
    );
}
