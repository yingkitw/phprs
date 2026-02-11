//! Unit tests for virtual machine

use crate::engine::types::{PhpResult, PhpType, PhpValue, Val};
use crate::engine::vm::{
    execute_ex, temp_var_ref, var_ref, get_opcode_name, ExecuteData, Op, OpArray,
    Opcode,
};

// Mutex to serialize tests that use the global output buffer
static OUTPUT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn run_php_code(code: &str) -> (PhpResult, String) {
    let _lock = OUTPUT_TEST_LOCK.lock().unwrap();
    let (op_array, ft) = crate::engine::compile::compile_string_with_functions(code, "test.php").unwrap();
    crate::php::output::php_output_start().unwrap();
    let mut ed = ExecuteData::new();
    ed.function_table = Some(std::sync::Arc::new(ft));
    let result = execute_ex(&mut ed, &op_array);
    let output = crate::php::output::php_output_end().unwrap();
    (result, output)
}

#[test]
fn test_opcode_enum() {
    // Verify opcode enum values
    assert_eq!(Opcode::Nop as u8, 0);
    assert_eq!(Opcode::Add as u8, 1);
    assert_eq!(Opcode::Sub as u8, 2);
    assert_eq!(Opcode::Mul as u8, 3);
    assert_eq!(Opcode::Div as u8, 4);
    assert_eq!(Opcode::Assign as u8, 22);
}

#[test]
fn test_op_creation() {
    let op = Op::new(
        Opcode::Add,
        Val::new(PhpValue::Long(10), PhpType::Long),
        Val::new(PhpValue::Long(20), PhpType::Long),
        Val::new(PhpValue::Long(0), PhpType::Long),
        0,
    );

    assert_eq!(op.opcode, Opcode::Add);
    assert_eq!(op.extended_value, 0);
}

#[test]
fn test_op_array_new() {
    let op_array = OpArray::new("test.php".to_string());
    assert_eq!(op_array.ops.len(), 0);
    assert_eq!(op_array.filename, Some("test.php".to_string()));
}

#[test]
fn test_get_opcode_name() {
    assert_eq!(get_opcode_name(Opcode::Nop), "NOP");
    assert_eq!(get_opcode_name(Opcode::Add), "ADD");
    assert_eq!(get_opcode_name(Opcode::Sub), "SUB");
    assert_eq!(get_opcode_name(Opcode::Mul), "MUL");
    assert_eq!(get_opcode_name(Opcode::Div), "DIV");
}

#[test]
fn test_execute_empty() {
    let op_array = OpArray::new("test.php".to_string());
    let mut execute_data = ExecuteData::new();

    let result = execute_ex(&mut execute_data, &op_array);
    // Empty op array should execute successfully
    assert!(matches!(result, PhpResult::Success));
}

#[test]
fn test_execute_data_creation() {
    let execute_data = ExecuteData::new();
    assert!(execute_data.op_array.is_none());
    assert_eq!(execute_data.current_op, 0);
}

#[test]
fn test_vm_assign_and_resolve_variable() {
    // $x = 42; — then verify $x is in symbol table
    let mut op_array = OpArray::new("test.php".to_string());
    let var_name = crate::engine::string::string_init("x", false);
    op_array.add_op(Op::new(
        Opcode::Assign,
        Val::new(PhpValue::String(Box::new(var_name)), PhpType::String),
        Val::new(PhpValue::Long(42), PhpType::Long),
        Val::new(PhpValue::Long(0), PhpType::Null),
        0,
    ));

    let mut ed = ExecuteData::new();
    let result = execute_ex(&mut ed, &op_array);
    assert!(matches!(result, PhpResult::Success));
    // Verify variable is in symbol table
    let val = ed.get_var("x");
    assert_eq!(val.get_type(), PhpType::Long);
    if let PhpValue::Long(v) = val.value { assert_eq!(v, 42); } else { panic!("expected Long"); }
}

