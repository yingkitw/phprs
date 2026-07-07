//! Statement Parsing
//!
//! Handles parsing of PHP statements (echo, return, variable assignment, etc.)
//!
//! ## Module Structure
//! - `oop` — Class definition compilation

pub(crate) mod oop;

use super::context::CompileContext;
use super::control_flow::{compile_for, compile_foreach, compile_if, compile_throw, compile_try_catch, compile_while};
use super::expression::{parse_expression, parse_additive_expr_with_initial};
use super::expression::helpers::token_is_punct;
use super::function::compile_function;
use crate::engine::facade::{null_val, result_val, string_val, string_val_copy, zero_val, clone_val, StdValFactory, ValFactory};
use crate::engine::types::Val;
use crate::engine::lexer::{Token, Lexer, TokenType};
use crate::engine::vm::{Opcode, temp_var_ref, var_ref};

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
    let mut last_token_type = current_token.token_type;
    let mut same_token_count = 0;

    loop {
        if current_token.token_type == TokenType::T_EOF {
            return Err("Unexpected EOF in statement block".to_string());
        }

        // Safety check: prevent infinite loops
        if current_token.token_type == last_token_type {
            same_token_count += 1;
            if same_token_count > 100 {
                return Err(format!("Infinite loop detected in statement block at token {:?}", current_token.token_type));
            }
        } else {
            same_token_count = 0;
            last_token_type = current_token.token_type;
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
        TokenType::T_ATTRIBUTE => {
            let next = skip_attribute_block(lexer)?;
            parse_statement(lexer, context, next)
        }
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
        TokenType::T_ENUM => oop::compile_enum(lexer, context),
        TokenType::T_TRAIT => oop::compile_trait(lexer, context),
        TokenType::T_RETURN => compile_return(lexer, context),
        TokenType::T_GLOBAL => compile_global(lexer, context),
        TokenType::T_YIELD => compile_yield(lexer, context),
        TokenType::T_NAMESPACE => compile_namespace(lexer, context),
        TokenType::T_USE => compile_use(lexer, context),
        _ => Ok(lexer.next_token()?),
    }
}

