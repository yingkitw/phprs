//! Statement Parsing
//!
//! Handles parsing of PHP statements (echo, return, variable assignment, etc.)
//!
//! ## Module Structure
//! - `oop` — Class definition compilation

mod oop;

use super::context::CompileContext;
use super::control_flow::{compile_for, compile_foreach, compile_if, compile_throw, compile_try_catch, compile_while};
use super::expression::{parse_expression, parse_additive_expr_with_initial};
use super::expression::helpers::token_is_punct;
use super::function::compile_function;
use crate::engine::facade::{null_val, long_val, result_val, string_val, string_val_copy, zero_val, clone_val, StdValFactory, ValFactory};
use crate::engine::lexer::{Token, Lexer, TokenType};
use crate::engine::vm::Opcode;

/// Helper: consume semicolon if present, return next token
fn skip_semicolon(lexer: &mut Lexer, token: Token) -> Result<Token, String> {
    if token_is_punct(&token, ";") {
        lexer.next_token()
    } else {
        Ok(token)
    }
}

/// Parse a statement block (statements between braces)
/// The opening brace should already be consumed
pub fn parse_statement_block(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(), String> {
    let mut brace_count = 1;
    let mut current_token = lexer.next_token()?;

    loop {
        if current_token.token_type == TokenType::T_EOF {
            return Err("Unexpected EOF in statement block".to_string());
        }

        if let Some(s) = current_token.value.as_ref() {
            if current_token.token_type == TokenType::T_STRING {
                if s.as_str() == "}" {
                    brace_count -= 1;
                    if brace_count == 0 { return Ok(()); }
                    current_token = lexer.next_token()?;
                    continue;
                } else if s.as_str() == "{" {
                    brace_count += 1;
                }
            }
        }

        current_token = parse_statement(lexer, context, current_token)?;
    }
}

/// Parse a single statement from the lexer
/// Returns the next token after the statement
pub fn parse_statement(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    token: Token,
) -> Result<Token, String> {
    context.set_line(token.lineno);

    match token.token_type {
        TokenType::T_ECHO => compile_echo(lexer, context),
        TokenType::T_VARIABLE => compile_variable_stmt(lexer, context, &token),
        TokenType::T_LNUMBER | TokenType::T_DNUMBER => Ok(lexer.next_token()?),
        TokenType::T_CONSTANT_ENCAPSED_STRING => Ok(lexer.next_token()?),
        TokenType::T_STRING => compile_string_stmt(lexer, context, &token),
        TokenType::T_PLUS | TokenType::T_MINUS | TokenType::T_MUL | TokenType::T_DIV | TokenType::T_MOD => {
            Ok(lexer.next_token()?)
        }
        TokenType::T_INCLUDE | TokenType::T_INCLUDE_ONCE | TokenType::T_REQUIRE | TokenType::T_REQUIRE_ONCE => {
            compile_include(lexer, context, &token)
        }
        TokenType::T_IF => compile_if(lexer, context),
        TokenType::T_WHILE => compile_while(lexer, context),
        TokenType::T_FOR => compile_for(lexer, context),
        TokenType::T_FOREACH => compile_foreach(lexer, context),
        TokenType::T_TRY => compile_try_catch(lexer, context),
        TokenType::T_THROW => compile_throw(lexer, context),
        TokenType::T_FUNCTION => compile_function(lexer, context),
        TokenType::T_CLASS => oop::compile_class(lexer, context),
        TokenType::T_RETURN => compile_return(lexer, context),
        _ => Ok(lexer.next_token()?),
    }
}

/// Compile echo statement
fn compile_echo(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    let (echo_value, _) = parse_expression(lexer, context)?;
    let zval_zero = zero_val();
    let zval_result = match &echo_value.value {
        crate::engine::types::PhpValue::String(s) => string_val_copy(s.as_str(), echo_value.get_type()),
        _ => result_val(echo_value.get_type()),
    };
    context.emit_opcode(Opcode::Echo, echo_value, zval_zero, zval_result);
    let next = lexer.next_token()?;
    skip_semicolon(lexer, next)
}

/// Compile variable statement ($var = expr, $var->method(), $var->prop = expr)
fn compile_variable_stmt(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    token: &Token,
) -> Result<Token, String> {
    let var_name = token.value.as_ref().unwrap().as_str();
    let next_token = lexer.next_token()?;

    if next_token.token_type == TokenType::T_EQUAL {
        // Variable assignment: $var = expression
        let (value_zval, after_expr) = parse_expression(lexer, context)?;
        let var_name_zval = string_val(var_name);
        let value_zval_op2 = StdValFactory::clone_val(&value_zval);
        context.emit_opcode(Opcode::Assign, var_name_zval, value_zval, value_zval_op2);
        skip_semicolon(lexer, after_expr)
    } else if token_is_punct(&next_token, "(") {
        // Callable variable: $var(args...)
        let var_zval = crate::engine::vm::var_ref(var_name);
        context.emit_opcode(Opcode::InitFCall, null_val(), null_val(), null_val());
        // Parse args using the same pattern as function calls
        let mut arg_token = lexer.next_token()?;
        while !token_is_punct(&arg_token, ")") {
            let (arg_val, after_arg) = crate::engine::compile::expression::parse_additive_expr_with_initial(lexer, context, arg_token)?;
            context.emit_opcode(Opcode::SendVal, arg_val, null_val(), null_val());
            if token_is_punct(&after_arg, ",") {
                arg_token = lexer.next_token()?;
            } else {
                arg_token = after_arg;
            }
        }
        let call_slot = context.alloc_temp();
        context.emit_opcode(Opcode::DoFCall, var_zval, null_val(), crate::engine::vm::temp_var_ref(call_slot));
        let next = lexer.next_token()?;
        skip_semicolon(lexer, next)
    } else if next_token.token_type == TokenType::T_OBJECT_OPERATOR {
        compile_object_stmt(lexer, context, var_name)
    } else {
        Ok(next_token)
    }
}

/// Compile $var->method() or $var->prop = expr statement
fn compile_object_stmt(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    var_name: &str,
) -> Result<Token, String> {
    let member_token = lexer.next_token()?;
    let member_name = member_token.value.as_ref()
        .ok_or("Expected property/method name after '->'")?
        .as_str();
    let member_zval = string_val(member_name);

    let peek = lexer.next_token()?;
    if token_is_punct(&peek, "(") {
        // Method call statement: $var->method(args...);
        let var_zval = crate::engine::vm::var_ref(var_name);
        context.emit_opcode(
            Opcode::InitMethodCall,
            clone_val(&var_zval),
            clone_val(&member_zval),
            zero_val(),
        );
        let mut arg_token = lexer.next_token()?;
        while !token_is_punct(&arg_token, ")") {
            let (arg_val, after_arg) = parse_additive_expr_with_initial(lexer, context, arg_token)?;
            context.emit_opcode(Opcode::SendVal, arg_val, zero_val(), zero_val());
            if token_is_punct(&after_arg, ",") {
                arg_token = lexer.next_token()?;
            } else {
                arg_token = after_arg;
            }
        }
        let call_slot = context.alloc_temp();
        context.emit_opcode(
            Opcode::DoMethodCall,
            member_zval,
            var_zval,
            crate::engine::vm::temp_var_ref(call_slot),
        );
        let next = lexer.next_token()?;
        skip_semicolon(lexer, next)
    } else if peek.token_type == TokenType::T_EQUAL {
        // Property assignment: $var->prop = expr;
        let var_zval = crate::engine::vm::var_ref(var_name);
        let (value, _) = parse_expression(lexer, context)?;
        context.emit_opcode(Opcode::AssignObjProp, var_zval, member_zval, value);
        let next = lexer.next_token()?;
        skip_semicolon(lexer, next)
    } else {
        Ok(peek)
    }
}

/// Compile T_STRING statement (function call or punctuation)
fn compile_string_stmt(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    token: &Token,
) -> Result<Token, String> {
    let val = token.value.as_ref().map(|s| s.as_str()).unwrap_or("");
    if val == ";" || val == "{" || val == "}" || val == ")" || val == "(" {
        return Ok(lexer.next_token()?);
    }
    let next = lexer.next_token()?;
    if token_is_punct(&next, "(") {
        let (_, after_token) = super::expression::parse_function_call_public(lexer, context, val)?;
        skip_semicolon(lexer, after_token)
    } else {
        Ok(next)
    }
}

/// Compile include/require statement
fn compile_include(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    token: &Token,
) -> Result<Token, String> {
    let include_type: u32 = match token.token_type {
        TokenType::T_INCLUDE => 0,
        TokenType::T_REQUIRE => 1,
        TokenType::T_INCLUDE_ONCE => 2,
        TokenType::T_REQUIRE_ONCE => 3,
        _ => 0,
    };
    let (path_zval, _) = parse_expression(lexer, context)?;
    let z1 = null_val();
    let z2 = null_val();
    let idx = context.emit_opcode_with_index(Opcode::Include, path_zval, z1, z2);
    context.update_jump_target(idx, include_type);
    let next = lexer.next_token()?;
    skip_semicolon(lexer, next)
}

/// Compile return statement
fn compile_return(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    let peek = lexer.next_token()?;
    // Check for bare `return;` (no expression)
    if token_is_punct(&peek, ";") {
        context.emit_opcode(Opcode::Return, null_val(), null_val(), null_val());
        return lexer.next_token().map(Ok)?;
    }
    // Parse the return expression
    let (return_value, after) = crate::engine::compile::expression::parse_additive_expr_with_initial(lexer, context, peek)?;
    context.emit_opcode(Opcode::Return, return_value, null_val(), null_val());
    skip_semicolon(lexer, after)
}
