//! Unit tests for compiler

use crate::engine::compile::{compile_file, compile_string, CompileContext};
use crate::engine::types::Val;
use crate::engine::vm::{Op, Opcode};

#[test]
fn test_compile_context_new() {
    let context = CompileContext::new();
    assert_eq!(context.current_line, 0);
    assert_eq!(context.op_array.ops.len(), 0);
    assert!(context.filename.is_none());
}

#[test]
fn test_compile_context_set_line() {
    let mut context = CompileContext::new();
    context.set_line(42);
    assert_eq!(context.current_line, 42);
}

#[test]
fn test_compile_context_set_filename() {
    let mut context = CompileContext::new();
    context.set_filename("test.php");
    assert_eq!(context.filename, Some("test.php".to_string()));
    assert_eq!(context.op_array.filename, Some("test.php".to_string()));
}

#[test]
fn test_compile_context_emit_opcode() {
    let mut context = CompileContext::new();
    context.set_line(10);

    let op1 = Val::new(
        crate::engine::types::PhpValue::Long(5),
        crate::engine::types::PhpType::Long,
    );
    let op2 = Val::new(
        crate::engine::types::PhpValue::Long(3),
        crate::engine::types::PhpType::Long,
    );
    let result = Val::new(
        crate::engine::types::PhpValue::Long(0),
        crate::engine::types::PhpType::Long,
    );

    context.emit_opcode(Opcode::Add, op1, op2, result);

    assert_eq!(context.op_array.ops.len(), 1);
    assert_eq!(context.op_array.ops[0].opcode, Opcode::Add);
}

#[test]
fn test_compile_context_finalize() {
    let mut context = CompileContext::new();
    context.set_line(100);
    context.set_filename("test.php");

    let op_array = context.finalize();
    assert_eq!(op_array.line_start, 0);
    assert_eq!(op_array.line_end, 100);
    assert_eq!(op_array.filename, Some("test.php".to_string()));
}

#[test]
fn test_compile_string() {
    let result = compile_string("<?php echo 'hello';", "test.php");
    assert!(result.is_ok());
    let op_array = result.unwrap();
    assert_eq!(op_array.filename, Some("test.php".to_string()));
}

#[test]
fn test_compile_file_nonexistent() {
    let result = compile_file("/nonexistent/file.php");
    assert!(result.is_err());
}
