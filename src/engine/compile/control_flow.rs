//! Control Flow Compilation
//!
//! Handles compilation of control flow structures (if, while, for, foreach)

use super::context::CompileContext;
use super::expression::parse_expression;
use super::statement::parse_statement_block;
use crate::engine::facade::{result_val, zero_val};
use crate::engine::lexer::{Token, Lexer, TokenType};
use crate::engine::types::Val;
use crate::engine::vm::Opcode;

/// Compile if statement: if (condition) { body } [else { body }]
pub fn compile_if(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    let next_token = lexer.next_token()?;
    if next_token.token_type != TokenType::T_STRING
        || next_token.value.as_ref().map(|s| s.as_str()) != Some("(")
    {
        return Err("Expected '(' after 'if'".to_string());
    }

    // Parse condition expression
    let (condition, _) = parse_expression(lexer, context)?;

    // Expect closing parenthesis
    let paren_token = lexer.next_token()?;
    if paren_token.token_type != TokenType::T_STRING
        || paren_token.value.as_ref().map(|s| s.as_str()) != Some(")")
    {
        return Err("Expected ')' after if condition".to_string());
    }

    // Emit conditional jump (JmpZ) - jump if condition is false
    let cond_zero = zero_val();
    let cond_result = result_val(crate::engine::types::PhpType::Bool);
    let jmpz_index =
        context.emit_opcode_with_index(Opcode::JmpZ, condition, cond_zero, cond_result);

    // Parse if body (statements in braces)
    let brace_token = lexer.next_token()?;
    if brace_token.token_type != TokenType::T_STRING
        || brace_token.value.as_ref().map(|s| s.as_str()) != Some("{")
    {
        return Err("Expected '{' after if condition".to_string());
    }

    // Parse statements in the if body
    parse_statement_block(lexer, context)?;
    let body_end_index = context.current_op_index();

    // Update JmpZ target to jump to after the if body
    context.update_jump_target(jmpz_index, body_end_index as u32);

    // Check for else clause
    let else_token = lexer.next_token()?;
    if else_token.token_type == TokenType::T_ELSE {
        // Parse else body
        let else_brace_token = lexer.next_token()?;
        if else_brace_token.token_type != TokenType::T_STRING
            || else_brace_token.value.as_ref().map(|s| s.as_str()) != Some("{")
        {
            return Err("Expected '{' after 'else'".to_string());
        }

        // Emit unconditional jump to skip else (will be updated after else body)
        let jmp_z1 = zero_val();
        let jmp_z2 = zero_val();
        let jmp_r = result_val(crate::engine::types::PhpType::Bool);
        let jmp_index = context.emit_opcode_with_index(
            Opcode::Jmp,
            jmp_z1,
            jmp_z2,
            jmp_r,
        );

        // Update JmpZ to jump to else body start
        context.update_jump_target(jmpz_index, context.current_op_index() as u32);

        // Parse statements in the else body
        parse_statement_block(lexer, context)?;

        // Update Jmp target to jump to after else body
        context.update_jump_target(jmp_index, context.current_op_index() as u32);
    } else {
        // No else clause, return the token so it can be processed by the caller
        return Ok(else_token);
    }

    // After if/else, get the next token
    Ok(lexer.next_token()?)
}

