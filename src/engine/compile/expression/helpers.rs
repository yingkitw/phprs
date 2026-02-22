//! Shared helpers for expression parsing — eliminates duplication (DRY)

use crate::engine::compile::context::CompileContext;
use crate::engine::facade::{self, StdValFactory, ValFactory};
use crate::engine::lexer::{Token, Lexer, TokenType};
use crate::engine::types::Val;
use crate::engine::vm::{temp_var_ref, var_ref, Opcode};

/// Check if token matches a specific punctuation string
pub(crate) fn token_is_punct(token: &Token, ch: &str) -> bool {
    token.token_type == TokenType::T_STRING && token.value.as_ref().map(|s| s.as_str()) == Some(ch)
}

pub(crate) fn token_is_keyword(token: &Token, keyword: &str) -> bool {
    token.token_type == TokenType::T_STRING && token.value.as_ref().map(|s| s.as_str()) == Some(keyword)
}

/// Convert a pre-consumed token into a primary Val
pub(crate) fn token_to_primary(tok: &Token) -> Result<Val, String> {
    match tok.token_type {
        TokenType::T_LNUMBER => {
            let num_val = tok.value.as_ref().unwrap().as_str().parse::<i64>().unwrap_or(0);
            Ok(facade::long_val(num_val))
        }
        TokenType::T_DNUMBER => {
            let num_val = tok.value.as_ref().unwrap().as_str().parse::<f64>().unwrap_or(0.0);
            Ok(facade::double_val(num_val))
        }
        TokenType::T_CONSTANT_ENCAPSED_STRING => {
            let str_val = tok.value.as_ref().unwrap().clone();
            Ok(Val::new(crate::engine::types::PhpValue::String(Box::new(str_val)), crate::engine::types::PhpType::String))
        }
        TokenType::T_VARIABLE => {
            let name = tok.value.as_ref().unwrap().as_str();
            Ok(var_ref(name))
        }
        TokenType::T_STRING => {
            let val = tok.value.as_ref().map(|s| s.as_str()).unwrap_or("");
            match val {
                "true" => Ok(StdValFactory::bool_val(true)),
                "false" => Ok(StdValFactory::bool_val(false)),
                "null" => Ok(facade::null_val()),
                _ => Ok(facade::null_val()),
            }
        }
        _ => Err(format!("Unexpected token in expression: {:?}", tok.token_type)),
    }
}

/// Emit a binary operator opcode and return the result temp ref
pub(crate) fn emit_binary_op(
    context: &mut CompileContext,
    opcode: Opcode,
    left: Val,
    right: Val,
) -> Val {
    let slot = context.alloc_temp();
    context.emit_opcode(opcode, left, right, temp_var_ref(slot));
    temp_var_ref(slot)
}

pub(crate) fn parse_match_expression(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    let open_paren = lexer.next_token()?;
    if !token_is_punct(&open_paren, "(") {
        return Err("Expected '(' after 'match'".to_string());
    }

    let (match_value, close_paren) = super::parse_expression(lexer, context)?;
    if !token_is_punct(&close_paren, ")") {
        return Err("Expected ')' after match expression".to_string());
    }

    let open_brace = lexer.next_token()?;
    if !token_is_punct(&open_brace, "{") {
        return Err("Expected '{' after match expression".to_string());
    }

    let result_slot = context.alloc_temp();
    let mut end_jumps = Vec::new();
    let mut token = lexer.next_token()?;

    while !token_is_punct(&token, "}") {
        let is_default = token.token_type == TokenType::T_DEFAULT || token_is_keyword(&token, "default");
        let (condition, next) = if is_default {
            (facade::null_val(), lexer.next_token()?)
        } else {
            super::operators::parse_additive_expr_with_initial(lexer, context, token)?
        };

        if next.token_type != TokenType::T_DOUBLE_ARROW {
            return Err("Expected '=>' in match arm".to_string());
        }

        if is_default {
            let (value, after) = super::parse_expression(lexer, context)?;
            context.emit_opcode(
                Opcode::QmAssign,
                value,
                facade::null_val(),
                temp_var_ref(result_slot),
            );
            token = after;
        } else {
            let cmp = emit_binary_op(context, Opcode::IsEqual, facade::clone_val(&match_value), condition);
            let jmp_idx = context.emit_opcode_with_index(
                Opcode::JmpZ,
                cmp,
                facade::null_val(),
                facade::null_val(),
            );
            let (value, after) = super::parse_expression(lexer, context)?;
            context.emit_opcode(
                Opcode::QmAssign,
                value,
                facade::null_val(),
                temp_var_ref(result_slot),
            );
            let end_jmp = context.emit_opcode_with_index(
                Opcode::Jmp,
                facade::null_val(),
                facade::null_val(),
                facade::null_val(),
            );
            context.update_jump_target(jmp_idx, context.current_op_index() as u32);
            end_jumps.push(end_jmp);
            token = after;
        }

        if token_is_punct(&token, ",") {
            token = lexer.next_token()?;
        } else if !token_is_punct(&token, "}") {
            return Err("Expected ',' or '}' after match arm".to_string());
        }
    }

    let end_idx = context.current_op_index();
    for jmp in end_jumps {
        context.update_jump_target(jmp, end_idx as u32);
    }

    Ok((temp_var_ref(result_slot), lexer.next_token()?))
}

