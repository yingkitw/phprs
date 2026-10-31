//! Loop control (`break` / `continue`) test suite
//!
//! `break N` / `continue N` compile to `Jmp` with compile-time-resolved
//! targets (loop head / increment) or fixups patched when the loop body
//! finishes. These tests pin PHP semantics: `continue` in a `for` loop still
//! runs the increment, `break N` / `continue N` cross N levels, and
//! misplaced statements are compile errors.

use phprs::engine::compile::compile_string_with_functions;
use phprs::engine::types::PhpResult;
use phprs::engine::vm::{execute_ex, ExecuteData};
use std::sync::Arc;

/// Compile and run a PHP code string, returning (result, output)
fn run_php(code: &str) -> Result<(PhpResult, String), String> {
    let (op_array, ft) = compile_string_with_functions(code, "loop_control_test.php")?;
    phprs::php::output::php_output_start().map_err(|e| e.to_string())?;
    let mut ed = ExecuteData::new();
    ed.function_table = Some(Arc::new(ft));
    let result = execute_ex(&mut ed, &op_array);
    let output = phprs::php::output::php_output_end().map_err(|e| e.to_string())?;
    Ok((result, output))
}

fn assert_success(r: &PhpResult, out: &str) {
    assert!(
        matches!(r, PhpResult::Success),
        "vm result: {r:?}, output: {out:?}"
    );
}

#[test]
fn test_break_and_continue_in_for() {
    let (r, out) = run_php(
        r#"<?php
$s = 0;
for ($i = 0; $i < 5; $i++) {
    if ($i == 2) { continue; }
    if ($i == 4) { break; }
    $s += $i;
}
echo $s;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    // 0 + 1 + 3 (i=2 skipped, i=4 breaks)
    assert_eq!(out, "4");
}

#[test]
fn test_continue_in_for_still_runs_increment() {
    // The defining for-loop semantics: continue jumps to the increment.
    let (r, out) = run_php(
        r#"<?php
$hits = 0;
for ($i = 0; $i < 4; $i++) {
    if ($i % 2 == 0) { continue; }
    $hits++;
}
echo $hits;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "2");
}

#[test]
fn test_break_and_continue_in_while() {
    let (r, out) = run_php(
        r#"<?php
$t = 0; $k = 0;
while (true) {
    $k++;
    if ($k > 3) { break; }
    if ($k == 2) { continue; }
    $t += $k;
}
echo $t . "," . $k;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    // t = 1 + 3 (k=2 skipped), loop exits at k=4
    assert_eq!(out, "4,4");
}

#[test]
fn test_break_and_continue_in_foreach() {
    let (r, out) = run_php(
        r#"<?php
$u = 0;
foreach ([1, 2, 3, 4, 5] as $v) {
    if ($v == 2) { continue; }
    if ($v == 4) { break; }
    $u += $v;
}
echo $u;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "4");
}

#[test]
fn test_break_two_levels_in_nested_for() {
    let (r, out) = run_php(
        r#"<?php
$n = 0;
for ($a = 0; $a < 3; $a++) {
    for ($b = 0; $b < 3; $b++) {
        if ($a == 1 && $b == 0) { break 2; }
        $n += 10;
    }
}
echo $n;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "30");
}

#[test]
fn test_continue_two_levels_in_nested_for() {
    // continue 2 from the inner loop still runs the OUTER increment.
    let (r, out) = run_php(
        r#"<?php
$n = 0;
for ($a = 0; $a < 3; $a++) {
    for ($b = 0; $b < 3; $b++) {
        if ($b == 1) { continue 2; }
        if ($a == 2) { break 2; }
        $n += 10;
    }
}
echo $n;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "20");
}

#[test]
fn test_continue_two_levels_in_nested_while() {
    let (r, out) = run_php(
        r#"<?php
$m = 0; $i = 0;
while ($i < 3) {
    $j = 0;
    while ($j < 3) {
        if ($j == 1) { $i++; continue 2; }
        $m += 10;
        $j++;
    }
}
echo $m;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "30");
}

#[test]
fn test_break_inside_if_and_mixed_levels() {
    let (r, out) = run_php(
        r#"<?php
$out = "";
for ($i = 0; $i < 4; $i++) {
    for ($j = 0; $j < 4; $j++) {
        if ($j == 1) { continue 1; }
        if ($i == 2) { break 2; }
        $out .= "$i$j ";
    }
}
echo $out;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    // i=0: j=0,2,3 (j=1 continues); i=1: same; i=2: j=0 triggers
    // break 2 before anything is appended.
    assert_eq!(out, "00 02 03 10 12 13 ");
}

#[test]
fn test_explicit_level_one() {
    let (r, out) = run_php(
        r#"<?php
$s = 0;
for ($i = 0; $i < 5; $i++) {
    if ($i == 3) { break 1; }
    $s += $i;
}
echo $s;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "3");
}

#[test]
fn test_break_outside_loop_is_compile_error() {
    let result = compile_string_with_functions("<?php break;", "err.php");
    assert!(result.is_err(), "break outside a loop must be a compile error");
}

#[test]
fn test_continue_outside_loop_is_compile_error() {
    let result = compile_string_with_functions("<?php continue;", "err.php");
    assert!(
        result.is_err(),
        "continue outside a loop must be a compile error"
    );
}

#[test]
fn test_break_too_many_levels_is_compile_error() {
    let result = compile_string_with_functions(
        "<?php while (true) { break 2; }",
        "err.php",
    );
    assert!(result.is_err(), "break 2 with one loop must be a compile error");
}

#[test]
fn test_break_zero_is_compile_error() {
    let result = compile_string_with_functions(
        "<?php while (true) { break 0; }",
        "err.php",
    );
    assert!(result.is_err(), "break 0 must be a compile error");
}

#[test]
fn test_break_in_function_does_not_see_outer_loop() {
    // Function bodies compile with a fresh CompileContext: a `break` inside
    // a function must not resolve against a lexically enclosing loop.
    let result = compile_string_with_functions(
        "<?php while (true) { function f() { break; } f(); }",
        "err.php",
    );
    assert!(
        result.is_err(),
        "break inside a function (no local loop) must be a compile error"
    );
}

#[test]
fn test_continue_in_closure_inside_loop() {
    // The closure has its own loop; the outer loop's stack must not leak in.
    let (r, out) = run_php(
        r#"<?php
$fn = function ($limit) {
    $s = 0;
    for ($i = 0; $i < 10; $i++) {
        if ($i == $limit) { break; }
        $s += $i;
    }
    return $s;
};
echo $fn(4);
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "6");
}

#[test]
fn test_loops_still_work_with_optimizer_passes() {
    // The DCE pass renumbers Jmp targets; break/continue Jmps must survive.
    let (r, out) = run_php(
        r#"<?php
$total = 0;
for ($i = 1; $i <= 100; $i++) {
    if ($i % 2 == 0) { continue; }
    if ($i > 10) { break; }
    $total += $i;
}
echo $total;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    // odds 1..=9 → 1+3+5+7+9 = 25
    assert_eq!(out, "25");
}
