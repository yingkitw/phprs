//! Operator expression parsing (logical, comparison, arithmetic chains)

use crate::engine::compile::context::CompileContext;
use crate::engine::facade::{self, result_val};
use crate::engine::lexer::{Token, Lexer, TokenType};
use crate::engine::types::Val;
use crate::engine::vm::Opcode;
use super::helpers::*;

/// Parse ternary expression (condition ? true_expr : false_expr)
/// Also handles short ternary (expr ?: default)
pub(crate) fn parse_ternary_expr(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    let (condition, token) = parse_null_coalesce_expr(lexer, context)?;

    // Check for ? (ternary)
    if !token_is_punct(&token, "?") {
        return Ok((condition, token));
    }

    // Peek at next token to distinguish short ternary (?:) from full ternary
    let peek = lexer.next_token()?;

    if token_is_punct(&peek, ":") {
        // Short ternary: expr ?: default
        let (fallback, after) = parse_ternary_expr(lexer, context)?;
        let slot = context.alloc_temp();
        // JmpNZ: if condition is truthy, jump to truthy path
        let jmp_idx = context.current_op_index();
        context.emit_opcode(
            Opcode::JmpNZ,
            facade::clone_val(&condition),
            facade::long_val(0), // placeholder target
            facade::null_val(),
        );
        // Falsy path: store fallback in temp slot
        context.emit_opcode(
            Opcode::QmAssign,
            fallback,
            facade::null_val(),
            crate::engine::vm::temp_var_ref(slot),
        );
        let jmp_end_idx = context.current_op_index();
        context.emit_opcode(
            Opcode::Jmp,
            facade::long_val(0), // placeholder
            facade::null_val(),
            facade::null_val(),
        );
        // Truthy path: store condition in temp slot
        let truthy_idx = context.current_op_index();
        context.emit_opcode(
            Opcode::QmAssign,
            condition,
            facade::null_val(),
            crate::engine::vm::temp_var_ref(slot),
        );
        let after_idx = context.current_op_index();
        // Patch jumps (use extended_value, which is what the VM reads)
        context.update_jump_target(jmp_idx, truthy_idx as u32);
        context.update_jump_target(jmp_end_idx, after_idx as u32);
        return Ok((crate::engine::vm::temp_var_ref(slot), after));
    }

    // Full ternary: condition ? true_expr : false_expr
    // `peek` is the first token of the true expression — need to parse with it
    let (true_expr, colon_token) = parse_additive_expr_with_initial(lexer, context, peek)?;
    if !token_is_punct(&colon_token, ":") {
        return Err(format!("Expected ':' in ternary expression, got {:?}", colon_token.token_type));
    }
    let (false_expr, after) = parse_ternary_expr(lexer, context)?;

    let slot = context.alloc_temp();
    // JmpZ: if condition is falsy, jump to false path
    let jmp_idx = context.current_op_index();
    context.emit_opcode(
        Opcode::JmpZ,
        facade::clone_val(&condition),
        facade::long_val(0), // placeholder
        facade::null_val(),
    );
    // True path: store in temp slot
    context.emit_opcode(
        Opcode::QmAssign,
        true_expr,
        facade::null_val(),
        crate::engine::vm::temp_var_ref(slot),
    );
    let jmp_end_idx = context.current_op_index();
    context.emit_opcode(
        Opcode::Jmp,
        facade::long_val(0), // placeholder
        facade::null_val(),
        facade::null_val(),
    );
    // False path: store in temp slot
    let false_start = context.current_op_index();
    context.emit_opcode(
        Opcode::QmAssign,
        false_expr,
        facade::null_val(),
        crate::engine::vm::temp_var_ref(slot),
    );
    let after_idx = context.current_op_index();
    // Patch jumps (use extended_value, which is what the VM reads)
    context.update_jump_target(jmp_idx, false_start as u32);
    context.update_jump_target(jmp_end_idx, after_idx as u32);

    Ok((crate::engine::vm::temp_var_ref(slot), after))
}