/// Parse the object access chain: $var[idx], $var->prop, $var->method(args...)
/// Shared between parse_primary_expr and parse_multiplicative_expr_with_initial
pub(crate) fn parse_access_chain(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    mut result: Val,
    mut next: Token,
) -> Result<(Val, Token), String> {
    loop {
        if token_is_punct(&next, "[") {
            // Array access
            let (idx_zval, close_token) = super::parse_expression(lexer, context)?;
            if !token_is_punct(&close_token, "]") {
                return Err("Expected ']' after array index".to_string());
            }
            result = emit_binary_op(context, Opcode::FetchDim, result, idx_zval);
            next = lexer.next_token()?;
        } else if next.token_type == TokenType::T_OBJECT_OPERATOR {
            let (new_result, new_next) = parse_object_access(lexer, context, result)?;
            result = new_result;
            next = new_next;
        } else if token_is_punct(&next, "(") {
            // Callable variable: $var(args...)
            let (call_result, call_next) = parse_callable_var(lexer, context, result)?;
            result = call_result;
            next = call_next;
        } else {
            break;
        }
    }
    Ok((result, next))
}

/// Parse object property access or method call after '->'
fn parse_object_access(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    obj: Val,
) -> Result<(Val, Token), String> {
    let member_token = lexer.next_token()?;
    let member_name = member_token.value.as_ref()
        .ok_or("Expected property/method name after '->'")?
        .as_str();
    let member_zval = facade::string_val(member_name);

    let peek = lexer.next_token()?;
    if token_is_punct(&peek, "(") {
        // Method call: $obj->method(args...)
        parse_method_call(lexer, context, obj, member_zval)
    } else {
        // Property access: $obj->prop
        let result = emit_binary_op(context, Opcode::FetchObjProp, obj, member_zval);
        Ok((result, peek))
    }
}

/// Parse method call arguments and emit InitMethodCall + SendVal + DoMethodCall
pub(crate) fn parse_method_call(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    obj: Val,
    member_zval: Val,
) -> Result<(Val, Token), String> {
    context.emit_opcode(
        Opcode::InitMethodCall,
        crate::engine::facade::clone_val(&obj),
        crate::engine::facade::clone_val(&member_zval),
        facade::null_val(),
    );

    // Parse arguments
    let mut arg_token = lexer.next_token()?;
    while !token_is_punct(&arg_token, ")") {
        let (arg_val, after_arg) = super::operators::parse_additive_expr_with_initial(lexer, context, arg_token)?;
        context.emit_opcode(Opcode::SendVal, arg_val, facade::null_val(), facade::null_val());
        if token_is_punct(&after_arg, ",") {
            arg_token = lexer.next_token()?;
        } else {
            arg_token = after_arg;
        }
    }

    let call_slot = context.alloc_temp();
    context.emit_opcode(Opcode::DoMethodCall, member_zval, obj, temp_var_ref(call_slot));
    Ok((temp_var_ref(call_slot), lexer.next_token()?))
}

