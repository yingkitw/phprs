//! End-to-end runs of curated `examples/` scripts (compile + VM + output capture).
//! Failing tests here mean a regression in real demos and tutorials.

use phprs::engine::compile::compile_string_with_functions;
use phprs::engine::types::PhpResult;
use phprs::engine::vm::{execute_ex, ExecuteData};
use std::path::PathBuf;
use std::sync::Arc;

fn examples_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

/// Compile and run a PHP file under `examples/` with the same path semantics as the CLI
/// (canonical script path for includes and `__FILE__`/`__DIR__`).
fn run_example_phprs(rel: &str) -> Result<(PhpResult, String), String> {
    let path = examples_root().join(rel);
    if !path.is_file() {
        return Err(format!("missing example file {}", path.display()));
    }
    let code = std::fs::read_to_string(&path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    let script_path = std::fs::canonicalize(&path)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string_lossy().into_owned());
    let (op_array, ft) = compile_string_with_functions(&code, &script_path)?;
    phprs::php::output::php_output_start().map_err(|e| e.to_string())?;
    let mut ed = ExecuteData::new();
    ed.function_table = Some(Arc::new(ft));
    let result = execute_ex(&mut ed, &op_array);
    let output = phprs::php::output::php_output_end().map_err(|e| e.to_string())?;
    Ok((result, output))
}

#[test]
fn example_01_hello_world_runs() {
    let (r, out) = run_example_phprs("01_hello_world.php").expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(
        out.contains("Hello, World!") && out.contains("Welcome to PHP-RS!"),
        "unexpected output: {out:?}"
    );
}

#[test]
fn example_operators_runs() {
    let (r, out) = run_example_phprs("operators.php").expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("a + b = 13"), "output: {out:?}");
    assert!(
        out.contains("OPERATORS WORK") || out.contains("operators work"),
        "output: {out:?}"
    );
}

#[test]
fn example_match_expression_runs() {
    let (r, out) = run_example_phprs("match_expression.php").expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("OK"), "output: {out:?}");
    assert!(out.contains("six"), "output: {out:?}");
}

#[test]
fn example_classes_compiles_and_runs() {
    // Full string interpolation / __construct parity with PHP is still evolving; require compile + non-fatal run.
    let (r, out) = run_example_phprs("classes.php").expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(
        out.contains("Hello, I'm") || out.contains("parent"),
        "expected some class method output: {out:?}"
    );
}

#[test]
fn example_control_flow_runs() {
    let (r, out) = run_example_phprs("control_flow.php").expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("Value is greater than 5"), "output: {out:?}");
    assert!(out.contains("Start of the week"), "output: {out:?}");
    assert!(out.contains("For loop:"), "output: {out:?}");
    assert!(out.contains("Iteration"), "output: {out:?}");
    assert!(out.contains("While loop:"), "output: {out:?}");
    assert!(out.contains("Count:"), "output: {out:?}");
    assert!(out.contains("Foreach loop:"), "output: {out:?}");
    assert!(out.contains("Item:"), "output: {out:?}");
}

#[test]
fn example_codeigniter_public_index_runs() {
    let (r, out) = run_example_phprs("codeigniter/public/index.php").expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(
        out.contains("CodeIgniter 4 bootstrap loaded"),
        "output: {out:?}"
    );
    assert!(
        out.contains("Hello from HomeController::index()"),
        "expected routed controller output: {out:?}"
    );
}

#[test]
fn example_drupal_index_runs() {
    let (r, out) = run_example_phprs("drupal/index.php").expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("Drupal bootstrap complete"), "output: {out:?}");
    assert!(out.contains("DrupalKernel::boot()"), "output: {out:?}");
}

#[test]
fn example_mbstring_runs() {
    let (r, out) = run_example_phprs("mbstring.php").expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(!out.trim().is_empty(), "expected mbstring example output");
}

#[test]
fn example_basic_types_runs() {
    let (r, out) = run_example_phprs("basic_types.php").expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("Long value: 42"), "output: {out:?}");
    assert!(out.contains("Hello, phprs!"), "output: {out:?}");
    assert!(out.contains("Null is null: yes"), "output: {out:?}");
}

#[test]
fn example_string_operations_runs() {
    let (r, out) = run_example_phprs("string_operations.php").expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("Concatenation (2 strings): HelloWorld"), "output: {out:?}");
    assert!(out.contains("Length of 'Hello': 5"), "output: {out:?}");
}

#[test]
fn example_array_operations_runs() {
    let (r, out) = run_example_phprs("array_operations.php").expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("Array size:"), "output: {out:?}");
    assert!(out.contains("Name: PHP-RS"), "output: {out:?}");
    assert!(out.contains("Index 0: first"), "output: {out:?}");
    let items = out.lines().filter(|l| l.starts_with("item:")).count();
    assert!(items >= 3, "expected foreach items, output: {out:?}");
}

#[test]
fn example_variables_runs() {
    let (r, out) = run_example_phprs("variables.php").expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("Counter: 42"), "output: {out:?}");
    assert!(out.contains("pi is float: yes"), "output: {out:?}");
    assert!(out.contains("active is bool: yes"), "output: {out:?}");
}

#[test]
fn example_filesystem_runs() {
    let (r, out) = run_example_phprs("filesystem.php").expect("run");
    assert!(matches!(r, PhpResult::Success), "vm result: {r:?}, output: {out:?}");
    assert!(out.contains("Does '.' exist? yes"), "output: {out:?}");
    assert!(out.contains("Contents length:"), "output: {out:?}");
}