/// Compile while loop: while (condition) { body }
pub fn compile_while(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    let next_token = lexer.next_token()?;
    if next_token.token_type != TokenType::T_STRING
        || next_token.value.as_ref().map(|s| s.as_str()) != Some("(")
    {
        return Err("Expected '(' after 'while'".to_string());
    }

    // Parse condition expression first
    let (condition, _) = parse_expression(lexer, context)?;

    // Expect closing parenthesis
    let paren_token = lexer.next_token()?;
    if paren_token.token_type != TokenType::T_STRING
        || paren_token.value.as_ref().map(|s| s.as_str()) != Some(")")
    {
        return Err("Expected ')' after while condition".to_string());
    }

    // Mark the start of the loop (condition check location)
    let loop_start_index = context.current_op_index();

    // Emit conditional jump to skip body if condition is false (JmpZ)
    let while_zero = zero_val();
    let while_result = result_val(crate::engine::types::PhpType::Bool);
    let jmpz_index =
        context.emit_opcode_with_index(Opcode::JmpZ, condition, while_zero, while_result);

    // Parse while body (statements in braces)
    let brace_token = lexer.next_token()?;
    if brace_token.token_type != TokenType::T_STRING
        || brace_token.value.as_ref().map(|s| s.as_str()) != Some("{")
    {
        return Err("Expected '{' after while condition".to_string());
    }

    // Parse statements in the while body
    parse_statement_block(lexer, context)?;
    let _loop_end_index = context.current_op_index();

    // Emit unconditional jump back to condition check (loop start)
    let back_z1 = zero_val();
    let back_z2 = zero_val();
    let back_r = result_val(crate::engine::types::PhpType::Bool);
    let jmp_back_index = context.emit_opcode_with_index(
        Opcode::Jmp,
        back_z1,
        back_z2,
        back_r,
    );
    // Update jump to point back to loop start (condition check)
    context.update_jump_target(jmp_back_index, loop_start_index as u32);

    // Update JmpZ to jump to after the loop
    context.update_jump_target(jmpz_index, context.current_op_index() as u32);

    // After while loop, get the next token
    Ok(lexer.next_token()?)
}

/// Compile for loop: for (init; condition; increment) { body }
pub fn compile_for(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    let next_token = lexer.next_token()?;
    if next_token.token_type != TokenType::T_STRING
        || next_token.value.as_ref().map(|s| s.as_str()) != Some("(")
    {
        return Err("Expected '(' after 'for'".to_string());
    }

    // Parse initialization expressions (can be empty)
    let init_token = lexer.next_token()?;
    if init_token.token_type != TokenType::T_STRING
        || init_token.value.as_ref().map(|s| s.as_str()) != Some(";")
    {
        // Has initialization - parse as expression
        let mut current_token = init_token;
        loop {
            if current_token.token_type == TokenType::T_STRING
                && current_token.value.as_ref().map(|s| s.as_str()) == Some(";")
            {
                break;
            }
            // Parse expression (this will consume tokens)
            let (_, next_tok) = parse_expression(lexer, context)?;
            current_token = next_tok;
            if current_token.token_type == TokenType::T_STRING
                && current_token.value.as_ref().map(|s| s.as_str()) == Some(";")
            {
                break;
            }
        }
    }

    // Emit unconditional jump (will be updated to point to condition check after body)
    let zero_val1 = Val::new(
        crate::engine::types::PhpValue::Long(0),
        crate::engine::types::PhpType::Long,
    );
    let zero_val2 = Val::new(
        crate::engine::types::PhpValue::Long(0),
        crate::engine::types::PhpType::Long,
    );
    let result_val = Val::new(
        crate::engine::types::PhpValue::Long(0),
        crate::engine::types::PhpType::Bool,
    );
    let jmp_to_cond_index =
        context.emit_opcode_with_index(Opcode::Jmp, zero_val1, zero_val2, result_val);

    // Mark the start of the loop body
    let loop_start_index = context.current_op_index();

    // Parse condition expression (can be empty - if empty, condition is always true)
    let cond_token = lexer.next_token()?;
    let (condition, cond_next_token) = if cond_token.token_type == TokenType::T_STRING
        && cond_token.value.as_ref().map(|s| s.as_str()) == Some(";")
    {
        // Empty condition - always true
        (
            Val::new(
                crate::engine::types::PhpValue::Long(1),
                crate::engine::types::PhpType::Bool,
            ),
            cond_token,
        )
    } else {
        // Has condition - parse it
        parse_expression(lexer, context)?
    };

    // Expect semicolon after condition
    let _semicolon_token = if cond_next_token.token_type == TokenType::T_STRING
        && cond_next_token.value.as_ref().map(|s| s.as_str()) == Some(";")
    {
        cond_next_token
    } else {
        return Err("Expected ';' after for condition".to_string());
    };

    // Parse increment expressions (can be empty)
    let incr_token = lexer.next_token()?;
    let _increment_end_token = if incr_token.token_type == TokenType::T_STRING
        && incr_token.value.as_ref().map(|s| s.as_str()) == Some(")")
    {
        // Empty increment
        incr_token
    } else {
        // Has increment - parse it (simplified: single expression)
        let (_, incr_next_token) = parse_expression(lexer, context)?;
        // Expect closing parenthesis
        if incr_next_token.token_type != TokenType::T_STRING
            || incr_next_token.value.as_ref().map(|s| s.as_str()) != Some(")")
        {
            return Err("Expected ')' after for increment".to_string());
        }
        incr_next_token
    };

    // Parse for body (statements in braces)
    let brace_token = lexer.next_token()?;
    if brace_token.token_type != TokenType::T_STRING
        || brace_token.value.as_ref().map(|s| s.as_str()) != Some("{")
    {
        return Err("Expected '{' after for loop".to_string());
    }

    // Parse statements in the for body
    parse_statement_block(lexer, context)?;

    // Mark condition check location
    let cond_check_index = context.current_op_index();

    // Update the initial jump to point to condition check
    context.update_jump_target(jmp_to_cond_index, cond_check_index as u32);

    // Compile condition check (re-evaluate condition)
    let cond_zero_val = Val::new(
        crate::engine::types::PhpValue::Long(0),
        crate::engine::types::PhpType::Long,
    );
    let cond_result_val = Val::new(
        crate::engine::types::PhpValue::Long(0),
        crate::engine::types::PhpType::Bool,
    );
    // Create a copy of condition for the jump
    let condition_copy = match &condition.value {
        crate::engine::types::PhpValue::Long(l) => Val::new(
            crate::engine::types::PhpValue::Long(*l),
            condition.get_type(),
        ),
        crate::engine::types::PhpValue::Double(d) => Val::new(
            crate::engine::types::PhpValue::Double(*d),
            condition.get_type(),
        ),
        crate::engine::types::PhpValue::String(s) => {
            let str_copy = crate::engine::string::string_init(s.as_str(), false);
            Val::new(
                crate::engine::types::PhpValue::String(Box::new(str_copy)),
                condition.get_type(),
            )
        }
        _ => Val::new(
            crate::engine::types::PhpValue::Long(1),
            crate::engine::types::PhpType::Bool,
        ),
    };
    let jmp_back_index = context.emit_opcode_with_index(
        Opcode::JmpNZ,
        condition_copy,
        cond_zero_val,
        cond_result_val,
    );
    // Update the jump target to point back to loop start
    context.update_jump_target(jmp_back_index, loop_start_index as u32);

    // After for loop, get the next token
    Ok(lexer.next_token()?)
}

