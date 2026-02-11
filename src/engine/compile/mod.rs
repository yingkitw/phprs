//! Compiler
//!
//! Compilation
//!
//! This module handles compilation of PHP code to opcodes

#[cfg(test)]
mod tests;

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
    let mut context = CompileContext::new();
    context.set_filename(filename);

    // Tokenize the code
    let mut lexer = Lexer::new(code);

    // Skip opening tag if present
    let mut token = lexer.next_token()?;
    if token.token_type == TokenType::T_OPEN_TAG {
        token = lexer.next_token()?;
    }

    // Basic compilation: tokenize and emit simple opcodes
    while token.token_type != TokenType::T_EOF {
        token = parse_statement(&mut lexer, &mut context, token)?;
    }

    Ok(context.finalize())
}

/// Compile file to opcodes
pub fn compile_file(filename: &str) -> Result<OpArray, String> {
    use std::fs;
    let code = fs::read_to_string(filename).map_err(|e| format!("Failed to read file: {e}"))?;
    compile_string(&code, filename)
}
