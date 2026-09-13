//! Cross-function exception propagation:
//! a `throw` in a callee (function, method, static method, included file,
//! call_user_func callback) with no local catch must be catchable by `try`
//! regions in the caller's frame.

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
fn test_throw_in_function_caught_by_caller() {
    assert_output(
        r#"<?php
        function risky() {
            throw new Exception("boom");
        }
        try {
            risky();
            echo "not reached";
        } catch (Exception $e) {
            echo "caught: " . $e->getMessage();
        }
        "#,
        "caught: boom",
    );
}

#[test]
fn test_throw_in_method_caught_by_caller() {
    assert_output(
        r#"<?php
        class Repo {
            public function load($name) {
                if ($name === "missing") {
                    throw new RuntimeException("no such repo: " . $name);
                }
                return "ok";
            }
        }
        $r = new Repo();
        try {
            echo $r->load("missing");
        } catch (RuntimeException $e) {
            echo "handled: " . $e->getMessage();
        }
        "#,
        "handled: no such repo: missing",
    );
}

#[test]
fn test_throw_in_static_method_caught_by_caller() {
    assert_output(
        r#"<?php
        class Cfg {
            public static function get($k) {
                throw new Exception("unknown key " . $k);
            }
        }
        try {
            Cfg::get("timeout");
        } catch (Exception $e) {
            echo "static: " . $e->getMessage();
        }
        "#,
        "static: unknown key timeout",
    );
}

#[test]
fn test_nested_calls_propagate_to_first_matching_frame() {
    assert_output(
        r#"<?php
        function c() {
            throw new Exception("deep");
        }
        function b() {
            c();
            return "b-done";
        }
        function a() {
            try {
                return b();
            } catch (Exception $e) {
                return "a-caught-" . $e->getMessage();
            }
        }
        echo a();
        "#,
        "a-caught-deep",
    );
}

#[test]
fn test_catch_by_parent_class() {
    assert_output(
        r#"<?php
        function f() {
            throw new OutOfBoundsException("oob");
        }
        try {
            f();
        } catch (LogicException $e) {
            echo "parent catch: " . $e->getMessage();
        }
        "#,
        "parent catch: oob",
    );
}

#[test]
fn test_rethrow_from_catch_caught_outer() {
    assert_output(
        r#"<?php
        function inner() {
            try {
                throw new Exception("first");
            } catch (Exception $e) {
                throw new Exception("wrapped: " . $e->getMessage());
            }
        }
        try {
            inner();
        } catch (Exception $e) {
            echo $e->getMessage();
        }
        "#,
        "wrapped: first",
    );
}

#[test]
fn test_execution_continues_after_caught_propagation() {
    assert_output(
        r#"<?php
        function f() {
            throw new Exception("x");
        }
        $log = [];
        try {
            f();
            $log[] = "skipped";
        } catch (Exception $e) {
            $log[] = "caught";
        }
        $log[] = "after";
        echo implode(",", $log);
        "#,
        "caught,after",
    );
}

#[test]
fn test_call_user_func_throw_caught() {
    assert_output(
        r#"<?php
        function handler() {
            throw new Exception("from callback");
        }
        try {
            call_user_func("handler");
        } catch (Exception $e) {
            echo "cb: " . $e->getMessage();
        }
        "#,
        "cb: from callback",
    );
}

#[test]
fn test_return_inside_try_in_callee_keeps_try_stack_clean() {
    assert_output(
        r#"<?php
        function normal() {
            try {
                return "fine";
            } catch (Exception $e) {
                return "wrong";
            }
        }
        function thrower() {
            throw new Exception("later");
        }
        echo normal();
        try {
            thrower();
        } catch (Exception $e) {
            echo " then " . $e->getMessage();
        }
        "#,
        "fine then later",
    );
}

#[test]
fn test_two_sequential_calls_both_caught() {
    assert_output(
        r#"<?php
        function f($m) {
            throw new Exception($m);
        }
        try { f("one"); } catch (Exception $e) { echo $e->getMessage(); }
        try { f("two"); } catch (Exception $e) { echo "/" . $e->getMessage(); }
        "#,
        "one/two",
    );
}

