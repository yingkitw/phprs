//! `switch` statement and `do-while` loop test suite
//!
//! `switch` compiles to an interleaved test/fallthrough layout: the subject
//! expression evaluates once, each case compares it with `IsEqual` + `JmpNZ`
//! (first match wins), bodies fall through unless `break` jumps to
//! after-switch, and `default` runs only when no case matched (regardless of
//! source position). `switch` is a breakable level: `continue` inside a case
//! acts like `break` (PHP semantics). `do-while` runs the body at least once
//! and `continue` jumps to the condition.

use phprs::engine::compile::compile_string_with_functions;
use phprs::engine::types::PhpResult;
use phprs::engine::vm::{execute_ex, ExecuteData};
use std::sync::Arc;

/// Compile and run a PHP code string, returning (result, output)
fn run_php(code: &str) -> Result<(PhpResult, String), String> {
    let (op_array, ft) = compile_string_with_functions(code, "switch_test.php")?;
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
fn test_switch_basic_match_and_default() {
    let (r, out) = run_php(
        r#"<?php
function classify($day) {
    switch ($day) {
        case "Monday":
            return "start";
        case "Friday":
            return "end";
        default:
            return "midweek";
    }
}
echo classify("Monday") . "," . classify("Friday") . "," . classify("Wednesday");
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "start,end,midweek");
}

#[test]
fn test_switch_default_middle_position_still_last_resort() {
    // PHP checks all cases before default, whatever its position.
    let (r, out) = run_php(
        r#"<?php
$m = "";
switch (2) {
    case 1:
        $m .= "one";
        break;
    default:
        $m .= "def";
        break;
    case 2:
        $m .= "two";
}
echo $m;

$n = "";
switch (99) {
    case 1:
        $n .= "one";
        break;
    default:
        $n .= "def";
        break;
    case 2:
        $n .= "two";
}
echo "|" . $n;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "two|def");
}

#[test]
fn test_switch_fallthrough_accumulates() {
    let (r, out) = run_php(
        r#"<?php
$n = 0;
switch (1) {
    case 1:
        $n += 1;
    case 2:
        $n += 10;
        break;
    case 3:
        $n += 100;
}
echo $n;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "11");
}

#[test]
fn test_switch_consecutive_labels_share_body() {
    let (r, out) = run_php(
        r#"<?php
function weekend($d) {
    switch ($d) {
        case "Saturday":
        case "Sunday":
            return "yes";
        default:
            return "no";
    }
}
echo weekend("Saturday") . "," . weekend("Sunday") . "," . weekend("Mon");
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "yes,yes,no");
}

#[test]
fn test_switch_expression_cases() {
    let (r, out) = run_php(
        r#"<?php
$v = 5;
switch (true) {
    case $v > 10:
        echo "big";
        break;
    case $v > 3:
        echo "medium";
        break;
    default:
        echo "small";
}

switch (2 + 3) {
    case 4:
        echo "|four";
        break;
    case 5:
        echo "|five";
        break;
}
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "medium|five");
}

#[test]
fn test_switch_no_match_no_default() {
    let (r, out) = run_php(
        r#"<?php
$hit = false;
switch (42) {
    case 1:
        $hit = true;
        break;
}
echo $hit ? "hit" : "miss";
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "miss");
}

#[test]
fn test_break_in_switch_only_exits_switch() {
    // `break` inside a switch inside a loop exits the SWITCH, not the loop.
    let (r, out) = run_php(
        r#"<?php
$out = "";
for ($i = 0; $i < 3; $i++) {
    switch ($i) {
        case 0:
            $out .= "zero ";
            break;
        case 1:
            $out .= "one ";
            break;
        default:
            $out .= "many ";
    }
    $out .= "loop{$i} ";
}
echo $out;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "zero loop0 one loop1 many loop2 ");
}

#[test]
fn test_break_two_levels_exits_switch_and_loop() {
    let (r, out) = run_php(
        r#"<?php
$out = "";
for ($i = 0; $i < 5; $i++) {
    switch ($i) {
        case 2:
            $out .= "two";
            break 2;
        default:
            $out .= "d{$i} ";
    }
    $out .= "after{$i} ";
}
echo $out;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "d0 after0 d1 after1 two");
}

#[test]
fn test_continue_one_in_switch_acts_like_break() {
    // PHP: `continue` targeting a switch is equivalent to `break`.
    let (r, out) = run_php(
        r#"<?php
$out = "";
for ($i = 0; $i < 3; $i++) {
    switch ($i) {
        case 1:
            $out .= "skip ";
            continue 1;
        default:
            $out .= "keep ";
    }
    $out .= "tail{$i} ";
}
echo $out;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    // PHP: `continue` targeting a switch exits the switch, and execution
    // resumes after it — "tail1" still runs.
    assert_eq!(out, "keep tail0 skip tail1 keep tail2 ");
}

#[test]
fn test_continue_two_levels_targets_loop_increment() {
    let (r, out) = run_php(
        r#"<?php
$out = "";
for ($i = 0; $i < 4; $i++) {
    switch ($i) {
        case 1:
        case 2:
            continue 2;
        default:
            $out .= "v{$i} ";
    }
    $out .= "t{$i} ";
}
echo $out;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    // i=1,2 skip both the switch tail and the loop tail (increment runs).
    assert_eq!(out, "v0 t0 v3 t3 ");
}

#[test]
fn test_nested_switch() {
    let (r, out) = run_php(
        r#"<?php
$out = "";
switch (1) {
    case 1:
        switch (2) {
            case 2:
                $out .= "inner ";
                break;
            default:
                $out .= "inner-def ";
        }
        $out .= "outer-resumed ";
        break;
}
echo $out;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "inner outer-resumed ");
}

#[test]
fn test_switch_empty_and_no_labels() {
    let (r, out) = run_php(
        r#"<?php
switch (1) { }
echo "ok";
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "ok");
}

#[test]
fn test_do_while_runs_at_least_once() {
    let (r, out) = run_php(
        r#"<?php
$c = 0;
do { $c++; } while ($c < 3);
echo $c;
$d = 0;
do { $d++; } while (false);
echo "|" . $d;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "3|1");
}

#[test]
fn test_do_while_break_and_continue() {
    // continue jumps to the condition (which still runs); break exits.
    let (r, out) = run_php(
        r#"<?php
$i = 0; $sum = 0;
do {
    $i++;
    if ($i == 2) { continue; }
    if ($i == 4) { break; }
    $sum += $i;
} while ($i < 100);
echo $sum;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    // i=1 (+1), i=2 continue (condition i<100 true), i=3 (+3), i=4 break → 4
    assert_eq!(out, "4");
}

#[test]
fn test_switch_inside_do_while() {
    let (r, out) = run_php(
        r#"<?php
$i = 0; $out = "";
do {
    switch ($i) {
        case 0:
            $out .= "z ";
            break;
        case 1:
            $out .= "one";
            break;
        default:
            $out .= "d ";
    }
    $i++;
} while ($i < 3);
echo $out;
"#,
    )
    .expect("run");
    assert_success(&r, &out);
    assert_eq!(out, "z oned ");
}