/// Compile foreach loop: foreach ($array as $value) { body }
///                      foreach ($array as $key => $value) { body }
pub fn compile_foreach(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    let next_token = lexer.next_token()?;
    if next_token.token_type != TokenType::T_STRING
        || next_token.value.as_ref().map(|s| s.as_str()) != Some("(")
    {
        return Err("Expected '(' after 'foreach'".to_string());
    }

    // Parse array/iterable expression
    let (_array_expr, _) = parse_expression(lexer, context)?;

    // Expect 'as' keyword
    let as_token = lexer.next_token()?;
    if as_token.token_type != TokenType::T_AS {
        return Err("Expected 'as' after foreach array expression".to_string());
    }

    // Parse value variable (and optionally key variable)
    let value_token = lexer.next_token()?;
    if value_token.token_type != TokenType::T_VARIABLE {
        return Err("Expected variable after 'as' in foreach".to_string());
    }

    let _value_var_name = value_token.value.as_ref().unwrap().as_str();

    // Check if there's a key variable (key => value syntax)
    let next_after_value = lexer.next_token()?;
    let has_key = if next_after_value.token_type == TokenType::T_DOUBLE_ARROW {
        // Has key => value syntax
        let key_token = lexer.next_token()?;
        if key_token.token_type != TokenType::T_VARIABLE {
            return Err("Expected variable after '=>' in foreach".to_string());
        }
        let _key_name = key_token.value.as_ref().unwrap().as_str();
        true
    } else {
        // No key, just value
        if next_after_value.token_type != TokenType::T_STRING
            || next_after_value.value.as_ref().map(|s| s.as_str()) != Some(")")
        {
            return Err("Expected ')' or '=>' after foreach value variable".to_string());
        }
        false
    };

    // Expect closing parenthesis
    let paren_token = if has_key {
        lexer.next_token()?
    } else {
        next_after_value
    };
    if paren_token.token_type != TokenType::T_STRING
        || paren_token.value.as_ref().map(|s| s.as_str()) != Some(")")
    {
        return Err("Expected ')' after foreach variables".to_string());
    }

    // Mark the start of the loop (after reset/fetch operations)
    let loop_start_index = context.current_op_index();

    // TODO: Emit FE_RESET opcode to reset array iterator
    // TODO: Emit FE_FETCH opcode to fetch next element

    // Parse foreach body (statements in braces)
    let brace_token = lexer.next_token()?;
    if brace_token.token_type != TokenType::T_STRING
        || brace_token.value.as_ref().map(|s| s.as_str()) != Some("{")
    {
        return Err("Expected '{' after foreach loop".to_string());
    }

    // Parse statements in the foreach body
    parse_statement_block(lexer, context)?;

    // Emit unconditional jump back to loop start (fetch next element)
    let jmp_zero_val1 = Val::new(
        crate::engine::types::PhpValue::Long(0),
        crate::engine::types::PhpType::Long,
    );
    let jmp_zero_val2 = Val::new(
        crate::engine::types::PhpValue::Long(0),
        crate::engine::types::PhpType::Long,
    );
    let jmp_result_val = Val::new(
        crate::engine::types::PhpValue::Long(0),
        crate::engine::types::PhpType::Bool,
    );
    let jmp_back_index = context.emit_opcode_with_index(
        Opcode::Jmp,
        jmp_zero_val1,
        jmp_zero_val2,
        jmp_result_val,
    );
    // Update jump to point back to loop start
    context.update_jump_target(jmp_back_index, loop_start_index as u32);

    // After foreach loop, get the next token
    Ok(lexer.next_token()?)
}

