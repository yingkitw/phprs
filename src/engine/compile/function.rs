//! Function Compilation
//!
//! Handles compilation of PHP function definitions and closures

use super::context::CompileContext;
use super::statement::parse_statement_block;
use crate::engine::lexer::{Token, Lexer, TokenType};
use crate::engine::types::Val;

use std::sync::atomic::{AtomicU64, Ordering};

/// Global counter for generating unique closure names
static CLOSURE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn is_punct(token: &Token, ch: &str) -> bool {
    token.token_type == TokenType::T_STRING && token.value.as_ref().map(|s| s.as_str()) == Some(ch)
}

/// Check if a token is a type hint keyword
fn is_type_hint(token: &Token) -> bool {
    if token.token_type == TokenType::T_STRING {
        if let Some(ref val) = token.value {
            return matches!(val.as_str(), "int" | "string" | "float" | "bool" | "array" | "object" | "mixed" | "void" | "never" | "self" | "static" | "iterable");
        }
    }
    token.token_type == TokenType::T_ARRAY || token.token_type == TokenType::T_CALLABLE
}

/// Parse parameter list (the opening '(' has already been consumed)
/// Returns the list of parameter names. Supports type declarations.
fn parse_params(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Vec<String>, String> {
    let mut params = Vec::new();
    let mut current_token = lexer.next_token()?;

    if is_punct(&current_token, ")") {
        return Ok(params);
    }

    loop {
        // Skip nullable type hint: ?type
        if is_punct(&current_token, "?") {
            current_token = lexer.next_token()?;
        }

        // Skip type hint: int, string, float, bool, array, etc.
        if is_type_hint(&current_token) {
            current_token = lexer.next_token()?;
        }

        if current_token.token_type != TokenType::T_VARIABLE {
            return Err(format!("Expected variable parameter in function definition, got {:?}", current_token.token_type));
        }

        let param_name = current_token.value.as_ref().unwrap().as_str();
        params.push(param_name.to_string());

        current_token = lexer.next_token()?;

        // Check for default value assignment
        if current_token.token_type == TokenType::T_EQUAL {
            let (_, next_token) = super::expression::parse_expression(lexer, context)?;
            current_token = next_token;
        }

        if is_punct(&current_token, ")") {
            break;
        }

        if !is_punct(&current_token, ",") {
            return Err("Expected ',' or ')' after parameter".to_string());
        }

        current_token = lexer.next_token()?;
    }

    Ok(params)
}

/// Skip return type declaration after ')' if present: ): type
/// Returns the next meaningful token (either '{' or whatever follows)
fn skip_return_type(lexer: &mut Lexer) -> Result<Token, String> {
    let token = lexer.next_token()?;
    if is_punct(&token, ":") {
        // Skip the return type
        let type_token = lexer.next_token()?;
        // Handle nullable return type: ?type
        if is_punct(&type_token, "?") {
            let _actual_type = lexer.next_token()?;
        }
        // The type token itself is consumed, get next
        lexer.next_token()
    } else {
        Ok(token)
    }
}

/// Parse `use ($var1, $var2, ...)` clause for closures
/// Returns list of captured variable names
fn parse_use_clause(lexer: &mut Lexer) -> Result<Vec<String>, String> {
    let mut captures = Vec::new();
    let paren = lexer.next_token()?;
    if !is_punct(&paren, "(") {
        return Err("Expected '(' after 'use'".to_string());
    }

    let mut current_token = lexer.next_token()?;
    if is_punct(&current_token, ")") {
        return Ok(captures);
    }

    loop {
        if current_token.token_type != TokenType::T_VARIABLE {
            return Err("Expected variable in use clause".to_string());
        }
        captures.push(current_token.value.as_ref().unwrap().as_str().to_string());

        current_token = lexer.next_token()?;
        if is_punct(&current_token, ")") {
            break;
        }
        if !is_punct(&current_token, ",") {
            return Err("Expected ',' or ')' in use clause".to_string());
        }
        current_token = lexer.next_token()?;
    }

    Ok(captures)
}

/// Compile function body into a separate context and return the finalized OpArray
/// Handles optional return type declaration before the opening brace
fn compile_function_body(
    lexer: &mut Lexer,
    context: &CompileContext,
    function_name: &str,
    lineno: u32,
) -> Result<crate::engine::vm::OpArray, String> {
    let mut func_context = CompileContext::new();
    func_context.set_filename(context.filename.as_deref().unwrap_or(""));
    func_context.set_line(lineno);
    func_context.op_array.function_name = Some(function_name.to_string());

    // Skip optional return type declaration (: type) and find '{'
    let brace_token = skip_return_type(lexer)?;
    if !is_punct(&brace_token, "{") {
        return Err("Expected '{' after function parameters".to_string());
    }

    parse_statement_block(lexer, &mut func_context)?;
    Ok(func_context.finalize())
}

/// Compile function definition: function name($param1, $param2) { body }
pub fn compile_function(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    let name_token = lexer.next_token()?;

    // If the next token is '(' — this is a closure used as a statement (rare but valid)
    if is_punct(&name_token, "(") {
        let (_val, next_token) = compile_closure_inner(lexer, context, name_token.lineno)?;
        return Ok(next_token);
    }

    if name_token.token_type != TokenType::T_STRING {
        return Err("Expected function name after 'function'".to_string());
    }

    let function_name = name_token.value.as_ref().unwrap().as_str();

    let paren_token = lexer.next_token()?;
    if !is_punct(&paren_token, "(") {
        return Err("Expected '(' after function name".to_string());
    }

    let params = parse_params(lexer, context)?;
    let mut func_op_array = compile_function_body(lexer, context, function_name, name_token.lineno)?;
    // Store param names so the VM can bind arguments
    func_op_array.vars = params.iter().map(|p| crate::engine::vm::var_ref(p)).collect();

    context
        .function_table
        .store_function(function_name, func_op_array);

    Ok(lexer.next_token()?)
}

/// Compile a closure expression: function($params) use ($captures) { body }
/// Called from expression parser when `function` keyword is encountered in expression position.
/// Returns (Val referencing the closure name, next Token)
pub fn compile_closure(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<(Val, Token), String> {
    // The 'function' keyword has been consumed. Next should be '('
    let paren_token = lexer.next_token()?;
    if !is_punct(&paren_token, "(") {
        return Err("Expected '(' after 'function' in closure".to_string());
    }
    compile_closure_inner(lexer, context, paren_token.lineno)
}

/// Inner closure compilation — '(' has been consumed
fn compile_closure_inner(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    lineno: u32,
) -> Result<(Val, Token), String> {
    let params = parse_params(lexer, context)?;

    // After params, we may see: `: returnType`, `use (...)`, or `{`
    let mut next = lexer.next_token()?;

    // Skip optional return type declaration
    if is_punct(&next, ":") {
        let type_token = lexer.next_token()?;
        if is_punct(&type_token, "?") {
            let _actual_type = lexer.next_token()?;
        }
        next = lexer.next_token()?;
    }

    // Check for `use` clause
    let captures = if next.token_type == TokenType::T_USE {
        let caps = parse_use_clause(lexer)?;
        next = lexer.next_token()?;
        caps
    } else {
        Vec::new()
    };

    if !is_punct(&next, "{") {
        return Err(format!("Expected '{{' in closure, got {:?}", next.token_type));
    }

    let closure_name = format!("__closure_{}", CLOSURE_COUNTER.fetch_add(1, Ordering::Relaxed));
    let mut func_context = CompileContext::new();
    func_context.set_filename(context.filename.as_deref().unwrap_or(""));
    func_context.set_line(lineno);
    func_context.op_array.function_name = Some(closure_name.clone());
    func_context.op_array.vars = params.iter().chain(captures.iter()).map(|p| {
        crate::engine::vm::var_ref(p)
    }).collect();

    parse_statement_block(lexer, &mut func_context)?;
    let func_op_array = func_context.finalize();
    context.function_table.store_function(&closure_name, func_op_array);

    let val = crate::engine::facade::string_val(&closure_name);
    let next_token = lexer.next_token()?;
    Ok((val, next_token))
}