/// Parse null coalescing expression (??)
fn parse_null_coalesce_expr(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    let (left, token) = parse_logical_or_expr(lexer, context)?;

    if token.token_type == TokenType::T_COALESCE {
        // Right-associative: parse right side as another null coalesce
        let (right, next_token) = parse_null_coalesce_expr(lexer, context)?;
        let slot = context.alloc_temp();
        context.emit_opcode(
            Opcode::Coalesce,
            left,
            right,
            crate::engine::vm::temp_var_ref(slot),
        );
        return Ok((crate::engine::vm::temp_var_ref(slot), next_token));
    }

    Ok((left, token))
}

/// Parse logical OR expression (||)
pub(crate) fn parse_logical_or_expr(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    let (mut left, mut token) = parse_logical_and_expr(lexer, context)?;

    while token.token_type == TokenType::T_BOOLEAN_OR {
        let (right, next_token) = parse_logical_and_expr(lexer, context)?;
        let or_result = result_val(crate::engine::types::PhpType::Bool);
        let or_result_dup = result_val(crate::engine::types::PhpType::Bool);
        // TODO: Add BoolOr opcode, for now use BoolXor as placeholder
        context.emit_opcode(Opcode::BoolXor, left, right, or_result);
        left = or_result_dup;
        token = next_token;
    }

    Ok((left, token))
}

/// Parse logical AND expression (&&)
fn parse_logical_and_expr(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    let (mut left, mut token) = parse_logical_not_expr(lexer, context)?;

    while token.token_type == TokenType::T_BOOLEAN_AND {
        let (right, next_token) = parse_logical_not_expr(lexer, context)?;
        let and_result = result_val(crate::engine::types::PhpType::Bool);
        let and_result_dup = result_val(crate::engine::types::PhpType::Bool);
        // TODO: Add BoolAnd opcode, for now use BoolXor as placeholder
        context.emit_opcode(Opcode::BoolXor, left, right, and_result);
        left = and_result_dup;
        token = next_token;
    }

    Ok((left, token))
}

/// Parse logical NOT expression (!)
fn parse_logical_not_expr(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    let token = lexer.next_token()?;

    if token.token_type == TokenType::T_BOOLEAN_NOT {
        let (expr, next_token) = parse_logical_not_expr(lexer, context)?;
        let bool_result = result_val(crate::engine::types::PhpType::Bool);
        let bool_result_dup = result_val(crate::engine::types::PhpType::Bool);
        let zero = facade::zero_val();
        context.emit_opcode(Opcode::BoolNot, expr, zero, bool_result);
        return Ok((bool_result_dup, next_token));
    }

    // Not a logical NOT — parse comparison with pre-consumed token
    parse_comparison_expr_with_token(lexer, context, Some(token))
}

/// Parse comparison expression with optional pre-consumed token
fn parse_comparison_expr_with_token(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    initial_token: Option<Token>,
) -> Result<(Val, Token), String> {
    let (mut left, mut token) = if let Some(tok) = initial_token {
        parse_additive_expr_with_initial(lexer, context, tok)?
    } else {
        parse_additive_expr(lexer, context)?
    };

    loop {
        // Check for comparison operators
        let is_lt = token_is_punct(&token, "<");
        let is_gt = token_is_punct(&token, ">");
        let is_cmp = matches!(
            token.token_type,
            TokenType::T_IS_EQUAL
                | TokenType::T_IS_NOT_EQUAL
                | TokenType::T_IS_SMALLER_OR_EQUAL
                | TokenType::T_IS_GREATER_OR_EQUAL
                | TokenType::T_IS_IDENTICAL
                | TokenType::T_IS_NOT_IDENTICAL
        ) || is_lt || is_gt;
        if !is_cmp { break; }

        let (right, next_token) = parse_additive_expr(lexer, context)?;
        let (opcode, swap) = if is_lt {
            (Opcode::IsSmaller, false)
        } else if is_gt {
            (Opcode::IsSmaller, true) // swap operands
        } else {
            match token.token_type {
                TokenType::T_IS_EQUAL => (Opcode::IsEqual, false),
                TokenType::T_IS_NOT_EQUAL => (Opcode::IsNotEqual, false),
                TokenType::T_IS_SMALLER_OR_EQUAL => (Opcode::IsSmallerOrEqual, false),
                TokenType::T_IS_GREATER_OR_EQUAL => (Opcode::IsSmallerOrEqual, true),
                TokenType::T_IS_IDENTICAL => (Opcode::IsIdentical, false),
                TokenType::T_IS_NOT_IDENTICAL => (Opcode::IsNotIdentical, false),
                _ => unreachable!(),
            }
        };
        let (l, r) = if swap { (right, left) } else { (left, right) };
        left = emit_binary_op(context, opcode, l, r);
        token = next_token;
    }

    Ok((left, token))
}