/// Parse callable variable: $var(args...) — the opening '(' has already been consumed
fn parse_callable_var(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    callable: Val,
) -> Result<(Val, Token), String> {
    let mut args = Vec::new();
    let mut current_token = lexer.next_token()?;

    if !token_is_punct(&current_token, ")") {
        let (arg_val, next_token) = super::operators::parse_additive_expr_with_initial(lexer, context, current_token)?;
        args.push(arg_val);
        current_token = next_token;

        while token_is_punct(&current_token, ",") {
            let (arg_val, next_token) = super::parse_expression(lexer, context)?;
            args.push(arg_val);
            current_token = next_token;
        }

        if !token_is_punct(&current_token, ")") {
            return Err("Expected ',' or ')' after callable argument".to_string());
        }
    }

    // Emit InitFCall
    context.emit_opcode(Opcode::InitFCall, facade::null_val(), facade::null_val(), facade::null_val());

    // Emit SendVal for each argument
    for arg in args {
        context.emit_opcode(Opcode::SendVal, arg, facade::null_val(), facade::null_val());
    }

    // Emit DoFCall with the callable variable as the function name
    let call_slot = context.alloc_temp();
    context.emit_opcode(Opcode::DoFCall, callable, facade::null_val(), temp_var_ref(call_slot));
    Ok((temp_var_ref(call_slot), lexer.next_token()?))
}

/// Compile `new ClassName()` — shared between parse_primary_expr and _with_initial
pub(crate) fn compile_new_obj(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    let class_token = lexer.next_token()?;
    let class_name = class_token.value.as_ref()
        .ok_or("Expected class name after 'new'")?
        .as_str();
    let resolved_name = context.resolve_class_name(class_name);
    let class_zval = facade::string_val(&resolved_name);
    let slot = context.alloc_temp();
    context.emit_opcode(Opcode::NewObj, class_zval, facade::null_val(), temp_var_ref(slot));

    // Check for constructor args: (...)
    let peek = lexer.next_token()?;
    if token_is_punct(&peek, "(") {
        // TODO: pass constructor args via SendVal + DoMethodCall(__construct)
        let mut depth = 1;
        let mut nt = lexer.next_token()?;
        while depth > 0 {
            if token_is_punct(&nt, "(") { depth += 1; }
            if token_is_punct(&nt, ")") { depth -= 1; }
            if depth > 0 { nt = lexer.next_token()?; }
        }
        Ok((temp_var_ref(slot), lexer.next_token()?))
    } else {
        Ok((temp_var_ref(slot), peek))
    }
}

/// Run the multiplicative operator loop (* / %) on an already-parsed left value
pub(crate) fn multiplicative_loop(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    mut left: Val,
    mut token: Token,
) -> Result<(Val, Token), String> {
    while token.token_type == TokenType::T_MUL
        || token.token_type == TokenType::T_DIV
        || token.token_type == TokenType::T_MOD
    {
        let op = token.token_type;
        let (right, next_token) = super::primary::parse_primary_expr(lexer, context)?;
        let opcode = match op {
            TokenType::T_MUL => Opcode::Mul,
            TokenType::T_DIV => Opcode::Div,
            TokenType::T_MOD => Opcode::Mod,
            _ => return Err("Unexpected operator".to_string()),
        };
        left = emit_binary_op(context, opcode, left, right);
        token = next_token;
    }
    Ok((left, token))
}

/// Run the additive operator loop (+ - .) on an already-parsed left value
pub(crate) fn additive_loop(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    mut left: Val,
    mut token: Token,
) -> Result<(Val, Token), String> {
    while token.token_type == TokenType::T_PLUS
        || token.token_type == TokenType::T_MINUS
        || token.token_type == TokenType::T_CONCAT
    {
        let op = token.token_type;
        let (right, next_token) = super::operators::parse_multiplicative_expr(lexer, context)?;
        let opcode = match op {
            TokenType::T_PLUS => Opcode::Add,
            TokenType::T_MINUS => Opcode::Sub,
            TokenType::T_CONCAT => Opcode::Concat,
            _ => return Err("Unexpected operator".to_string()),
        };
        left = emit_binary_op(context, opcode, left, right);
        token = next_token;
    }
    Ok((left, token))
}

