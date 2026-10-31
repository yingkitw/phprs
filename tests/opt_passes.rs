//! Optimization pass regression tests
//!
//! Constant folding and dead code elimination run inside
//! `compile_string_with_functions`, so every test here exercises optimized
//! op arrays. These tests pin the *behavior* (semantics preserved); the
//! white-box structural assertions live in `src/engine/opt.rs`.

use phprs::engine::compile::compile_string_with_functions;
use phprs::engine::types::PhpResult;
use phprs::engine::vm::{execute_ex, ExecuteData};
use std::sync::Arc;

/// Compile (with optimization passes applied) and run a PHP code string,
/// returning (result, output)
fn run_php(code: &str) -> Result<(PhpResult, String), String> {
    let (op_array, ft) = compile_string_with_functions(code, "opt_passes_test.php")?;
    phprs::php::output::php_output_start().map_err(|e| e.to_string())?;
    let mut ed = ExecuteData::new();
    ed.function_table = Some(Arc::new(ft));
    let result = execute_ex(&mut ed, &op_array);
    let output = phprs::php::output::php_output_end().map_err(|e| e.to_string())?;
    Ok((result, output))
}

#[test]
fn test_folded_constants_in_expressions() {
    let (r, out) = run_php(
        r#"<?php
echo 2 + 3 * 4;              // 14
echo "|";
echo 10 / 4;                 // 2.5
echo "|";
echo 10 / 2;                 // 5 (int, folded through zval_div)
echo "|";
echo "a" . "b" . (1 + 2);    // ab3
"#,
    )
    .expect("run");
    assert!(matches!(r, PhpResult::Success), "result: {r:?}, out: {out:?}");
    assert_eq!(out, "14|2.5|5|ab3");
}

#[test]
fn test_folded_comparisons_and_bools() {
    let (r, out) = run_php(
        r#"<?php
var_dump(1 < 2);
var_dump(1 + 1 === 2);
var_dump("x" <=> "y");
"#,
    )
    .expect("run");
    assert!(matches!(r, PhpResult::Success), "result: {r:?}, out: {out:?}");
    assert!(out.contains("bool(true)"), "got: {out:?}");
    assert!(out.contains("int(-1)"), "got: {out:?}");
}

#[test]
fn test_division_by_zero_still_fails_at_runtime() {
    let r = run_php("<?php echo 1 / 0;");
    // The op must survive folding so runtime error semantics are intact.
    assert!(r.is_err() || !matches!(r.as_ref().unwrap().0, PhpResult::Success));
}

#[test]
fn test_dead_code_after_return_not_executed() {
    let (r, out) = run_php(
        r#"<?php
function f() {
    return "ok";
    echo "unreachable";
}
echo f();
"#,
    )
    .expect("run");
    assert!(matches!(r, PhpResult::Success), "result: {r:?}, out: {out:?}");
    assert_eq!(out, "ok");
}

#[test]
fn test_loops_with_renumbered_jumps() {
    let (r, out) = run_php(
        r#"<?php
// Nested loops with conditions (note: break/continue are not yet
// implemented in the engine — see TODO.md — so guard with if instead).
$total = 0;
for ($i = 0; $i < 3; $i++) {
    for ($j = 0; $j < 3; $j++) {
        if ($j != 1) { $total += 10; }
    }
}
echo $total;

// while loop
$n = 3;
while ($n > 0) { $n--; }
echo "|{$n}";

// foreach
$sum = 0;
foreach ([1, 2, 3, 4] as $k => $v) { $sum += $v; }
echo "|{$sum}";
"#,
    )
    .expect("run");
    assert!(matches!(r, PhpResult::Success), "result: {r:?}, out: {out:?}");
    assert_eq!(out, "60|0|10");
}

#[test]
fn test_try_catch_finally_with_dead_code() {
    let (r, out) = run_php(
        r#"<?php
function risky($v) {
    if ($v) { throw new RuntimeException("boom"); }
    return "safe";
}
try {
    echo risky(false);
    echo "|";
    risky(true);
    echo "unreachable-after-throw";
} catch (RuntimeException $e) {
    echo $e->getMessage();
} finally {
    echo "|cleaned";
}
"#,
    )
    .expect("run");
    assert!(matches!(r, PhpResult::Success), "result: {r:?}, out: {out:?}");
    assert_eq!(out, "safe|boom|cleaned");
}

#[test]
fn test_closures_and_folded_bodies() {
    let (r, out) = run_php(
        r#"<?php
$adder = function ($x) { return 20 + 30 + $x; };
echo $adder(5);
echo "|";
echo array_reduce([1, 2, 3], function ($c, $i) { return $c + 10 * $i; }, 0);
"#,
    )
    .expect("run");
    assert!(matches!(r, PhpResult::Success), "result: {r:?}, out: {out:?}");
    assert_eq!(out, "55|60");
}

#[test]
fn test_generators_still_work_when_optimized() {
    let (r, out) = run_php(
        r#"<?php
function gen() {
    yield 1;
    yield 2;
    yield 3;
}
$g = gen();
$total = 0;
while ($g->valid()) {
    $total += $g->current();
    $g->next();
}
echo $total;
"#,
    )
    .expect("run");
    assert!(matches!(r, PhpResult::Success), "result: {r:?}, out: {out:?}");
    assert_eq!(out, "6");
}

#[test]
fn test_match_and_ternary_with_folded_arms() {
    let (r, out) = run_php(
        r#"<?php
$x = 2;
echo match ($x) { 1 => "one", 2 => "two", default => "other" };
echo "|";
echo $x > 0 ? "pos" : "neg";
echo "|";
echo 3 ** 2;
"#,
    )
    .expect("run");
    assert!(matches!(r, PhpResult::Success), "result: {r:?}, out: {out:?}");
    assert_eq!(out, "two|pos|9");
}

#[test]
fn test_constant_condition_branches() {
    let (r, out) = run_php(
        r#"<?php
if (false) { echo "never"; } else { echo "else-taken"; }
echo "|";
while (false) { echo "loop-never"; }
echo "after-loop";
"#,
    )
    .expect("run");
    assert!(matches!(r, PhpResult::Success), "result: {r:?}, out: {out:?}");
    assert_eq!(out, "else-taken|after-loop");
}

#[test]
fn test_define_constant_not_folded_prematurely() {
    // Bare identifiers are ConstantAst operands — they must NOT fold at
    // compile time because the define() happens at runtime before use.
    let (r, out) = run_php(
        r#"<?php
define("MY_CONST", 41);
echo MY_CONST + 1;
"#,
    )
    .expect("run");
    assert!(matches!(r, PhpResult::Success), "result: {r:?}, out: {out:?}");
    assert_eq!(out, "42");
}
