//! Compiler
//!
//! Compilation
//!
//! This module handles compilation of PHP code to opcodes

#[cfg(test)]
mod tests;

pub mod const_eval;
pub mod context;
pub mod control_flow;
pub mod expression;
pub mod function;
pub mod function_table;
pub mod statement;

use crate::engine::lexer::{Lexer, TokenType};
use crate::engine::vm::OpArray;
pub use context::CompileContext;
use statement::parse_statement;

/// Compile PHP code string to opcodes
pub fn compile_string(code: &str, filename: &str) -> Result<OpArray, String> {
    let (op_array, _) = compile_string_with_functions(code, filename)?;
    Ok(op_array)
}

/// Compile PHP code string to opcodes and return the function table
pub fn compile_string_with_functions(
    code: &str,
    filename: &str,
) -> Result<(OpArray, function_table::FunctionTable), String> {
    let mut context = CompileContext::new();
    context.set_filename(filename);

    let mut lexer = Lexer::new(code);

    // Skip opening tag if present
    let mut token = lexer.next_token()?;
    if token.token_type == TokenType::T_OPEN_TAG {
        token = lexer.next_token()?;
    }

    while token.token_type != TokenType::T_EOF {
        token = parse_statement(&mut lexer, &mut context, token)?;
    }

    let ft = std::mem::take(&mut context.function_table);
    Ok((context.finalize(), ft))
}

/// Compile file to opcodes
pub fn compile_file(filename: &str) -> Result<OpArray, String> {
    let (op_array, _) = compile_file_with_functions(filename)?;
    Ok(op_array)
}

/// Compile file to opcodes and return the function table
pub fn compile_file_with_functions(
    filename: &str,
) -> Result<(OpArray, function_table::FunctionTable), String> {
    use std::fs;
    let code = fs::read_to_string(filename).map_err(|e| format!("Failed to read file: {e}"))?;
    compile_string_with_functions(&code, filename)
}