pub(crate) fn skip_attribute_block(lexer: &mut Lexer) -> Result<Token, String> {
    let mut depth = 1;
    let mut token = lexer.next_token()?;
    while depth > 0 {
        if token.token_type == TokenType::T_ATTRIBUTE {
            depth += 1;
        } else if token_is_punct(&token, "]") {
            depth -= 1;
            if depth == 0 {
                return Ok(lexer.next_token()?);
            }
        }
        token = lexer.next_token()?;
    }
    Ok(token)
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

    if next_token.token_type == TokenType::T_INC || next_token.token_type == TokenType::T_DEC {
        let initial_zval = super::expression::helpers::token_to_primary(token, context)?;
        let (_v, after) = super::expression::helpers::parse_access_chain(
            lexer,
            context,
            initial_zval,
            next_token,
        )?;
        return skip_semicolon(lexer, after);
    }

    if next_token.token_type == TokenType::T_EQUAL {
        // Variable assignment: $var = expression
        let (value_zval, after_expr) = parse_expression(lexer, context)?;
        let var_name_zval = string_val(var_name);
        let value_zval_op2 = StdValFactory::clone_val(&value_zval);
        context.emit_opcode(Opcode::Assign, var_name_zval, value_zval, value_zval_op2);
        skip_semicolon(lexer, after_expr)
    } else if token_is_punct(&next_token, "[") {
        // $var[key] = value;  $var[a][b] = value;  $var[] = value;
        let (keys, after_target) = parse_dim_assign_keys(lexer, context, next_token)?;
        if after_target.token_type == TokenType::T_EQUAL {
            let (value_zval, after_value) = parse_expression(lexer, context)?;
            emit_dim_assignment(context, var_name, &keys, value_zval)?;
            skip_semicolon(lexer, after_value)
        } else if after_target.token_type == TokenType::T_INC
            || after_target.token_type == TokenType::T_DEC
        {
            let is_inc = after_target.token_type == TokenType::T_INC;
            emit_dim_inc_dec(context, var_name, &keys, is_inc)?;
            let next = lexer.next_token()?;
            skip_semicolon(lexer, next)
        } else {
            Err("Expected '=' after array index assignment target".to_string())
        }
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

/// One dimension in `$var[k1][k2][] =` — key expression or append when `append` is true.
struct DimAssignKey {
    key: Val,
    append: bool,
}

fn parse_dim_assign_keys(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    open_bracket: Token,
) -> Result<(Vec<DimAssignKey>, Token), String> {
    if !token_is_punct(&open_bracket, "[") {
        return Err("Expected '[' to start array index assignment".to_string());
    }

    let mut keys = Vec::new();
    let mut token = lexer.next_token()?;

    loop {
        if token_is_punct(&token, "]") {
            keys.push(DimAssignKey {
                key: null_val(),
                append: true,
            });
            token = lexer.next_token()?;
            break;
        }

        let (index_val, after_index) =
            parse_additive_expr_with_initial(lexer, context, token)?;
        if !token_is_punct(&after_index, "]") {
            return Err("Expected ']' after array index in assignment".to_string());
        }
        keys.push(DimAssignKey {
            key: index_val,
            append: false,
        });
        token = lexer.next_token()?;
        if token_is_punct(&token, "[") {
            token = lexer.next_token()?;
            continue;
        }
        break;
    }

    Ok((keys, token))
}

fn emit_dim_assignment(
    context: &mut CompileContext,
    var_name: &str,
    keys: &[DimAssignKey],
    value: Val,
) -> Result<(), String> {
    if keys.is_empty() {
        return Err("Array assignment requires at least one index".to_string());
    }

    if keys.len() == 1 {
        let var_name_zval = string_val(var_name);
        let last = &keys[0];
        if last.append {
            context.emit_opcode_ext(Opcode::AssignDim, var_name_zval, value, null_val(), 1);
        } else {
            context.emit_opcode(Opcode::AssignDim, var_name_zval, value, clone_val(&last.key));
        }
        return Ok(());
    }

    let root_slot = context.alloc_temp();
    context.emit_opcode(
        Opcode::FetchVar,
        var_ref(var_name),
        null_val(),
        temp_var_ref(root_slot),
    );

    let mut slots = vec![root_slot];
    for key in &keys[..keys.len() - 1] {
        if key.append {
            return Err("Array append '[]' is only valid as the final dimension".to_string());
        }
        let next_slot = context.alloc_temp();
        context.emit_opcode(
            Opcode::FetchDim,
            temp_var_ref(*slots.last().unwrap()),
            clone_val(&key.key),
            temp_var_ref(next_slot),
        );
        slots.push(next_slot);
    }

    let last = keys.last().unwrap();
    let container = temp_var_ref(*slots.last().unwrap());
    if last.append {
        context.emit_opcode_ext(Opcode::AssignDim, container, value, null_val(), 1);
    } else {
        context.emit_opcode(
            Opcode::AssignDim,
            container,
            value,
            clone_val(&last.key),
        );
    }

    for i in (0..keys.len() - 1).rev() {
        let parent = if i == 0 {
            string_val(var_name)
        } else {
            temp_var_ref(slots[i])
        };
        context.emit_opcode(
            Opcode::AssignDim,
            parent,
            temp_var_ref(slots[i + 1]),
            clone_val(&keys[i].key),
        );
    }

    Ok(())
}

/// Compile `$obj->prop[k] = v` and chained variants (`$this->data['a']['b'] = v`).
fn emit_obj_prop_dim_assignment(
    context: &mut CompileContext,
    obj_var_name: &str,
    prop_name: &str,
    keys: &[DimAssignKey],
    value: Val,
) -> Result<(), String> {
    if keys.is_empty() {
        return Err("Object property array assignment requires at least one index".to_string());
    }

    let obj_var = var_ref(obj_var_name);
    let prop_zval = string_val(prop_name);

    let prop_slot = context.alloc_temp();
    context.emit_opcode(
        Opcode::FetchObjProp,
        clone_val(&obj_var),
        clone_val(&prop_zval),
        temp_var_ref(prop_slot),
    );

    if keys.len() == 1 {
        let last = &keys[0];
        if last.append {
            context.emit_opcode_ext(
                Opcode::AssignDim,
                temp_var_ref(prop_slot),
                value,
                null_val(),
                1,
            );
        } else {
            context.emit_opcode(
                Opcode::AssignDim,
                temp_var_ref(prop_slot),
                value,
                clone_val(&last.key),
            );
        }
        context.emit_opcode(
            Opcode::AssignObjProp,
            obj_var,
            prop_zval,
            temp_var_ref(prop_slot),
        );
        return Ok(());
    }

    let mut slots = vec![prop_slot];
    for key in &keys[..keys.len() - 1] {
        if key.append {
            return Err("Array append '[]' is only valid as the final dimension".to_string());
        }
        let next_slot = context.alloc_temp();
        context.emit_opcode(
            Opcode::FetchDim,
            temp_var_ref(*slots.last().unwrap()),
            clone_val(&key.key),
            temp_var_ref(next_slot),
        );
        slots.push(next_slot);
    }

    let last = keys.last().unwrap();
    let container = temp_var_ref(*slots.last().unwrap());
    if last.append {
        context.emit_opcode_ext(Opcode::AssignDim, container, value, null_val(), 1);
    } else {
        context.emit_opcode(
            Opcode::AssignDim,
            container,
            value,
            clone_val(&last.key),
        );
    }

    for i in (0..keys.len() - 1).rev() {
        context.emit_opcode(
            Opcode::AssignDim,
            temp_var_ref(slots[i]),
            temp_var_ref(slots[i + 1]),
            clone_val(&keys[i].key),
        );
    }

    context.emit_opcode(
        Opcode::AssignObjProp,
        obj_var,
        prop_zval,
        temp_var_ref(prop_slot),
    );

    Ok(())
}

fn emit_dim_inc_dec(
    context: &mut CompileContext,
    var_name: &str,
    keys: &[DimAssignKey],
    increment: bool,
) -> Result<(), String> {
    if keys.is_empty() || keys.iter().any(|k| k.append) {
        return Err("Increment/decrement requires fixed array indices".to_string());
    }

    let current_slot = emit_dim_fetch_chain(context, var_name, keys)?;
    let one = crate::engine::facade::long_val(1);
    let updated = super::expression::helpers::emit_binary_op(
        context,
        if increment { Opcode::Add } else { Opcode::Sub },
        temp_var_ref(current_slot),
        one,
    );
    emit_dim_assignment(context, var_name, keys, updated)
}

/// Load `$var[k1][k2]...` into a temp slot (auto-vivifies missing intermediate arrays on write-back only).
fn emit_dim_fetch_chain(
    context: &mut CompileContext,
    var_name: &str,
    keys: &[DimAssignKey],
) -> Result<u32, String> {
    if keys.is_empty() {
        return Err("Array fetch requires at least one index".to_string());
    }

    if keys.len() == 1 {
        let root_slot = context.alloc_temp();
        context.emit_opcode(
            Opcode::FetchVar,
            var_ref(var_name),
            null_val(),
            temp_var_ref(root_slot),
        );
        let slot = context.alloc_temp();
        context.emit_opcode(
            Opcode::FetchDim,
            temp_var_ref(root_slot),
            clone_val(&keys[0].key),
            temp_var_ref(slot),
        );
        return Ok(slot);
    }

    let root_slot = context.alloc_temp();
    context.emit_opcode(
        Opcode::FetchVar,
        var_ref(var_name),
        null_val(),
        temp_var_ref(root_slot),
    );

    let mut slots = vec![root_slot];
    for key in &keys[..keys.len() - 1] {
        let next_slot = context.alloc_temp();
        context.emit_opcode(
            Opcode::FetchDim,
            temp_var_ref(*slots.last().unwrap()),
            clone_val(&key.key),
            temp_var_ref(next_slot),
        );
        slots.push(next_slot);
    }

    let leaf_slot = context.alloc_temp();
    let last = keys.last().unwrap();
    context.emit_opcode(
        Opcode::FetchDim,
        temp_var_ref(*slots.last().unwrap()),
        clone_val(&last.key),
        temp_var_ref(leaf_slot),
    );
    Ok(leaf_slot)
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
            arg_token = crate::engine::compile::expression::helpers::parse_call_arg_unknown(lexer, context, arg_token)?;
            if token_is_punct(&arg_token, ",") {
                arg_token = lexer.next_token()?;
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
    } else if token_is_punct(&peek, "[") {
        // Property dimension assignment: $var->prop['k'] = expr; $var->prop['a']['b'] = expr;
        let (keys, after_target) = parse_dim_assign_keys(lexer, context, peek)?;
        if after_target.token_type == TokenType::T_EQUAL {
            let (value, after_value) = parse_expression(lexer, context)?;
            emit_obj_prop_dim_assignment(context, var_name, member_name, &keys, value)?;
            skip_semicolon(lexer, after_value)
        } else {
            Err("Expected '=' after object property index assignment target".to_string())
        }
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
    } else if next.token_type == TokenType::T_PAAMAYIM_NEKUDOTAYIM {
        // Static access statement: ClassName::method() or ClassName::$prop = expr
        let class_val = string_val(val);
        let member_token = lexer.next_token()?;
        if member_token.token_type == TokenType::T_VARIABLE {
            let prop_name = member_token.value.as_ref().unwrap().as_str();
            let prop_zval = string_val(prop_name);
            let after = lexer.next_token()?;
            if after.token_type == TokenType::T_EQUAL {
                // Static property assignment: ClassName::$prop = expr
                let (value, after_value) = parse_expression(lexer, context)?;
                context.emit_opcode(Opcode::AssignStaticProp, class_val, prop_zval, value);
                skip_semicolon(lexer, after_value)
            } else {
                // Static property read as statement (result ignored)
                let slot = context.alloc_temp();
                context.emit_opcode(Opcode::FetchStaticProp, class_val, prop_zval, temp_var_ref(slot));
                skip_semicolon(lexer, after)
            }
        } else {
            let member_name = member_token.value.as_ref()
                .ok_or("Expected member name after '::'")?
                .as_str();
            let member_zval = string_val(member_name);
            let peek = lexer.next_token()?;
            if token_is_punct(&peek, "(") {
                // Static method call
                context.emit_opcode(
                    Opcode::InitFCall,
                    null_val(),
                    null_val(),
                    null_val(),
                );
                let mut arg_token = lexer.next_token()?;
                while !token_is_punct(&arg_token, ")") {
                    arg_token = super::expression::helpers::parse_call_arg_unknown(lexer, context, arg_token)?;
                    if token_is_punct(&arg_token, ",") {
                        arg_token = lexer.next_token()?;
                    }
                }
                let _call_slot = context.alloc_temp();
                context.emit_opcode(
                    Opcode::DoStaticCall,
                    member_zval,
                    class_val,
                    null_val(),
                );
                let after_close = lexer.next_token()?;
                skip_semicolon(lexer, after_close)
            } else {
                // Static constant access (result ignored)
                let slot = context.alloc_temp();
                context.emit_opcode(Opcode::FetchStaticProp, class_val, member_zval, temp_var_ref(slot));
                skip_semicolon(lexer, peek)
            }
        }
    } else {
        Ok(next)
    }
}

/// Compile global statement: global $a, $b;
fn compile_global(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    let mut token = lexer.next_token()?;
    loop {
        if token.token_type != TokenType::T_VARIABLE {
            return Err("Expected variable after global".to_string());
        }
        let var_name = token.value.as_ref().unwrap().as_str();
        let clean = if var_name.starts_with('$') {
            &var_name[1..]
        } else {
            var_name
        };
        context.emit_opcode(
            Opcode::BindGlobal,
            string_val(clean),
            null_val(),
            null_val(),
        );
        token = lexer.next_token()?;
        if token_is_punct(&token, ",") {
            token = lexer.next_token()?;
            continue;
        }
        break;
    }
    skip_semicolon(lexer, token)
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

/// Compile namespace declaration: namespace Foo\Bar;
fn compile_namespace(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    // Read namespace name parts: Foo\Bar\Baz
    let mut parts = Vec::new();
    let mut tok = lexer.next_token()?;
    loop {
        if tok.token_type == TokenType::T_STRING {
            parts.push(tok.value.as_ref().unwrap().as_str().to_string());
        } else {
            break;
        }
        tok = lexer.next_token()?;
        if tok.token_type == TokenType::T_NS_SEPARATOR {
            tok = lexer.next_token()?;
        } else {
            break;
        }
    }
    let ns = parts.join("\\");
    context.current_namespace = Some(ns);
    // tok should be ';' or '{'
    if token_is_punct(&tok, "{") {
        // Bracketed namespace — parse until '}'
        let mut brace_count = 1;
        let mut current = lexer.next_token()?;
        loop {
            if current.token_type == TokenType::T_EOF {
                return Err("Unexpected EOF in namespace block".to_string());
            }
            if token_is_punct(&current, "}") {
                brace_count -= 1;
                if brace_count == 0 {
                    return Ok(lexer.next_token()?);
                }
            } else if token_is_punct(&current, "{") {
                brace_count += 1;
            }
            current = parse_statement(lexer, context, current)?;
        }
    }
    skip_semicolon(lexer, tok)
}

/// Compile use statement: use Foo\Bar\Baz; or use Foo\Bar\Baz as Alias;
fn compile_use(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    // Read fully qualified name parts
    let mut parts = Vec::new();
    let mut tok = lexer.next_token()?;
    loop {
        if tok.token_type == TokenType::T_STRING {
            parts.push(tok.value.as_ref().unwrap().as_str().to_string());
        } else {
            break;
        }
        tok = lexer.next_token()?;
        if tok.token_type == TokenType::T_NS_SEPARATOR {
            tok = lexer.next_token()?;
        } else {
            break;
        }
    }
    let fqn = parts.join("\\");
    // Check for `as Alias`
    let short_name = if tok.token_type == TokenType::T_AS {
        let alias_tok = lexer.next_token()?;
        let alias = alias_tok.value.as_ref().unwrap().as_str().to_string();
        tok = lexer.next_token()?;
        alias
    } else {
        // Default short name is the last segment
        parts.last().cloned().unwrap_or_default()
    };
    context.use_imports.insert(short_name, fqn);
    skip_semicolon(lexer, tok)
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
    let (return_value, after) =
        crate::engine::compile::expression::operators::parse_ternary_expr_with_initial(
            lexer, context, peek,
        )?;
    context.emit_opcode(Opcode::Return, return_value, null_val(), null_val());
    skip_semicolon(lexer, after)
}

/// Compile yield statement (currently treated as return-style value)
fn compile_yield(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    let yield_array = context.ensure_yield_array();
    let peek = lexer.next_token()?;
    if token_is_punct(&peek, ";") {
        context.emit_opcode(Opcode::AddArrayElement, yield_array, null_val(), null_val());
        return lexer.next_token().map(Ok)?;
    }
    let (yield_value, after) = crate::engine::compile::expression::parse_additive_expr_with_initial(lexer, context, peek)?;
    context.emit_opcode(Opcode::AddArrayElement, yield_array, yield_value, null_val());
    skip_semicolon(lexer, after)
}