/// Compile try-catch-finally: try { body } catch (ExceptionClass $e) { body } [finally { body }]
pub fn compile_try_catch(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    // Expect opening brace for try body
    let brace_token = lexer.next_token()?;
    if brace_token.token_type != TokenType::T_STRING
        || brace_token.value.as_ref().map(|s| s.as_str()) != Some("{")
    {
        return Err("Expected '{' after 'try'".to_string());
    }

    // Emit TryCatchBegin opcode - marks the start of the try block
    let try_zero1 = zero_val();
    let try_zero2 = zero_val();
    let try_result = zero_val();
    let try_begin_index = context.emit_opcode_with_index(
        Opcode::TryCatchBegin,
        try_zero1,
        try_zero2,
        try_result,
    );

    // Parse try body
    parse_statement_block(lexer, context)?;

    // Emit TryCatchEnd - marks the end of the try block
    let try_end_z1 = zero_val();
    let try_end_z2 = zero_val();
    let try_end_r = zero_val();
    let try_end_index = context.emit_opcode_with_index(
        Opcode::TryCatchEnd,
        try_end_z1,
        try_end_z2,
        try_end_r,
    );

    // Emit jump to skip catch/finally blocks (for normal execution)
    let skip_z1 = zero_val();
    let skip_z2 = zero_val();
    let skip_r = zero_val();
    let skip_jmp_index = context.emit_opcode_with_index(
        Opcode::Jmp,
        skip_z1,
        skip_z2,
        skip_r,
    );

    // Parse catch blocks (one or more)
    let mut next_token = lexer.next_token()?;
    while next_token.token_type == TokenType::T_CATCH {
        // Expect '('
        let paren_token = lexer.next_token()?;
        if paren_token.token_type != TokenType::T_STRING
            || paren_token.value.as_ref().map(|s| s.as_str()) != Some("(")
        {
            return Err("Expected '(' after 'catch'".to_string());
        }

        // Parse exception class name
        let class_token = lexer.next_token()?;
        if class_token.token_type != TokenType::T_STRING {
            return Err("Expected exception class name in catch".to_string());
        }
        let class_name = class_token
            .value
            .as_ref()
            .map(|s| s.as_str().to_string())
            .unwrap_or_default();

        // Parse variable name
        let var_token = lexer.next_token()?;
        if var_token.token_type != TokenType::T_VARIABLE {
            return Err("Expected variable after exception class in catch".to_string());
        }

        // Expect ')'
        let close_paren = lexer.next_token()?;
        if close_paren.token_type != TokenType::T_STRING
            || close_paren.value.as_ref().map(|s| s.as_str()) != Some(")")
        {
            return Err("Expected ')' after catch variable".to_string());
        }

        // Expect '{'
        let catch_brace = lexer.next_token()?;
        if catch_brace.token_type != TokenType::T_STRING
            || catch_brace.value.as_ref().map(|s| s.as_str()) != Some("{")
        {
            return Err("Expected '{' after catch clause".to_string());
        }

        // Emit CatchBegin opcode with exception class name
        let class_str = crate::engine::string::string_init(&class_name, false);
        let class_zval = Val::new(
            crate::engine::types::PhpValue::String(Box::new(class_str)),
            crate::engine::types::PhpType::String,
        );
        let catch_z2 = zero_val();
        let catch_r = zero_val();
        context.emit_opcode(Opcode::CatchBegin, class_zval, catch_z2, catch_r);

        // Parse catch body
        parse_statement_block(lexer, context)?;

        // Emit CatchEnd
        let catch_end_z1 = zero_val();
        let catch_end_z2 = zero_val();
        let catch_end_r = zero_val();
        context.emit_opcode(Opcode::CatchEnd, catch_end_z1, catch_end_z2, catch_end_r);

        // Get next token to check for more catch blocks or finally
        next_token = lexer.next_token()?;
    }

    // Check for finally block
    if next_token.token_type == TokenType::T_FINALLY {
        let finally_brace = lexer.next_token()?;
        if finally_brace.token_type != TokenType::T_STRING
            || finally_brace.value.as_ref().map(|s| s.as_str()) != Some("{")
        {
            return Err("Expected '{' after 'finally'".to_string());
        }

        // Emit FinallyBegin
        let fin_z1 = zero_val();
        let fin_z2 = zero_val();
        let fin_r = zero_val();
        context.emit_opcode(Opcode::FinallyBegin, fin_z1, fin_z2, fin_r);

        // Parse finally body
        parse_statement_block(lexer, context)?;

        // Emit FinallyEnd
        let fin_end_z1 = zero_val();
        let fin_end_z2 = zero_val();
        let fin_end_r = zero_val();
        context.emit_opcode(Opcode::FinallyEnd, fin_end_z1, fin_end_z2, fin_end_r);

        // Get next token
        next_token = lexer.next_token()?;
    }

    // Update the skip jump to point past all catch/finally blocks
    context.update_jump_target(skip_jmp_index, context.current_op_index() as u32);

    // Update TryCatchBegin extended_value to point to first catch block
    context.update_jump_target(try_begin_index, (try_end_index + 2) as u32);

    Ok(next_token)
}

/// Compile throw statement: throw new ExceptionClass("message");
pub fn compile_throw(
    lexer: &mut Lexer,
    context: &mut CompileContext,
) -> Result<Token, String> {
    // Parse the expression to throw (typically: new ExceptionClass(...))
    let (throw_expr, next_token) = parse_expression(lexer, context)?;

    // Emit Throw opcode
    let throw_z2 = zero_val();
    let throw_r = zero_val();
    context.emit_opcode(Opcode::Throw, throw_expr, throw_z2, throw_r);

    // Expect semicolon
    if next_token.token_type == TokenType::T_STRING
        && next_token.value.as_ref().map(|s| s.as_str()) == Some(";")
    {
        Ok(lexer.next_token()?)
    } else {
        Ok(next_token)
    }
}
