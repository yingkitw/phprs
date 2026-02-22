//! Primary expression parsing (literals, variables, new, ->, arrays, function calls)

use crate::engine::compile::context::CompileContext;
use crate::engine::facade::{self, StdValFactory, ValFactory};
use crate::engine::lexer::{Token, Lexer, TokenType};
use crate::engine::types::Val;
use crate::engine::vm::var_ref;
use super::helpers::*;

/// Parse primary expression (numbers, variables, strings, new, arrays, function calls)
pub(crate) fn parse_primary_expr(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    let token = lexer.next_token()?;
    context.set_line(token.lineno);

    match token.token_type {
        TokenType::T_LNUMBER => {
            let num_val = token.value.as_ref().unwrap().as_str().parse::<i64>().unwrap_or(0);
            let zval = facade::long_val(num_val);
            Ok((zval, lexer.next_token()?))
        }
        TokenType::T_DNUMBER => {
            let num_val = token.value.as_ref().unwrap().as_str().parse::<f64>().unwrap_or(0.0);
            let zval = facade::double_val(num_val);
            Ok((zval, lexer.next_token()?))
        }
        TokenType::T_CONSTANT_ENCAPSED_STRING => {
            let str_val = token.value.unwrap();
            let s = str_val.as_str();
            if s.contains('$') {
                let result = compile_interpolated_string(s, context)?;
                Ok((result, lexer.next_token()?))
            } else {
                let zval = Val::new(crate::engine::types::PhpValue::String(Box::new(str_val)), crate::engine::types::PhpType::String); // PhpString clone, not &str
                Ok((zval, lexer.next_token()?))
            }
        }
        TokenType::T_NEW => compile_new_obj(lexer, context),
        TokenType::T_MATCH => parse_match_expression(lexer, context),
        TokenType::T_YIELD => {
            let (yield_val, next) = super::parse_expression(lexer, context)?;
            Ok((yield_val, next))
        }
        TokenType::T_VARIABLE => {
            let name = token.value.as_ref().unwrap().as_str();
            let result = var_ref(name);
            let next = lexer.next_token()?;
            parse_access_chain(lexer, context, result, next)
        }
        TokenType::T_STRING => {
            let val = token.value.as_ref().map(|s| s.as_str()).unwrap_or("");
            if val == "[" {
                let result = parse_array_literal(lexer, context)?;
                Ok((result, lexer.next_token()?))
            } else if val == "(" {
                let (inner, close_token) = super::parse_expression(lexer, context)?;
                if token_is_punct(&close_token, ")") {
                    Ok((inner, lexer.next_token()?))
                } else {
                    Err("Expected ')' after parenthesized expression".to_string())
                }
            } else if val == "true" {
                Ok((StdValFactory::bool_val(true), lexer.next_token()?))
            } else if val == "false" {
                Ok((StdValFactory::bool_val(false), lexer.next_token()?))
            } else if val == "null" {
                Ok((facade::null_val(), lexer.next_token()?))
            } else {
                let next_token = lexer.next_token()?;
                if token_is_punct(&next_token, "(") {
                    parse_function_call(lexer, context, val)
                } else {
                    // Bare identifier: treat as constant lookup (WordPress/PHP compatibility)
                    let result = super::helpers::compile_constant_lookup(context, val);
                    Ok((result, next_token))
                }
            }
        }
        _ => Err(format!("Unexpected token in expression: {:?}", token.token_type)),
    }
}
