//! OOP statement compilation (class definitions)

use crate::engine::compile::context::CompileContext;
use crate::engine::compile::const_eval::eval_class_default_expr;
use crate::engine::compile::expression::parse_expression;
use crate::engine::compile::expression::helpers::token_is_punct;
use crate::engine::facade::null_val;
use crate::engine::lexer::{Token, Lexer, TokenType};
use crate::engine::compile::function::parse_params;
use crate::engine::types::{ClassEntry, ClassMethod, Visibility};

use super::{parse_statement, skip_attribute_block};

/// Parse a class/trait body: visibility, static, readonly, final, methods, properties, use TraitName
pub(crate) fn parse_class_body(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    ce: &mut ClassEntry,
) -> Result<(), String> {
    let mut next = lexer.next_token()?;
    loop {
        if token_is_punct(&next, "}") { break; }
        if next.token_type == TokenType::T_EOF {
            return Err("Unexpected EOF in class/trait body".to_string());
        }

        // Skip attributes (#[...]) before class members
        if next.token_type == TokenType::T_ATTRIBUTE {
            next = skip_attribute_block(lexer)?;
            continue;
        }

        // Handle `use TraitName;` inside class body
        if next.token_type == TokenType::T_USE {
            next = compile_use_trait(lexer, context, ce)?;
            continue;
        }

        // Parse visibility modifier
        let visibility = match next.token_type {
            TokenType::T_PUBLIC => { next = lexer.next_token()?; Visibility::Public }
            TokenType::T_PROTECTED => { next = lexer.next_token()?; Visibility::Protected }
            TokenType::T_PRIVATE => { next = lexer.next_token()?; Visibility::Private }
            _ => Visibility::Public,
        };

        // Check for readonly (PHP 8.1)
        let is_readonly = if next.token_type == TokenType::T_READONLY {
            next = lexer.next_token()?;
            true
        } else {
            false
        };

        // Check for static
        let is_static = if next.token_type == TokenType::T_STATIC {
            next = lexer.next_token()?;
            true
        } else {
            false
        };

        // Check for const
        if next.token_type == TokenType::T_CONST {
            next = compile_class_const(lexer, context, ce, visibility)?;
            continue;
        }

        if next.token_type == TokenType::T_FUNCTION {
            next = compile_class_method(lexer, context, ce, visibility, is_static)?;
        } else if next.token_type == TokenType::T_VARIABLE {
            next = compile_class_property(lexer, context, ce, &next, is_static, is_readonly, visibility)?;
        } else {
            next = lexer.next_token()?;
        }
    }
    Ok(())
}

/// Compile `use TraitName;` inside a class body — copies trait methods into the class
fn compile_use_trait(
    lexer: &mut Lexer,
    context: &CompileContext,
    ce: &mut ClassEntry,
) -> Result<Token, String> {
    let trait_name_token = lexer.next_token()?;
    let trait_name = trait_name_token.value.as_ref()
        .ok_or("Expected trait name after 'use'")?
        .as_str()
        .to_string();

    let resolved_trait = context.resolve_class_name(&trait_name);

    // Look up the trait in the class table (stored with __trait_ prefix)
    let trait_key = format!("__trait_{}", resolved_trait);
    if let Some(trait_ce) = context.class_table.get(&trait_key) {
        // Copy trait methods into the class
        for (method_name, method) in &trait_ce.methods {
            if !ce.methods.contains_key(method_name) {
                // Clone the method's op array
                let mut new_ops = Vec::new();
                for op in &method.op_array.ops {
                    new_ops.push(crate::engine::vm::Op::new(
                        op.opcode,
                        crate::engine::vm::execute_data::clone_val(&op.op1),
                        crate::engine::vm::execute_data::clone_val(&op.op2),
                        crate::engine::vm::execute_data::clone_val(&op.result),
                        op.extended_value,
                    ));
                }
                let method_file = method
                    .op_array
                    .filename
                    .clone()
                    .filter(|f| !f.is_empty())
                    .unwrap_or_else(|| format!("{}::{}", ce.name, method_name));
                let mut new_op_array =
                    crate::engine::vm::OpArray::with_capacity(new_ops.len(), method_file);
                new_op_array.ops = new_ops;
                ce.methods.insert(method_name.clone(), ClassMethod {
                    name: method.name.clone(),
                    visibility: method.visibility,
                    is_static: method.is_static,
                    params: method.params.clone(),
                    op_array: new_op_array,
                });
            }
        }
        // Copy trait properties
        for (prop_name, prop_val) in &trait_ce.default_properties {
            if !ce.default_properties.contains_key(prop_name) {
                ce.default_properties.insert(prop_name.clone(), crate::engine::vm::execute_data::clone_val(prop_val));
            }
        }
    }

    let next = lexer.next_token()?;
    if token_is_punct(&next, ";") {
        Ok(lexer.next_token()?)
    } else {
        Ok(next)
    }
}