#[test]
fn test_vm_add_with_temp_vars() {
    let _lock = OUTPUT_TEST_LOCK.lock().unwrap();
    // temp[0] = 10 + 20 → echo temp[0]
    let mut op_array = OpArray::new("test.php".to_string());
    op_array.add_op(Op::new(
        Opcode::Add,
        Val::new(PhpValue::Long(10), PhpType::Long),
        Val::new(PhpValue::Long(20), PhpType::Long),
        temp_var_ref(0),
        0,
    ));
    op_array.add_op(Op::new(
        Opcode::Echo,
        temp_var_ref(0),
        Val::new(PhpValue::Long(0), PhpType::Null),
        Val::new(PhpValue::Long(0), PhpType::Null),
        0,
    ));

    crate::php::output::php_output_start().unwrap();
    let mut ed = ExecuteData::new();
    let result = execute_ex(&mut ed, &op_array);
    let output = crate::php::output::php_output_end().unwrap();
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "30");
}

#[test]
fn test_vm_variable_in_expression() {
    let _lock = OUTPUT_TEST_LOCK.lock().unwrap();
    // Test: $a = 5; $b = 3; echo $a + $b;
    let mut op_array = OpArray::new("test.php".to_string());
    let var_a = crate::engine::string::string_init("a", false);
    let var_b = crate::engine::string::string_init("b", false);
    // ASSIGN $a = 5
    op_array.add_op(Op::new(
        Opcode::Assign,
        Val::new(PhpValue::String(Box::new(var_a)), PhpType::String),
        Val::new(PhpValue::Long(5), PhpType::Long),
        Val::new(PhpValue::Long(0), PhpType::Null),
        0,
    ));
    // ASSIGN $b = 3
    op_array.add_op(Op::new(
        Opcode::Assign,
        Val::new(PhpValue::String(Box::new(var_b)), PhpType::String),
        Val::new(PhpValue::Long(3), PhpType::Long),
        Val::new(PhpValue::Long(0), PhpType::Null),
        0,
    ));
    // ADD var_ref("a") + var_ref("b") → temp[0]
    op_array.add_op(Op::new(
        Opcode::Add,
        var_ref("a"),
        var_ref("b"),
        temp_var_ref(0),
        0,
    ));
    // ECHO temp[0]
    op_array.add_op(Op::new(
        Opcode::Echo,
        temp_var_ref(0),
        Val::new(PhpValue::Long(0), PhpType::Null),
        Val::new(PhpValue::Long(0), PhpType::Null),
        0,
    ));

    crate::php::output::php_output_start().unwrap();
    let mut ed = ExecuteData::new();
    let result = execute_ex(&mut ed, &op_array);
    let output = crate::php::output::php_output_end().unwrap();
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "8");
}

#[test]
fn test_vm_builtin_strlen() {
    let _lock = OUTPUT_TEST_LOCK.lock().unwrap();
    // InitFCall; SendVal "hello"; DoFCall strlen → temp[0]; Echo temp[0]
    let mut op_array = OpArray::new("test.php".to_string());
    let zero = || Val::new(PhpValue::Long(0), PhpType::Null);
    // InitFCall
    op_array.add_op(Op::new(Opcode::InitFCall, zero(), zero(), zero(), 0));
    // SendVal "hello"
    let hello = crate::engine::string::string_init("hello", false);
    op_array.add_op(Op::new(
        Opcode::SendVal,
        Val::new(PhpValue::String(Box::new(hello)), PhpType::String),
        zero(), zero(), 0,
    ));
    // DoFCall strlen → temp[0]
    let fname = crate::engine::string::string_init("strlen", false);
    op_array.add_op(Op::new(
        Opcode::DoFCall,
        Val::new(PhpValue::String(Box::new(fname)), PhpType::String),
        zero(), temp_var_ref(0), 0,
    ));
    // Echo temp[0]
    op_array.add_op(Op::new(Opcode::Echo, temp_var_ref(0), zero(), zero(), 0));

    crate::php::output::php_output_start().unwrap();
    let mut ed = ExecuteData::new();
    let result = execute_ex(&mut ed, &op_array);
    let output = crate::php::output::php_output_end().unwrap();
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "5");
}

#[test]
fn test_compile_and_execute_variable_arithmetic() {
    let code = r#"<?php
$x = 10;
$y = 20;
echo $x + $y;
"#;
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "30");
}

#[test]
fn test_compile_and_execute_function_call() {
    let code = r#"<?php
echo strlen("hello world");
"#;
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "11");
}

#[test]
fn test_compile_and_execute_strtoupper() {
    let code = r#"<?php
echo strtoupper("php-rs");
"#;
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "PHP-RS");
}

