//! Function Compilation
//!
//! Handles compilation of PHP function definitions

use super::context::CompileContext;
use super::statement::parse_statement_block;
use crate::engine::lexer::{Token, Lexer, TokenType};

/// Compile function definition: function name($param1, $param2) { body }
pub fn compile_function(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    // Parse function name
    let name_token = lexer.next_token()?;
    if name_token.token_type != TokenType::T_STRING {
        // Check if it's a variable (for anonymous functions)
        if name_token.token_type != TokenType::T_VARIABLE {
            return Err("Expected function name or variable after 'function'".to_string());
        }
        // Anonymous function - for now, skip
        return Ok(name_token);
    }

    let function_name = name_token.value.as_ref().unwrap().as_str();

    // Expect opening parenthesis
    let paren_token = lexer.next_token()?;
    if paren_token.token_type != TokenType::T_STRING
        || paren_token.value.as_ref().map(|s| s.as_str()) != Some("(")
    {
        return Err("Expected '(' after function name".to_string());
    }

    // Parse parameters (can be empty)
    let mut _params = Vec::new();
    let mut current_token = lexer.next_token()?;

    // Check if parameters list is empty
    if current_token.token_type == TokenType::T_STRING
        && current_token.value.as_ref().map(|s| s.as_str()) == Some(")")
    {
        // No parameters
    } else {
        // Parse parameters
        loop {
            // Expect variable parameter
            if current_token.token_type != TokenType::T_VARIABLE {
                return Err("Expected variable parameter in function definition".to_string());
            }

            let param_name = current_token.value.as_ref().unwrap().as_str();
            _params.push(param_name.to_string());

            // Get next token (could be comma, closing paren, or default value)
            current_token = lexer.next_token()?;

            // Check for default value assignment
            if current_token.token_type == TokenType::T_EQUAL {
                // Has default value - parse the expression
                let (_, next_token) = super::expression::parse_expression(lexer, context)?;
                current_token = next_token;
            }

            // Check if we're done with parameters
            if current_token.token_type == TokenType::T_STRING
                && current_token.value.as_ref().map(|s| s.as_str()) == Some(")")
            {
                break;
            }

            // Expect comma for next parameter
            if current_token.token_type != TokenType::T_STRING
                || current_token.value.as_ref().map(|s| s.as_str()) != Some(",")
            {
                return Err("Expected ',' or ')' after parameter".to_string());
            }

            current_token = lexer.next_token()?;
        }
    }

    // Create a new compilation context for the function body
    let mut func_context = CompileContext::new();
    func_context.set_filename(context.filename.as_deref().unwrap_or(""));
    func_context.set_line(current_token.lineno);

    // Set function name in the op_array
    func_context.op_array.function_name = Some(function_name.to_string());

    // Parse function body (statements in braces)
    let brace_token = lexer.next_token()?;
    if brace_token.token_type != TokenType::T_STRING
        || brace_token.value.as_ref().map(|s| s.as_str()) != Some("{")
    {
        return Err("Expected '{' after function parameters".to_string());
    }

    // Parse statements in the function body
    parse_statement_block(lexer, &mut func_context)?;

    // Finalize the function op_array
    let func_op_array = func_context.finalize();

    // Store the function in the function table for later lookup
    context
        .function_table
        .store_function(function_name, func_op_array);

    // After function definition, get the next token
    Ok(lexer.next_token()?)
}