/// Compile a class definition: class Foo { public $x = 0; public function bar() { ... } }
pub(crate) fn compile_class(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    let name_token = lexer.next_token()?;
    let class_name = name_token.value.as_ref()
        .ok_or("Expected class name")?
        .as_str()
        .to_string();
    let resolved_name = context.resolve_class_name(&class_name);
    let mut ce = ClassEntry::new(&resolved_name);

    // Optional: extends ParentClass
    let mut next = lexer.next_token()?;
    if next.token_type == TokenType::T_EXTENDS {
        let parent_token = lexer.next_token()?;
        let parent_name = parent_token.value.as_ref()
            .ok_or("Expected parent class name")?
            .as_str();
        ce.parent_name = Some(context.resolve_class_name(parent_name));
        next = lexer.next_token()?;
    }

    // Optional: implements Interface1, Interface2
    if next.token_type == TokenType::T_IMPLEMENTS {
        // Skip interface names (just parse past them for now)
        next = lexer.next_token()?;
        while next.token_type == TokenType::T_STRING {
            next = lexer.next_token()?;
            if token_is_punct(&next, ",") {
                next = lexer.next_token()?;
            }
        }
    }

    if !token_is_punct(&next, "{") {
        return Err("Expected '{' after class declaration".to_string());
    }

    parse_class_body(lexer, context, &mut ce)?;
    context.register_class(ce);
    Ok(lexer.next_token()?)
}

/// Compile a trait definition: trait Foo { public function bar() { ... } }
pub(crate) fn compile_trait(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    let name_token = lexer.next_token()?;
    let trait_name = name_token.value.as_ref()
        .ok_or("Expected trait name")?
        .as_str()
        .to_string();
    let resolved_name = context.resolve_class_name(&trait_name);
    let mut ce = ClassEntry::new(&resolved_name);

    let next = lexer.next_token()?;
    if !token_is_punct(&next, "{") {
        return Err("Expected '{' after trait name".to_string());
    }

    parse_class_body(lexer, context, &mut ce)?;

    // Store trait with __trait_ prefix so it doesn't collide with classes
    let trait_key = format!("__trait_{}", resolved_name);
    ce.name = trait_key.clone();
    context.register_class(ce);
    Ok(lexer.next_token()?)
}

/// Compile an enum definition: enum Status { case Pending; case Active; }
/// Also supports backed enums: enum Color: string { case Red = 'red'; }
pub(crate) fn compile_enum(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    let name_token = lexer.next_token()?;
    let enum_name = name_token.value.as_ref()
        .ok_or("Expected enum name")?
        .as_str()
        .to_string();
    let resolved_name = context.resolve_class_name(&enum_name);
    let mut ce = ClassEntry::new(&resolved_name);
    ce.is_enum = true;

    // Optional: backed enum type (: string or : int)
    let mut next = lexer.next_token()?;
    if token_is_punct(&next, ":") {
        let type_token = lexer.next_token()?;
        if type_token.token_type == TokenType::T_STRING {
            let type_str = type_token.value.as_ref().unwrap().as_str();
            ce.enum_base_type = match type_str {
                "string" => Some(crate::engine::types::PhpType::String),
                "int" => Some(crate::engine::types::PhpType::Long),
                _ => None,
            };
        }
        next = lexer.next_token()?;
    }

    if !token_is_punct(&next, "{") {
        return Err("Expected '{' after enum declaration".to_string());
    }

    // Parse enum body
    let mut body_token = lexer.next_token()?;
    while !token_is_punct(&body_token, "}") {
        if body_token.token_type == TokenType::T_EOF {
            return Err("Unexpected EOF in enum body".to_string());
        }

        if body_token.token_type == TokenType::T_CASE {
            let case_name_token = lexer.next_token()?;
            let case_name = case_name_token.value.as_ref()
                .ok_or("Expected case name after 'case'")?
                .as_str()
                .to_string();

            let peek = lexer.next_token()?;
            if peek.token_type == TokenType::T_EQUAL {
                let (case_value, after) = parse_expression(lexer, context)?;
                ce.constants.insert(case_name, case_value);
                body_token = after;
            } else {
                // For pure enums, store the case name as its value
                let name_val = crate::engine::facade::string_val(&case_name);
                ce.constants.insert(case_name, name_val);
                body_token = peek;
            }

            if token_is_punct(&body_token, ";") {
                body_token = lexer.next_token()?;
            }
        } else if body_token.token_type == TokenType::T_FUNCTION {
            // Enum methods (like __construct or custom methods)
            body_token = compile_class_method(lexer, context, &mut ce, Visibility::Public, false)?;
        } else {
            body_token = lexer.next_token()?;
        }
    }

    context.register_class(ce);
    Ok(lexer.next_token()?)
}