/// Parse additive expression with a pre-consumed initial token
pub fn parse_additive_expr_with_initial(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    initial_token: Token,
) -> Result<(Val, Token), String> {
    let (left, token) = parse_multiplicative_expr_with_initial(lexer, context, initial_token)?;
    additive_loop(lexer, context, left, token)
}

/// Parse additive expression (+, -, .)
fn parse_additive_expr(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    let (left, token) = parse_multiplicative_expr(lexer, context)?;
    additive_loop(lexer, context, left, token)
}

/// Parse multiplicative expression with a pre-consumed initial token
fn parse_multiplicative_expr_with_initial(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    initial_token: Token,
) -> Result<(Val, Token), String> {
    // Handle interpolated strings
    if initial_token.token_type == TokenType::T_CONSTANT_ENCAPSED_STRING {
        let s = initial_token.value.as_ref().unwrap().as_str();
        if s.contains('$') {
            let result = compile_interpolated_string(s, context)?;
            let next = lexer.next_token()?;
            return multiplicative_loop(lexer, context, result, next);
        }
    }

    // Handle T_NEW
    if initial_token.token_type == TokenType::T_NEW {
        let (result, next) = compile_new_obj(lexer, context)?;
        return multiplicative_loop(lexer, context, result, next);
    }

    // Handle T_FUNCTION (closure in expression position)
    if initial_token.token_type == TokenType::T_FUNCTION {
        let (result, next) = crate::engine::compile::function::compile_closure(lexer, context)?;
        return multiplicative_loop(lexer, context, result, next);
    }

    // Convert token to primary Val
    let initial_zval = token_to_primary(&initial_token)?;

    // Handle special cases based on token type
    let (left, token) = if initial_token.token_type == TokenType::T_STRING {
        let val = initial_token.value.as_ref().map(|s| s.as_str()).unwrap_or("");
        if val == "(" {
            let (inner, close_token) = super::parse_expression(lexer, context)?;
            if token_is_punct(&close_token, ")") {
                (inner, lexer.next_token()?)
            } else {
                return Err("Expected ')' after parenthesized expression".to_string());
            }
        } else if val == "[" {
            let result = parse_array_literal(lexer, context)?;
            (result, lexer.next_token()?)
        } else {
            let next = lexer.next_token()?;
            if token_is_punct(&next, "(") {
                let fname = initial_token.value.as_ref().unwrap().as_str();
                parse_function_call(lexer, context, fname)?
            } else {
                (initial_zval, next)
            }
        }
    } else if initial_token.token_type == TokenType::T_VARIABLE {
        let next = lexer.next_token()?;
        parse_access_chain(lexer, context, initial_zval, next)?
    } else {
        (initial_zval, lexer.next_token()?)
    };

    multiplicative_loop(lexer, context, left, token)
}

/// Parse multiplicative expression (*, /, %)
pub(crate) fn parse_multiplicative_expr(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    let (left, token) = super::primary::parse_primary_expr(lexer, context)?;
    multiplicative_loop(lexer, context, left, token)
}
