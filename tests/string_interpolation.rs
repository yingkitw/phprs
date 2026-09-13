//! Complex string interpolation (double-quote syntax):
//! - simple: `"$name"`
//! - simple-syntax accessor: `"$arr[key]"`, `"$arr[0]"`, `"$arr[$k]"`, `"$obj->prop"`
//! - complex (curly): `"{$arr['key']}"`, `"{$obj->prop}"`, `"{$arr['a']['b']}"`,
//!   `"{$obj->method()}"`, `"{$fn()}"`, `"{$x + 1}"`-style expressions

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
fn test_simple_variable_still_works() {
    assert_output(
        r#"<?php
        $name = "World";
        echo "Hello $name!";
        "#,
        "Hello World!",
    );
}

#[test]
fn test_curly_array_string_key() {
    assert_output(
        r#"<?php
        $arr = ["key" => "value"];
        echo "got {$arr['key']}!";
        "#,
        "got value!",
    );
}

#[test]
fn test_curly_array_int_key() {
    assert_output(
        r#"<?php
        $arr = [10, 20, 30];
        echo "second={$arr[1]}";
        "#,
        "second=20",
    );
}

#[test]
fn test_curly_nested_dims() {
    assert_output(
        r#"<?php
        $data = ["a" => ["b" => ["c" => "deep"]]];
        echo "found {$data['a']['b']['c']}!";
        "#,
        "found deep!",
    );
}

#[test]
fn test_curly_object_property() {
    assert_output(
        r#"<?php
        class User {
            public $name = "Alice";
            public $role = "admin";
        }
        $u = new User();
        echo "{$u->name} is {$u->role}";
        "#,
        "Alice is admin",
    );
}

#[test]
fn test_curly_method_call() {
    assert_output(
        r#"<?php
        class Greeter {
            public function greet() {
                return "hi";
            }
        }
        $g = new Greeter();
        echo "say {$g->greet()}";
        "#,
        "say hi",
    );
}

#[test]
fn test_curly_function_call_and_missing_key_reads_empty() {
    assert_output(
        r#"<?php
        function who() {
            return "phprs";
        }
        $arr = [];
        echo "hello " . who() . " missing=[{$arr['nope']}]";
        "#,
        "hello phprs missing=[]",
    );
}

#[test]
fn test_curly_expression_with_var_key() {
    assert_output(
        r#"<?php
        $arr = ["x" => "1", "y" => "2"];
        $k = "y";
        echo "picked {$arr[$k]}";
        "#,
        "picked 2",
    );
}

#[test]
fn test_multiple_curly_in_one_string() {
    assert_output(
        r#"<?php
        $a = "A";
        $b = "B";
        echo "{$a} and {$b} and {$a}{$b}";
        "#,
        "A and B and AB",
    );
}

#[test]
fn test_simple_syntax_array_accessors() {
    assert_output(
        r#"<?php
        $arr = ["key" => "v1", 0 => "v2", 1 => "v3"];
        $k = "key";
        echo "$arr[key] $arr[0] $arr[1] $arr[$k]";
        "#,
        "v1 v2 v3 v1",
    );
}

#[test]
fn test_simple_syntax_object_property() {
    assert_output(
        r#"<?php
        class Point {
            public $x = 3;
            public $y = 4;
        }
        $p = new Point();
        echo "($p->x, $p->y)";
        "#,
        "(3, 4)",
    );
}

#[test]
fn test_dollar_sign_literal_cases() {
    assert_output(
        r#"<?php
        $n = 5;
        echo "costs $5 and $n dollars";
        "#,
        "costs $5 and 5 dollars",
    );
}

#[test]
fn test_curly_brace_literal_without_dollar() {
    assert_output(
        r#"<?php
        $a = "x";
        echo "{$a} {literal} {y}";
        "#,
        "x {literal} {y}",
    );
}

#[test]
fn test_interpolation_in_assign_and_concat() {
    assert_output(
        r#"<?php
        $user = ["name" => "Bo", "id" => 7];
        $msg = "user {$user['name']} (#{$user['id']})";
        echo $msg . "!";
        "#,
        "user Bo (#7)!",
    );
}

#[test]
fn test_unterminated_curly_is_compile_error() {
    let err = run_php(r#"<?php echo "{$arr['key']"; "#);
    assert!(err.is_err(), "expected compile error, got {err:?}");
}

#[test]
fn test_unterminated_bracket_simple_syntax_is_compile_error() {
    let err = run_php(r#"<?php echo "bad $arr[key"; "#);
    assert!(err.is_err(), "expected compile error, got {err:?}");
}
