//! Shared helpers for expression parsing — eliminates duplication (DRY)

use crate::engine::compile::context::CompileContext;
use crate::engine::facade::{self, StdValFactory, ValFactory};
use crate::engine::lexer::{Lexer, Token, TokenType};
use crate::engine::string::string_init;
use crate::engine::types::{PhpType, PhpValue, Val};
use crate::engine::vm::{Opcode, temp_var_ref, var_ref};

/// Check if token matches a specific punctuation string
pub(crate) fn token_is_punct(token: &Token, ch: &str) -> bool {
    token.token_type == TokenType::T_STRING && token.value.as_ref().map(|s| s.as_str()) == Some(ch)
}

pub(crate) fn token_is_keyword(token: &Token, keyword: &str) -> bool {
    token.token_type == TokenType::T_STRING
        && token.value.as_ref().map(|s| s.as_str()) == Some(keyword)
}

/// Convert a pre-consumed token into a primary Val
pub(crate) fn token_to_primary(tok: &Token, context: &mut CompileContext) -> Result<Val, String> {
    match tok.token_type {
        TokenType::T_LNUMBER => {
            let num_val = tok
                .value
                .as_ref()
                .unwrap()
                .as_str()
                .parse::<i64>()
                .unwrap_or(0);
            Ok(facade::long_val(num_val))
        }
        TokenType::T_DNUMBER => {
            let num_val = tok
                .value
                .as_ref()
                .ok_or("Number token missing value")?
                .as_str()
                .parse::<f64>()
                .unwrap_or(0.0);
            Ok(facade::double_val(num_val))
        }
        TokenType::T_CONSTANT_ENCAPSED_STRING => {
            let str_val = tok
                .value
                .as_ref()
                .ok_or("String token missing value")?
                .clone();
            Ok(Val::new(
                crate::engine::types::PhpValue::String(Box::new(str_val)),
                crate::engine::types::PhpType::String,
            ))
        }
        TokenType::T_VARIABLE => {
            let name = tok
                .value
                .as_ref()
                .ok_or("Variable token missing value")?
                .as_str();
            Ok(var_ref(name))
        }
        TokenType::T_STRING => {
            let val = tok.value.as_ref().map(|s| s.as_str()).unwrap_or("");
            match val {
                "true" => Ok(StdValFactory::bool_val(true)),
                "false" => Ok(StdValFactory::bool_val(false)),
                "null" => Ok(facade::null_val()),
                _ => Ok(compile_constant_lookup(context, val)),
            }
        }
        TokenType::T_STATIC => Ok(crate::engine::facade::string_val("static")),
        _ => Err(format!(
            "Unexpected token in expression: {:?}",
            tok.token_type
        )),
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

/// Emit a unary/binary logical opcode and return the result temp ref
pub(crate) fn emit_logical_op(
    context: &mut CompileContext,
    opcode: Opcode,
    left: Val,
    right: Val,
) -> Val {
    emit_binary_op(context, opcode, left, right)
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
        let is_default =
            token.token_type == TokenType::T_DEFAULT || token_is_keyword(&token, "default");
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
            let cmp = emit_binary_op(
                context,
                Opcode::IsEqual,
                facade::clone_val(&match_value),
                condition,
            );
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
fn local_var_name_from_var_ref(val: &Val) -> Option<String> {
    if val.get_type() != PhpType::Undef {
        return None;
    }
    if let PhpValue::String(s) = &val.value {
        let n = s.as_str();
        let clean = if n.starts_with('$') { &n[1..] } else { n };
        Some(clean.to_string())
    } else {
        None
    }
}

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
        } else if next.token_type == TokenType::T_PAAMAYIM_NEKUDOTAYIM {
            let (new_result, new_next) = parse_static_access(lexer, context, result)?;
            result = new_result;
            next = new_next;
        } else if token_is_punct(&next, "(") {
            // Callable variable: $var(args...)
            let (call_result, call_next) = parse_callable_var(lexer, context, result)?;
            result = call_result;
            next = call_next;
        } else if next.token_type == TokenType::T_INC {
            let name = local_var_name_from_var_ref(&result)
                .ok_or_else(|| "Post-increment requires a simple variable".to_string())?;
            let old_slot = context.alloc_temp();
            context.emit_opcode(
                Opcode::FetchVar,
                facade::clone_val(&result),
                facade::null_val(),
                temp_var_ref(old_slot),
            );
            let one = facade::long_val(1);
            let new_val = emit_binary_op(context, Opcode::Add, facade::clone_val(&result), one);
            let var_name_zval = facade::string_val(&name);
            let new_val_op2 = StdValFactory::clone_val(&new_val);
            context.emit_opcode(Opcode::Assign, var_name_zval, new_val, new_val_op2);
            result = temp_var_ref(old_slot);
            next = lexer.next_token()?;
        } else if next.token_type == TokenType::T_DEC {
            let name = local_var_name_from_var_ref(&result)
                .ok_or_else(|| "Post-decrement requires a simple variable".to_string())?;
            let old_slot = context.alloc_temp();
            context.emit_opcode(
                Opcode::FetchVar,
                facade::clone_val(&result),
                facade::null_val(),
                temp_var_ref(old_slot),
            );
            let one = facade::long_val(1);
            let new_val = emit_binary_op(context, Opcode::Sub, facade::clone_val(&result), one);
            let var_name_zval = facade::string_val(&name);
            let new_val_op2 = StdValFactory::clone_val(&new_val);
            context.emit_opcode(Opcode::Assign, var_name_zval, new_val, new_val_op2);
            result = temp_var_ref(old_slot);
            next = lexer.next_token()?;
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
    let member_name = member_token
        .value
        .as_ref()
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

/// Parse static access after '::': ClassName::$prop, ClassName::method(), static::$prop, etc.
/// The `result` Val contains the class name (as a string Val or var_ref)
fn parse_static_access(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    class_val: Val,
) -> Result<(Val, Token), String> {
    let member_token = lexer.next_token()?;

    // Handle static property access: ClassName::$prop or static::$prop
    if member_token.token_type == TokenType::T_VARIABLE {
        let prop_name = member_token
            .value
            .as_ref()
            .ok_or("Expected property name after '::'")?
            .as_str();
        let prop_zval = facade::string_val(prop_name);
        let slot = context.alloc_temp();
        context.emit_opcode(
            Opcode::FetchStaticProp,
            class_val,
            prop_zval,
            temp_var_ref(slot),
        );
        return Ok((temp_var_ref(slot), lexer.next_token()?));
    }

    // Handle static method call or constant: ClassName::method() or ClassName::CONST
    let member_name = member_token
        .value
        .as_ref()
        .ok_or("Expected member name after '::'")?
        .as_str();
    let member_zval = facade::string_val(member_name);

    let peek = lexer.next_token()?;
    if token_is_punct(&peek, "(") {
        let arg_token = lexer.next_token()?;
        if arg_token.token_type == TokenType::T_ELLIPSIS {
            let close = lexer.next_token()?;
            if !token_is_punct(&close, ")") {
                return Err("Expected ')' after '...' in first-class callable".to_string());
            }
            let callable = emit_first_class_static_callable(context, class_val, member_name);
            return Ok((callable, lexer.next_token()?));
        }

        // Static method call: ClassName::method(args...)
        context.emit_opcode(
            Opcode::InitFCall,
            facade::null_val(),
            facade::null_val(),
            facade::null_val(),
        );

        let mut arg_token = arg_token;
        while !token_is_punct(&arg_token, ")") {
            arg_token = parse_call_arg_unknown(lexer, context, arg_token)?;
            if token_is_punct(&arg_token, ",") {
                arg_token = lexer.next_token()?;
            }
        }

        let call_slot = context.alloc_temp();
        context.emit_opcode(
            Opcode::DoStaticCall,
            member_zval,
            class_val,
            temp_var_ref(call_slot),
        );
        Ok((temp_var_ref(call_slot), lexer.next_token()?))
    } else {
        // Static constant access: ClassName::CONST — for now, emit FetchStaticProp
        let slot = context.alloc_temp();
        context.emit_opcode(
            Opcode::FetchStaticProp,
            class_val,
            member_zval,
            temp_var_ref(slot),
        );
        Ok((temp_var_ref(slot), peek))
    }
}

/// Parse method call arguments and emit InitMethodCall + SendVal + DoMethodCall
pub(crate) fn parse_method_call(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    obj: Val,
    member_zval: Val,
) -> Result<(Val, Token), String> {
    let method_name = crate::engine::operators::zval_get_string(&member_zval)
        .as_str()
        .to_string();

    let arg_token = lexer.next_token()?;
    if arg_token.token_type == TokenType::T_ELLIPSIS {
        let close = lexer.next_token()?;
        if !token_is_punct(&close, ")") {
            return Err("Expected ')' after '...' in first-class callable".to_string());
        }
        let callable = emit_first_class_method_callable(context, obj, &method_name);
        return Ok((callable, lexer.next_token()?));
    }

    context.emit_opcode(
        Opcode::InitMethodCall,
        crate::engine::facade::clone_val(&obj),
        crate::engine::facade::clone_val(&member_zval),
        facade::null_val(),
    );

    // Parse arguments
    let mut arg_token = arg_token;
    while !token_is_punct(&arg_token, ")") {
        arg_token = parse_call_arg_unknown(lexer, context, arg_token)?;
        if token_is_punct(&arg_token, ",") {
            arg_token = lexer.next_token()?;
        }
    }

    let call_slot = context.alloc_temp();
    context.emit_opcode(
        Opcode::DoMethodCall,
        member_zval,
        obj,
        temp_var_ref(call_slot),
    );
    Ok((temp_var_ref(call_slot), lexer.next_token()?))
}

/// Parse callable variable: $var(args...) — the opening '(' has already been consumed
fn parse_callable_var(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    callable: Val,
) -> Result<(Val, Token), String> {
    let mut current_token = lexer.next_token()?;

    // Emit InitFCall
    context.emit_opcode(
        Opcode::InitFCall,
        facade::null_val(),
        facade::null_val(),
        facade::null_val(),
    );

    if !token_is_punct(&current_token, ")") {
        current_token = parse_call_arg_unknown(lexer, context, current_token)?;

        while token_is_punct(&current_token, ",") {
            let next = lexer.next_token()?;
            current_token = parse_call_arg_unknown(lexer, context, next)?;
        }

        if !token_is_punct(&current_token, ")") {
            return Err("Expected ',' or ')' after callable argument".to_string());
        }
    }

    // Emit DoFCall with the callable variable as the function name
    let call_slot = context.alloc_temp();
    context.emit_opcode(
        Opcode::DoFCall,
        callable,
        facade::null_val(),
        temp_var_ref(call_slot),
    );
    Ok((temp_var_ref(call_slot), lexer.next_token()?))
}

/// Compile `new ClassName()` or `new class { ... }` — shared between parse_primary_expr and _with_initial
pub(crate) fn compile_new_obj(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    let class_token = lexer.next_token()?;

    // Anonymous class: new class { ... }
    let (resolved_name, peek, ctor_args) = if class_token.token_type == TokenType::T_CLASS {
        use std::sync::atomic::{AtomicU64, Ordering};
        static ANON_CLASS_COUNTER: AtomicU64 = AtomicU64::new(0);
        let anon_name = format!(
            "__anon_class_{}",
            ANON_CLASS_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let mut ce = crate::engine::types::ClassEntry::new(&anon_name);

        let mut next = lexer.next_token()?;
        let mut ctor_args = Vec::new();

        // Check for constructor args before class body: class(args...)
        if token_is_punct(&next, "(") {
            let mut arg_token = lexer.next_token()?;
            while !token_is_punct(&arg_token, ")") {
                let (arg_val, after_arg) =
                    super::operators::parse_additive_expr_with_initial(lexer, context, arg_token)?;
                ctor_args.push(arg_val);
                if token_is_punct(&after_arg, ",") {
                    arg_token = lexer.next_token()?;
                } else {
                    arg_token = after_arg;
                }
            }
            next = lexer.next_token()?; // token after )
        }

        // Optional: extends ParentClass
        if next.token_type == TokenType::T_EXTENDS {
            let parent_token = lexer.next_token()?;
            let parent_name = parent_token
                .value
                .as_ref()
                .ok_or("Expected parent class name after 'extends'")?
                .as_str();
            ce.parent_name = Some(context.resolve_class_name(parent_name));
            next = lexer.next_token()?;
        }

        // Optional: implements Interface1, Interface2
        if next.token_type == TokenType::T_IMPLEMENTS {
            next = lexer.next_token()?;
            while next.token_type == TokenType::T_STRING {
                next = lexer.next_token()?;
                if token_is_punct(&next, ",") {
                    next = lexer.next_token()?;
                }
            }
        }

        if !token_is_punct(&next, "{") {
            return Err("Expected '{' after anonymous class declaration".to_string());
        }

        crate::engine::compile::statement::oop::parse_class_body(lexer, context, &mut ce)?;
        context.register_class(ce);
        (anon_name, lexer.next_token()?, ctor_args)
    } else {
        let class_name = class_token
            .value
            .as_ref()
            .ok_or("Expected class name after 'new'")?
            .as_str();
        (
            context.resolve_class_name(class_name),
            lexer.next_token()?,
            Vec::new(),
        )
    };

    let class_zval = facade::string_val(&resolved_name);
    let slot = context.alloc_temp();
    context.emit_opcode(
        Opcode::NewObj,
        class_zval,
        facade::null_val(),
        temp_var_ref(slot),
    );

    // Check for constructor args: (...)
    let args = if !ctor_args.is_empty() {
        ctor_args
    } else if token_is_punct(&peek, "(") {
        let mut args = Vec::new();
        let mut arg_token = lexer.next_token()?;
        while !token_is_punct(&arg_token, ")") {
            let (arg_val, after_arg) =
                super::operators::parse_additive_expr_with_initial(lexer, context, arg_token)?;
            args.push(arg_val);
            if token_is_punct(&after_arg, ",") {
                arg_token = lexer.next_token()?;
            } else {
                arg_token = after_arg;
            }
        }
        args
    } else {
        Vec::new()
    };

    if !args.is_empty() {
        // Emit InitMethodCall for __construct
        let obj_temp = temp_var_ref(slot);
        context.emit_opcode(
            Opcode::InitMethodCall,
            crate::engine::facade::clone_val(&obj_temp),
            facade::string_val("__construct"),
            facade::null_val(),
        );

        // Emit SendVal for each argument
        for arg in args {
            context.emit_opcode(Opcode::SendVal, arg, facade::null_val(), facade::null_val());
        }

        // Emit DoMethodCall for __construct
        let construct_slot = context.alloc_temp();
        context.emit_opcode(
            Opcode::DoMethodCall,
            facade::string_val("__construct"),
            obj_temp,
            temp_var_ref(construct_slot),
        );

        Ok((temp_var_ref(slot), lexer.next_token()?))
    } else {
        Ok((temp_var_ref(slot), peek))
    }
}

/// Compile `clone $obj` — emits CloneObj opcode
pub(crate) fn compile_clone_obj(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    let (expr_val, next) = super::parse_expression(lexer, context)?;
    let slot = context.alloc_temp();
    context.emit_opcode(
        Opcode::CloneObj,
        expr_val,
        facade::null_val(),
        temp_var_ref(slot),
    );
    Ok((temp_var_ref(slot), next))
}

/// Run the multiplicative operator loop (* / %) on an already-parsed left value
pub(crate) fn multiplicative_loop(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    mut left: Val,
    mut token: Token,
) -> Result<(Val, Token), String> {
    // Exponent (`**`) binds tighter than * / %
    let (new_left, new_token) = pow_loop(lexer, context, left, token)?;
    left = new_left;
    token = new_token;
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
        let (new_left, new_token) = pow_loop(lexer, context, left, token)?;
        left = new_left;
        token = new_token;
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

/// Exponent chain (`**`, right-associative) — binds tighter than `*` / `/` / `%`
pub(crate) fn pow_loop(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    mut left: Val,
    mut token: Token,
) -> Result<(Val, Token), String> {
    while token.token_type == TokenType::T_POW {
        // Exponent parses as a full primary expression (own pow chain → right-assoc)
        let (right, next) = super::primary::parse_primary_expr(lexer, context)?;
        left = emit_binary_op(context, Opcode::Pow, left, right);
        token = next;
    }
    Ok((left, token))
}

/// Compile an interpolated string like "Hello $name, you are $age years old"
pub(crate) fn compile_interpolated_string(
    s: &str,
    context: &mut CompileContext,
) -> Result<Val, String> {
    let bytes = s.as_bytes();
    let mut parts: Vec<Val> = Vec::new();
    let mut i = 0;
    let mut text_start = 0;

    while i < bytes.len() {
        if bytes[i] == b'$'
            && i + 1 < bytes.len()
            && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_')
        {
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

/// Shared body for `[...]` and `array(...)` literals. Opening delimiter already consumed.
fn parse_array_elements(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    close: &str,
    close_list_err: &str,
) -> Result<Val, String> {
    let arr_slot = context.alloc_temp();
    context.emit_opcode(
        Opcode::InitArray,
        facade::null_val(),
        facade::null_val(),
        temp_var_ref(arr_slot),
    );

    let mut next = lexer.next_token()?;
    if !token_is_punct(&next, close) {
        loop {
            let (val_zval, after_val) =
                super::operators::parse_additive_expr_with_initial(lexer, context, next)?;

            if after_val.token_type == TokenType::T_DOUBLE_ARROW {
                let key_zval = val_zval;
                let (value_zval, after_value) = super::parse_expression(lexer, context)?;
                context.emit_opcode(
                    Opcode::AddArrayElement,
                    temp_var_ref(arr_slot),
                    value_zval,
                    key_zval,
                );
                let last_idx = context.current_op_index() - 1;
                context.update_jump_target(last_idx, 1);
                next = after_value;
            } else {
                context.emit_opcode(
                    Opcode::AddArrayElement,
                    temp_var_ref(arr_slot),
                    val_zval,
                    facade::null_val(),
                );
                next = after_val;
            }

            if token_is_punct(&next, close) {
                break;
            }
            if token_is_punct(&next, ",") {
                next = lexer.next_token()?;
                if token_is_punct(&next, close) {
                    break;
                }
                continue;
            }
            return Err(close_list_err.to_string());
        }
    }
    Ok(temp_var_ref(arr_slot))
}

/// Parse array literal: [elem1, elem2, ...] or ['key' => val, ...]
/// The opening '[' has already been consumed
pub(crate) fn parse_array_literal(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Val, String> {
    parse_array_elements(lexer, context, "]", "Expected ',' or ']' in array literal")
}

/// Parse legacy array constructor: array(...) — `array` keyword already consumed
pub(crate) fn parse_long_array_literal(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Val, String> {
    let open = lexer.next_token()?;
    if !token_is_punct(&open, "(") {
        return Err("Expected '(' after array".to_string());
    }
    parse_array_elements(lexer, context, ")", "Expected ',' or ')' in array()")
}

/// Parse a single call argument, detecting named arguments (name: value)
/// Emits SendVal, SendVarRef, or SendValNamed directly
pub(crate) fn parse_call_arg(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    first_token: Token,
    ref_params: Option<&[bool]>,
    positional_idx: &mut usize,
) -> Result<Token, String> {
    // Check for named argument: name: value
    if first_token.token_type == TokenType::T_STRING {
        let name = first_token.value.as_ref().unwrap().as_str();
        let peek = lexer.peek_token()?;
        if token_is_punct(&peek, ":") {
            lexer.next_token()?; // consume ':'
            let (value, after) = super::parse_expression(lexer, context)?;
            context.emit_opcode(
                Opcode::SendValNamed,
                value,
                facade::string_val(name),
                facade::null_val(),
            );
            return Ok(after);
        }
    }

    // Pass-by-reference: bare $var when callee declares &$param
    if first_token.token_type == TokenType::T_VARIABLE {
        let wants_ref = ref_params
            .and_then(|flags| flags.get(*positional_idx))
            .copied()
            .unwrap_or(false);
        if wants_ref {
            let name = first_token.value.as_ref().unwrap().as_str();
            context.emit_opcode(
                Opcode::SendVarRef,
                var_ref(name),
                facade::null_val(),
                facade::null_val(),
            );
            *positional_idx += 1;
            return lexer.next_token();
        }
    }

    // Positional argument (or non-named T_STRING)
    let (arg_val, after) =
        super::operators::parse_additive_expr_with_initial(lexer, context, first_token)?;
    context.emit_opcode(
        Opcode::SendVal,
        arg_val,
        facade::null_val(),
        facade::null_val(),
    );
    *positional_idx += 1;
    Ok(after)
}

/// Parse a call argument when callee ref metadata is unknown (dynamic calls).
pub(crate) fn parse_call_arg_unknown(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    first_token: Token,
) -> Result<Token, String> {
    let mut positional_idx = 0;
    parse_call_arg(lexer, context, first_token, None, &mut positional_idx)
}

fn compile_first_class_function(name: &str) -> Val {
    Val::new(
        PhpValue::String(Box::new(string_init(name, false))),
        PhpType::Callable,
    )
}

fn compile_first_class_static(class_name: &str, method: &str) -> Val {
    let encoded = format!("{class_name}::{method}");
    Val::new(
        PhpValue::String(Box::new(string_init(&encoded, false))),
        PhpType::Callable,
    )
}

fn emit_array_assoc_pair(context: &mut CompileContext, arr: Val, key: &str, value: Val) {
    context.emit_opcode(Opcode::AddArrayElement, arr, value, facade::string_val(key));
    let last_idx = context.current_op_index() - 1;
    context.update_jump_target(last_idx, 1);
}

fn emit_first_class_method_callable(context: &mut CompileContext, obj: Val, method: &str) -> Val {
    let arr_slot = context.alloc_temp();
    context.emit_opcode(
        Opcode::InitArray,
        facade::null_val(),
        facade::null_val(),
        temp_var_ref(arr_slot),
    );
    let arr = temp_var_ref(arr_slot);
    emit_array_assoc_pair(
        context,
        facade::clone_val(&arr),
        "type",
        facade::string_val("method"),
    );
    emit_array_assoc_pair(
        context,
        facade::clone_val(&arr),
        "method",
        facade::string_val(method),
    );
    emit_array_assoc_pair(context, facade::clone_val(&arr), "object", obj);
    arr
}

fn emit_first_class_static_callable(
    context: &mut CompileContext,
    class_val: Val,
    method: &str,
) -> Val {
    if let PhpValue::String(ref s) = class_val.value {
        return compile_first_class_static(s.as_str(), method);
    }
    let arr_slot = context.alloc_temp();
    context.emit_opcode(
        Opcode::InitArray,
        facade::null_val(),
        facade::null_val(),
        temp_var_ref(arr_slot),
    );
    let arr = temp_var_ref(arr_slot);
    emit_array_assoc_pair(
        context,
        facade::clone_val(&arr),
        "type",
        facade::string_val("static"),
    );
    emit_array_assoc_pair(context, facade::clone_val(&arr), "class", class_val);
    emit_array_assoc_pair(
        context,
        facade::clone_val(&arr),
        "method",
        facade::string_val(method),
    );
    arr
}

pub(crate) fn parse_function_call(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    function_name: &str,
) -> Result<(Val, Token), String> {
    let mut current_token = lexer.next_token()?;
    if current_token.token_type == TokenType::T_ELLIPSIS {
        let close = lexer.next_token()?;
        if !token_is_punct(&close, ")") {
            return Err("Expected ')' after '...' in first-class callable".to_string());
        }
        return Ok((
            compile_first_class_function(function_name),
            lexer.next_token()?,
        ));
    }

    // Emit InitFCall
    context.emit_opcode(
        Opcode::InitFCall,
        facade::null_val(),
        facade::null_val(),
        facade::null_val(),
    );

    let ref_params: Option<Vec<bool>> = context
        .function_table
        .lookup_function(function_name)
        .map(|op_array| op_array.ref_params.clone());
    let mut positional_idx = 0;

    if !token_is_punct(&current_token, ")") {
        current_token = parse_call_arg(
            lexer,
            context,
            current_token,
            ref_params.as_deref(),
            &mut positional_idx,
        )?;

        while token_is_punct(&current_token, ",") {
            let next = lexer.next_token()?;
            current_token = parse_call_arg(
                lexer,
                context,
                next,
                ref_params.as_deref(),
                &mut positional_idx,
            )?;
        }

        if !token_is_punct(&current_token, ")") {
            return Err("Expected ',' or ')' after function argument".to_string());
        }
    }

    // Emit DoFCall
    let func_name_zval = facade::string_val(function_name);
    let result_slot = context.alloc_temp();
    context.emit_opcode(
        Opcode::DoFCall,
        func_name_zval,
        facade::null_val(),
        temp_var_ref(result_slot),
    );

    Ok((temp_var_ref(result_slot), lexer.next_token()?))
}

/// Emit constant(name) lookup for bare identifiers (WordPress/PHP compatibility)
/// Returns a string Val with the constant name that will be resolved at runtime
pub(crate) fn compile_constant_lookup(_context: &mut CompileContext, name: &str) -> Val {
    Val::new(
        PhpValue::String(Box::new(string_init(name, false))),
        PhpType::ConstantAst,
    )
}
