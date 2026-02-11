//! OOP statement compilation (class definitions)

use crate::engine::compile::context::CompileContext;
use crate::engine::compile::expression::parse_expression;
use crate::engine::compile::expression::helpers::token_is_punct;
use crate::engine::facade::null_val;
use crate::engine::lexer::{Token, Lexer, TokenType};
use crate::engine::types::{ClassEntry, ClassMethod, Visibility, Val};

use super::parse_statement;

/// Compile a class definition: class Foo { public $x = 0; public function bar() { ... } }
pub(crate) fn compile_class(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    // Expect class name
    let name_token = lexer.next_token()?;
    let class_name = name_token.value.as_ref()
        .ok_or("Expected class name")?
        .as_str()
        .to_string();

    let mut ce = ClassEntry::new(&class_name);

    // Optional: extends ParentClass
    let mut next = lexer.next_token()?;
    if next.token_type == TokenType::T_EXTENDS {
        let parent_token = lexer.next_token()?;
        ce.parent_name = Some(parent_token.value.as_ref()
            .ok_or("Expected parent class name")?
            .as_str()
            .to_string());
        next = lexer.next_token()?;
    }

    // Expect opening brace
    if !token_is_punct(&next, "{") {
        return Err("Expected '{' after class declaration".to_string());
    }

    // Parse class body
    next = lexer.next_token()?;
    loop {
        if token_is_punct(&next, "}") { break; }
        if next.token_type == TokenType::T_EOF {
            return Err("Unexpected EOF in class body".to_string());
        }

        // Parse visibility modifier
        let visibility = match next.token_type {
            TokenType::T_PUBLIC => { next = lexer.next_token()?; Visibility::Public }
            TokenType::T_PROTECTED => { next = lexer.next_token()?; Visibility::Protected }
            TokenType::T_PRIVATE => { next = lexer.next_token()?; Visibility::Private }
            _ => Visibility::Public,
        };

        // Check for static
        let is_static = if next.token_type == TokenType::T_STATIC {
            next = lexer.next_token()?;
            true
        } else {
            false
        };

        if next.token_type == TokenType::T_FUNCTION {
            next = compile_class_method(lexer, &mut ce, visibility, is_static)?;
        } else if next.token_type == TokenType::T_VARIABLE {
            next = compile_class_property(lexer, context, &mut ce, &next)?;
        } else {
            next = lexer.next_token()?;
        }
    }

    context.register_class(ce);
    Ok(lexer.next_token()?)
}

/// Compile a class method definition
fn compile_class_method(
    lexer: &mut Lexer,
    ce: &mut ClassEntry,
    visibility: Visibility,
    is_static: bool,
) -> Result<Token, String> {
    let method_name_token = lexer.next_token()?;
    let method_name = method_name_token.value.as_ref()
        .ok_or("Expected method name")?
        .as_str()
        .to_string();

    // Parse parameter list: (param1, param2, ...)
    let open_paren = lexer.next_token()?;
    if !token_is_punct(&open_paren, "(") {
        return Err("Expected '(' after method name".to_string());
    }

    let mut params = Vec::new();
    let mut pt = lexer.next_token()?;
    while !token_is_punct(&pt, ")") {
        if pt.token_type == TokenType::T_VARIABLE {
            let pname = pt.value.as_ref().unwrap().as_str();
            let pname = if pname.starts_with('$') { &pname[1..] } else { pname };
            params.push(pname.to_string());
        }
        pt = lexer.next_token()?;
        if token_is_punct(&pt, ",") {
            pt = lexer.next_token()?;
        }
    }

    // Parse method body: { ... }
    let open_brace = lexer.next_token()?;
    if !token_is_punct(&open_brace, "{") {
        return Err("Expected '{' after method parameters".to_string());
    }

    // Compile method body into a separate op array
    let mut method_context = CompileContext::new();
    let mut body_token = lexer.next_token()?;
    let mut brace_depth = 1;
    while brace_depth > 0 {
        if token_is_punct(&body_token, "{") { brace_depth += 1; }
        if token_is_punct(&body_token, "}") {
            brace_depth -= 1;
            if brace_depth == 0 { break; }
        }
        if body_token.token_type == TokenType::T_EOF {
            return Err("Unexpected EOF in method body".to_string());
        }
        body_token = parse_statement(lexer, &mut method_context, body_token)?;
    }

    let method = ClassMethod {
        name: method_name.clone(),
        visibility,
        is_static,
        params,
        op_array: method_context.take_op_array(),
    };
    ce.methods.insert(method_name, method);
    Ok(lexer.next_token()?)
}

/// Compile a class property definition
fn compile_class_property(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    ce: &mut ClassEntry,
    token: &Token,
) -> Result<Token, String> {
    let prop_name = token.value.as_ref().unwrap().as_str();
    let prop_name = if prop_name.starts_with('$') { &prop_name[1..] } else { prop_name };
    let prop_name = prop_name.to_string();

    let peek = lexer.next_token()?;
    let mut next = if peek.token_type == TokenType::T_EQUAL {
        let (default_val, after) = parse_expression(lexer, context)?;
        ce.default_properties.insert(prop_name, default_val);
        after
    } else {
        ce.default_properties.insert(prop_name, null_val());
        peek
    };

    // Skip semicolon
    if token_is_punct(&next, ";") {
        next = lexer.next_token()?;
    }
    Ok(next)
}