#[test]
fn test_uncaught_through_frames_is_failure() {
    let (r, _out) = run_php(
        r#"<?php
        function deep() {
            throw new Exception("nobody catches me");
        }
        function middle() {
            deep();
        }
        middle();
        echo "not reached";
        "#,
    )
    .expect("run");
    assert!(
        matches!(r, PhpResult::Failure),
        "expected Failure, got {r:?}"
    );
}

#[test]
fn test_throw_in_include_caught_by_includer() {
    let dir = std::env::temp_dir().join(format!("phprs_exc_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let inc = dir.join("thrower_inc.php");
    std::fs::write(&inc, r#"<?php throw new Exception("from include");"#).expect("write");

    let main = dir.join("main_inc.php");
    std::fs::write(
        &main,
        format!(
            r#"<?php
            try {{
                include "{}";
            }} catch (Exception $e) {{
                echo "include caught: " . $e->getMessage();
            }}
            "#,
            inc.display()
        ),
    )
    .expect("write");

    let (op_array, ft) =
        phprs::engine::compile::compile_file_with_functions(main.to_str().unwrap())
            .expect("compile");
    phprs::php::output::php_output_start().expect("output");
    let mut ed = ExecuteData::new();
    ed.function_table = Some(Arc::new(ft));
    let result = execute_ex(&mut ed, &op_array);
    let output = phprs::php::output::php_output_end().expect("output end");
    assert!(matches!(result, PhpResult::Success), "result: {result:?}");
    assert_eq!(output.trim(), "include caught: from include");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_method_chain_throw_caught_same_object() {
    assert_output(
        r#"<?php
        class Api {
            public function fetch() {
                throw new Exception("503");
                return $this;
            }
            public function parse() {
                return "parsed";
            }
        }
        $api = new Api();
        try {
            echo $api->fetch()->parse();
        } catch (Exception $e) {
            echo "api failed: " . $e->getMessage();
        }
        "#,
        "api failed: 503",
    );
}

#[test]
fn test_finally_runs_on_normal_completion() {
    assert_output(
        r#"<?php
        try {
            echo "try";
        } finally {
            echo "finally";
        }
        echo "done";
        "#,
        "tryfinallydone",
    );
}

#[test]
fn test_finally_runs_after_catch() {
    assert_output(
        r#"<?php
        try {
            throw new Exception("err");
        } catch (Exception $e) {
            echo "caught";
        } finally {
            echo "finally";
        }
        echo "done";
        "#,
        "caughtfinallydone",
    );
}

#[test]
fn test_finally_runs_during_unwinding_then_caught_outer() {
    assert_output(
        r#"<?php
        try {
            try {
                throw new Exception("inner");
            } finally {
                echo "finally";
            }
        } catch (Exception $e) {
            echo "caught:" . $e->getMessage();
        }
        echo "done";
        "#,
        "finallycaught:innerdone",
    );
}

#[test]
fn test_finally_runs_during_unwinding_uncaught() {
    let (r, out) = run_php(r#"<?php
        try {
            try {
                throw new Exception("uncaught");
            } finally {
                echo "finally";
            }
        } catch (OtherException $e) {
            echo "should not catch";
        }
        echo "should not reach";
        "#).expect("run");
    assert!(matches!(r, PhpResult::Failure), "should fail");
    assert!(out.contains("finally"), "finally must run: {out:?}");
    assert!(!out.contains("should not"), "should not print: {out:?}");
}

#[test]
fn test_finally_in_function_during_unwinding() {
    assert_output(
        r#"<?php
        function risky() {
            try {
                throw new Exception("boom");
            } finally {
                echo "finally";
            }
        }
        try {
            risky();
        } catch (Exception $e) {
            echo "caught:" . $e->getMessage();
        }
        echo "done";
        "#,
        "finallycaught:boomdone",
    );
}
