//! Evaluate constant class member initializers at compile time.

use super::context::CompileContext;
use super::expression::parse_expression;
use crate::engine::facade::{clone_val, null_val};
use crate::engine::lexer::{Lexer, Token};
use crate::engine::types::Val;
use crate::engine::vm::execute::execute_ex_returning;
use crate::engine::vm::{ExecuteData, Opcode};

/// Parse and evaluate a class property/constant default expression into a materialized [`Val`].
pub(crate) fn eval_class_default_expr(
    lexer: &mut Lexer,
    parent: &CompileContext,
) -> Result<(Val, Token), String> {
    let mut child = CompileContext::new();
    child.current_namespace = parent.current_namespace.clone();
    child.use_imports = parent.use_imports.clone();

    let (expr_val, after) = parse_expression(lexer, &mut child)?;
    child.emit_opcode(Opcode::Return, expr_val, null_val(), null_val());

    let mut ed = ExecuteData::new();
    let (status, ret) = execute_ex_returning(&mut ed, &child.op_array);
    match ret {
        Some(v) if matches!(status, crate::engine::types::PhpResult::Success) => {
            Ok((clone_val(&v), after))
        }
        _ => Err("Class member default must be a constant expression".to_string()),
    }
}
