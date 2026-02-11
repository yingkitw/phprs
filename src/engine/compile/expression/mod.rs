//! Expression Parsing
//!
//! Handles parsing of PHP expressions (logical, comparison, arithmetic)
//!
//! ## Module Structure
//! - `operators` — Operator chain (logical → comparison → additive → multiplicative)
//! - `primary` — Primary expressions (literals, variables, new, ->, arrays, function calls)
//! - `helpers` — Shared utilities (DRY helpers for access chains, method calls, etc.)

pub(crate) mod helpers;
pub(crate) mod operators;
pub(crate) mod primary;

use crate::engine::compile::context::CompileContext;
use crate::engine::lexer::{Token, Lexer};
use crate::engine::types::Val;

/// Parse a simple expression (logical, comparison, arithmetic operations)
/// Returns the Val representing the result and the next token
pub fn parse_expression(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    operators::parse_logical_or_expr(lexer, context)
}

/// Parse additive expression with a pre-consumed initial token (public API)
pub fn parse_additive_expr_with_initial(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    initial_token: Token,
) -> Result<(Val, Token), String> {
    operators::parse_additive_expr_with_initial(lexer, context, initial_token)
}

/// Public wrapper for parse_function_call (used by statement parser)
pub fn parse_function_call_public(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    function_name: &str,
) -> Result<(Val, Token), String> {
    helpers::parse_function_call(lexer, context, function_name)
}
