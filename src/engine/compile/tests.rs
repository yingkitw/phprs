//! Unit tests for compiler

use crate::engine::compile::{CompileContext, compile_file, compile_string};
use crate::engine::types::Val;
use crate::engine::vm::Opcode;

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
fn test_parse_primary_strlen_first_class_callable() {
    use crate::engine::compile::expression::primary::parse_primary_expr;
    use crate::engine::lexer::Lexer;
    use crate::engine::types::PhpType;

    let mut context = CompileContext::new();
    let mut lexer = Lexer::new("strlen(...)");
    let (val, _) = parse_primary_expr(&mut lexer, &mut context).expect("parse");
    assert_eq!(
        val.get_type(),
        PhpType::Callable,
        "expected Callable from strlen(...), got {:?}",
        val.get_type()
    );
}

#[test]
fn test_lexer_strlen_fcc_tokens() {
    use crate::engine::lexer::{Lexer, TokenType};

    let mut lexer = Lexer::new("strlen(...)");
    assert_eq!(lexer.next_token().unwrap().token_type, TokenType::T_STRING);
    assert_eq!(lexer.next_token().unwrap().token_type, TokenType::T_STRING);
    assert_eq!(
        lexer.next_token().unwrap().token_type,
        TokenType::T_ELLIPSIS
    );
    assert_eq!(lexer.next_token().unwrap().token_type, TokenType::T_STRING);
}

#[test]
fn test_compile_first_class_callable_assignment() {
    use crate::engine::compile::compile_string_with_functions;
    use crate::engine::types::PhpType;

    let (op_array, _) =
        compile_string_with_functions("$fn = strlen(...);", "test.php").expect("compile");
    let assign = op_array
        .ops
        .iter()
        .find(|op| op.opcode == Opcode::Assign)
        .expect("assign opcode");
    assert_eq!(
        assign.op2.get_type(),
        PhpType::Callable,
        "expected Callable on RHS, got {:?}",
        assign.op2.get_type()
    );
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