/// Compile an interpolated string like "Hello $name, you are $age years old"
pub(crate) fn compile_interpolated_string(s: &str, context: &mut CompileContext) -> Result<Val, String> {
    let bytes = s.as_bytes();
    let mut parts: Vec<Val> = Vec::new();
    let mut i = 0;
    let mut text_start = 0;

    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_') {
            if i > text_start {
                parts.push(facade::string_val(&s[text_start..i]));
            }
            let var_start = i + 1;
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            parts.push(var_ref(&format!("${}", &s[var_start..i])));
            text_start = i;
        } else {
            i += 1;
        }
    }
    if text_start < bytes.len() {
        parts.push(facade::string_val(&s[text_start..]));
    }

    if parts.is_empty() {
        return Ok(facade::string_val(""));
    }
    if parts.len() == 1 {
        return Ok(parts.remove(0));
    }

    let mut result = parts.remove(0);
    for part in parts {
        result = emit_binary_op(context, Opcode::Concat, result, part);
    }
    Ok(result)
}

/// Parse array literal: [elem1, elem2, ...] or ['key' => val, ...]
/// The opening '[' has already been consumed
pub(crate) fn parse_array_literal(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Val, String> {
    let arr_slot = context.alloc_temp();
    context.emit_opcode(Opcode::InitArray, facade::null_val(), facade::null_val(), temp_var_ref(arr_slot));

    let mut next = lexer.next_token()?;
    if !token_is_punct(&next, "]") {
        loop {
            let (val_zval, after_val) = super::operators::parse_additive_expr_with_initial(lexer, context, next)?;

            if after_val.token_type == TokenType::T_DOUBLE_ARROW {
                // key => value
                let key_zval = val_zval;
                let (value_zval, after_value) = super::parse_expression(lexer, context)?;
                context.emit_opcode(Opcode::AddArrayElement, temp_var_ref(arr_slot), value_zval, key_zval);
                let last_idx = context.current_op_index() - 1;
                context.update_jump_target(last_idx, 1);
                next = after_value;
            } else {
                context.emit_opcode(Opcode::AddArrayElement, temp_var_ref(arr_slot), val_zval, facade::null_val());
                next = after_val;
            }

            if token_is_punct(&next, "]") { break; }
            if token_is_punct(&next, ",") {
                next = lexer.next_token()?;
                if token_is_punct(&next, "]") { break; }
                continue;
            }
            return Err("Expected ',' or ']' in array literal".to_string());
        }
    }
    Ok(temp_var_ref(arr_slot))
}

/// Parse function call: function_name(arg1, arg2, ...)
/// The opening '(' has already been consumed
pub(crate) fn parse_function_call(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    function_name: &str,
) -> Result<(Val, Token), String> {
    let mut args = Vec::new();
    let mut current_token = lexer.next_token()?;

    if !token_is_punct(&current_token, ")") {
        // First argument: already consumed its first token
        let (arg_zval, next_token) = super::operators::parse_additive_expr_with_initial(lexer, context, current_token)?;
        args.push(arg_zval);
        current_token = next_token;

        while token_is_punct(&current_token, ",") {
            let (arg_zval, next_token) = super::parse_expression(lexer, context)?;
            args.push(arg_zval);
            current_token = next_token;
        }

        if !token_is_punct(&current_token, ")") {
            return Err("Expected ',' or ')' after function argument".to_string());
        }
    }

    // Emit InitFCall
    context.emit_opcode(Opcode::InitFCall, facade::null_val(), facade::null_val(), facade::null_val());

    // Emit SendVal for each argument
    for arg in args {
        context.emit_opcode(Opcode::SendVal, arg, facade::null_val(), facade::null_val());
    }

    // Emit DoFCall
    let func_name_zval = facade::string_val(function_name);
    let result_slot = context.alloc_temp();
    context.emit_opcode(Opcode::DoFCall, func_name_zval, facade::null_val(), temp_var_ref(result_slot));

    Ok((temp_var_ref(result_slot), lexer.next_token()?))
}

/// Emit constant(name) lookup for bare identifiers (WordPress/PHP compatibility)
pub(crate) fn compile_constant_lookup(context: &mut CompileContext, name: &str) -> Val {
    let slot = context.alloc_temp();
    context.emit_opcode(Opcode::InitFCall, facade::null_val(), facade::null_val(), facade::null_val());
    context.emit_opcode(Opcode::SendVal, facade::string_val(name), facade::null_val(), facade::null_val());
    context.emit_opcode(Opcode::DoFCall, facade::string_val("constant"), facade::null_val(), temp_var_ref(slot));
    temp_var_ref(slot)
}