/// Compile a class method definition
fn compile_class_method(
    lexer: &mut Lexer,
    context: &mut CompileContext,
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

    let (param_names, variadic, ref_flags) = parse_params(lexer, context)?;
    let params: Vec<String> = param_names
        .iter()
        .map(|p| {
            if p.starts_with('$') {
                p[1..].to_string()
            } else {
                p.to_string()
            }
        })
        .collect();

    // Parse method body: { ... }
    let open_brace = lexer.next_token()?;
    if !token_is_punct(&open_brace, "{") {
        return Err("Expected '{' after method parameters".to_string());
    }

    // Compile method body into a separate op array (inherit source file for __FILE__ / includes)
    let mut method_context = CompileContext::new();
    if let Some(ref f) = context.filename {
        method_context.set_filename(f);
    }
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

    let mut method = ClassMethod {
        name: method_name.clone(),
        visibility,
        is_static,
        params,
        op_array: method_context.take_op_array(),
    };
    method.op_array.variadic_param = variadic;
    method.op_array.ref_params = ref_flags;
    ce.methods.insert(method_name, method);
    Ok(lexer.next_token()?)
}

/// Compile a class property definition
fn compile_class_property(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    ce: &mut ClassEntry,
    token: &Token,
    is_static: bool,
    is_readonly: bool,
    visibility: Visibility,
) -> Result<Token, String> {
    let prop_name = token.value.as_ref().unwrap().as_str();
    let prop_name = if prop_name.starts_with('$') { &prop_name[1..] } else { prop_name };
    let prop_name = prop_name.to_string();

    let peek = lexer.next_token()?;
    let mut next = if peek.token_type == TokenType::T_EQUAL {
        let (default_val, after) = eval_class_default_expr(lexer, context)?;
        if is_static {
            ce.static_properties.insert(prop_name.clone(), default_val);
        } else {
            ce.default_properties.insert(prop_name.clone(), default_val);
        }
        after
    } else {
        if is_static {
            ce.static_properties.insert(prop_name.clone(), null_val());
        } else {
            ce.default_properties.insert(prop_name.clone(), null_val());
        }
        peek
    };

    ce.property_flags.insert(prop_name.clone(), crate::engine::types::PropertyFlags {
        visibility,
        is_static,
        is_readonly,
        is_final: false,
    });

    // Skip semicolon
    if token_is_punct(&next, ";") {
        next = lexer.next_token()?;
    }
    Ok(next)
}

/// Compile a class constant: const NAME = value;
fn compile_class_const(
    lexer: &mut Lexer,
    context: &mut CompileContext,
    ce: &mut ClassEntry,
    _visibility: Visibility,
) -> Result<Token, String> {
    let name_token = lexer.next_token()?;
    let const_name = name_token.value.as_ref()
        .ok_or("Expected constant name after 'const'")?
        .as_str()
        .to_string();

    let eq_token = lexer.next_token()?;
    if eq_token.token_type != TokenType::T_EQUAL {
        return Err("Expected '=' after constant name".to_string());
    }

    let (value, after) = eval_class_default_expr(lexer, context)?;
    ce.constants.insert(const_name, value);

    let mut next = after;
    if token_is_punct(&next, ";") {
        next = lexer.next_token()?;
    }
    Ok(next)
}
