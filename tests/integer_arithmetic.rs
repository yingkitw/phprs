//! Integer arithmetic type preservation (PHP semantics):
//! int op int stays int (`10 - 5 === int(5)`); division yields int only
//! when evenly divisible (`10/2 === 5`, `10/4 === 2.5`).

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
fn test_sub_mul_preserve_int() {
    assert_output(
        r#"<?php
        var_dump(10 - 5);
        var_dump(7 * 3);
        "#,
        "int(5)\nint(21)",
    );
}

#[test]
fn test_division_int_when_even() {
    assert_output(
        r#"<?php
        var_dump(10 / 2);
        var_dump(10 / 4);
        "#,
        "int(5)\nfloat(2.5)",
    );
}

#[test]
fn test_arithmetic_result_as_array_key() {
    // 2-3 === -1 → integer key; "2-3" arithmetic used to produce doubles
    // that never matched integer-keyed slots
    assert_output(
        r#"<?php
        $a = array();
        $a[2 - 3] = "neg";
        echo $a[-1];
        "#,
        "neg",
    );
}