#[test]
fn test_compile_and_execute_complex_expression() {
    let code = r#"<?php
$a = 5;
$b = 3;
$c = $a * $b + 2;
echo $c;
"#;
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "17");
}

#[test]
fn test_compile_and_execute_string_interpolation() {
    let code = "<?php\n$name = \"World\";\necho \"Hello $name!\";\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "Hello World!");
}

#[test]
fn test_compile_and_execute_concat_operator() {
    let code = "<?php\necho \"a\" . \" \" . \"b\";\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "a b");
}

#[test]
fn test_compile_and_execute_parenthesized_expr() {
    let code = "<?php\n$x = 2;\n$y = 3;\necho ($x + $y) * 4;\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "20");
}

#[test]
fn test_compile_and_execute_function_call_statement() {
    let code = "<?php\n$x = 42;\nvar_dump($x);\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "int(42)\n");
}

#[test]
fn test_compile_and_execute_concat_with_function() {
    let code = "<?php\necho strlen(\"hello\") . \" chars\";\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "5 chars");
}

#[test]
fn test_compile_and_execute_indexed_array() {
    let code = "<?php\n$arr = [10, 20, 30];\necho count($arr);\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "3");
}

#[test]
fn test_compile_and_execute_assoc_array() {
    let code = "<?php\n$m = [\"a\" => \"hello\", \"b\" => \"world\"];\necho count($m);\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "2");
}

#[test]
fn test_compile_and_execute_array_access() {
    let code = "<?php\n$arr = [10, 20, 30];\necho $arr[0] . \" \" . $arr[2];\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "10 30");
}

#[test]
fn test_compile_and_execute_assoc_array_access() {
    let code = "<?php\n$m = [\"x\" => \"hello\"];\necho $m[\"x\"];\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "hello");
}

#[test]
fn test_compile_and_execute_ternary() {
    let code = "<?php\n$x = 10;\n$y = $x > 5 ? 'big' : 'small';\necho $y;\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "big");
}

#[test]
fn test_compile_and_execute_ternary_false() {
    let code = "<?php\n$x = 2;\n$y = $x > 5 ? 'big' : 'small';\necho $y;\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "small");
}

#[test]
fn test_compile_and_execute_null_coalesce() {
    let code = "<?php\n$x = null;\n$y = $x ?? 'default';\necho $y;\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "default");
}

#[test]
fn test_compile_and_execute_null_coalesce_non_null() {
    let code = "<?php\n$x = 'hello';\n$y = $x ?? 'default';\necho $y;\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "hello");
}

#[test]
fn test_compile_and_execute_closure_basic() {
    let code = "<?php\n$greet = function() { echo 'hi'; };\n$greet();\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "hi");
}

#[test]
fn test_compile_and_execute_closure_with_args() {
    let code = "<?php\n$add = function($a, $b) { echo $a + $b; };\n$add(3, 4);\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "7");
}

#[test]
fn test_compile_and_execute_function_return() {
    let code = "<?php\nfunction double($x) { return $x * 2; }\necho double(5);\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "10");
}

#[test]
fn test_compile_and_execute_typed_function() {
    let code = "<?php\nfunction add(int $a, int $b): int { return $a + $b; }\necho add(3, 4);\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "7");
}

#[test]
fn test_compile_and_execute_typed_closure() {
    let code = "<?php\n$fn = function(string $s): string { return $s; };\necho $fn('hello');\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "hello");
}

#[test]
fn test_compile_namespace_and_use() {
    let code = "<?php\nnamespace App\\Models;\nuse App\\Services\\Logger as Log;\necho 'ok';\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "ok");
}

#[test]
fn test_compile_trait_definition() {
    // Trait definition should parse without error
    let code = "<?php\ntrait Greetable {\n  public function greet() { echo 'hello'; }\n}\necho 'ok';\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "ok");
}

#[test]
fn test_compile_trait_use_in_class() {
    let code = "<?php\ntrait Greetable {\n  public function greet() { echo 'hi'; }\n}\nclass Person {\n  use Greetable;\n}\n$p = new Person();\n$p->greet();\n";
    let (result, output) = run_php_code(code);
    assert!(matches!(result, PhpResult::Success));
    assert_eq!(output, "hi");
}
