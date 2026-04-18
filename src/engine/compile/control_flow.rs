//! Control Flow Compilation
//!
//! Handles compilation of control flow structures (if, while, for, foreach)

use super::context::CompileContext;
use super::expression::helpers::{
    additive_loop, multiplicative_loop, parse_access_chain, token_is_punct, token_to_primary,
};
use super::expression::operators::parse_comparison_expr_with_token;
use super::expression::{parse_additive_expr_with_initial, parse_expression};
use super::statement::parse_statement_block;
use crate::engine::facade::{clone_val, null_val, result_val, string_val, zero_val};
use crate::engine::lexer::{Lexer, Token, TokenType};
use crate::engine::types::Val;
use crate::engine::vm::{temp_var_ref, Op, Opcode};

/// Compile if statement: if (condition) { body } [else { body }]
pub fn compile_if(lexer: &mut Lexer, context: &mut CompileContext) -> Result<Token, String> {
    let next_token = lexer.next_token()?;
    if next_token.token_type != TokenType::T_STRING
        || next_token.value.as_ref().map(|s| s.as_str()) != Some("(")
    {
        return Err("Expected '(' after 'if'".to_string());
    }

    // Parse condition expression (parse_expression returns the next token, e.g. ')')
    let (condition, paren_token) = parse_expression(lexer, context)?;

    // Expect closing parenthesis (use token returned by parse_expression, do not consume again)
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
        let jmp_index = context.emit_opcode_with_index(Opcode::Jmp, jmp_z1, jmp_z2, jmp_r);

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
pub fn compile_while(lexer: &mut Lexer, context: &mut CompileContext) -> Result<Token, String> {
    let next_token = lexer.next_token()?;
    if next_token.token_type != TokenType::T_STRING
        || next_token.value.as_ref().map(|s| s.as_str()) != Some("(")
    {
        return Err("Expected '(' after 'while'".to_string());
    }

    // Condition must be re-evaluated each iteration: loop head starts *before* condition ops.
    let loop_start_index = context.current_op_index();

    // Parse condition; parse_expression returns the next token (should be ')')
    let (condition, paren_token) = parse_expression(lexer, context)?;
    if paren_token.token_type != TokenType::T_STRING
        || paren_token.value.as_ref().map(|s| s.as_str()) != Some(")")
    {
        return Err("Expected ')' after while condition".to_string());
    }

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
    let jmp_back_index = context.emit_opcode_with_index(Opcode::Jmp, back_z1, back_z2, back_r);
    // Update jump to point back to loop start (condition check)
    context.update_jump_target(jmp_back_index, loop_start_index as u32);

    // Update JmpZ to jump to after the loop
    context.update_jump_target(jmpz_index, context.current_op_index() as u32);

    // After while loop, get the next token
    Ok(lexer.next_token()?)
}

/// Compile for loop: for (init; condition; increment) { body }
pub fn compile_for(lexer: &mut Lexer, context: &mut CompileContext) -> Result<Token, String> {
    let next_token = lexer.next_token()?;
    if next_token.token_type != TokenType::T_STRING
        || next_token.value.as_ref().map(|s| s.as_str()) != Some("(")
    {
        return Err("Expected '(' after 'for'".to_string());
    }

    // Parse initialization expressions (can be empty); supports `$a = expr`, comma-separated clauses, and expression inits (e.g. `$i++`).
    let init_token = lexer.next_token()?;
    if !token_is_punct(&init_token, ";") {
        let mut current_token = init_token;
        loop {
            if token_is_punct(&current_token, ";") {
                break;
            }
            current_token = if current_token.token_type == TokenType::T_VARIABLE {
                let peek = lexer.next_token()?;
                if peek.token_type == TokenType::T_EQUAL {
                    let var_name = current_token.value.as_ref().unwrap().as_str();
                    let (value_zval, after_expr) = parse_expression(lexer, context)?;
                    let var_name_zval = string_val(var_name);
                    let value_zval_op2 = clone_val(&value_zval);
                    context.emit_opcode(Opcode::Assign, var_name_zval, value_zval, value_zval_op2);
                    after_expr
                } else {
                    let initial_zval = token_to_primary(&current_token, context)?;
                    let (left, tok) = parse_access_chain(lexer, context, initial_zval, peek)?;
                    let (left, tok) = multiplicative_loop(lexer, context, left, tok)?;
                    let (_v, after) = additive_loop(lexer, context, left, tok)?;
                    after
                }
            } else {
                let (_v, after) = parse_additive_expr_with_initial(lexer, context, current_token)?;
                after
            };
            if token_is_punct(&current_token, ",") {
                current_token = lexer.next_token()?;
                continue;
            }
            if token_is_punct(&current_token, ";") {
                break;
            }
            return Err(format!(
                "Expected ',' or ';' after for-loop initializer, got {:?}",
                current_token.token_type
            ));
        }
    }

    // Loop head: condition is checked before each body (PHP: for (init; cond; incr) { body }).
    let loop_head_index = context.current_op_index();

    let cond_token = lexer.next_token()?;
    let (condition, cond_next_token) = if token_is_punct(&cond_token, ";") {
        (
            Val::new(
                crate::engine::types::PhpValue::Long(1),
                crate::engine::types::PhpType::Bool,
            ),
            cond_token,
        )
    } else {
        parse_comparison_expr_with_token(lexer, context, Some(cond_token))?
    };

    if !token_is_punct(&cond_next_token, ";") {
        return Err("Expected ';' after for condition".to_string());
    }

    let for_zero = zero_val();
    let for_jmpz_result = result_val(crate::engine::types::PhpType::Bool);
    let jmpz_exit_index = context.emit_opcode_with_index(
        Opcode::JmpZ,
        clone_val(&condition),
        for_zero,
        for_jmpz_result,
    );

    // Parse increment but emit after the body (source order is cond; incr) { body }).
    let incr_token = lexer.next_token()?;
    let incr_ops: Vec<Op> = if token_is_punct(&incr_token, ")") {
        Vec::new()
    } else {
        let incr_begin = context.current_op_index();
        let (_, incr_next_token) = parse_additive_expr_with_initial(lexer, context, incr_token)?;
        if !token_is_punct(&incr_next_token, ")") {
            return Err("Expected ')' after for increment".to_string());
        }
        let incr_end = context.current_op_index();
        context.op_array.ops.drain(incr_begin..incr_end).collect()
    };

    let brace_token = lexer.next_token()?;
    if brace_token.token_type != TokenType::T_STRING
        || brace_token.value.as_ref().map(|s| s.as_str()) != Some("{")
    {
        return Err("Expected '{' after for loop".to_string());
    }

    parse_statement_block(lexer, context)?;

    context.op_array.ops.extend(incr_ops);

    let back_z1 = zero_val();
    let back_z2 = zero_val();
    let back_r = result_val(crate::engine::types::PhpType::Bool);
    let jmp_back_index =
        context.emit_opcode_with_index(Opcode::Jmp, back_z1, back_z2, back_r);
    context.update_jump_target(jmp_back_index, loop_head_index as u32);

    let after_loop = context.current_op_index() as u32;
    context.update_jump_target(jmpz_exit_index, after_loop);

    Ok(lexer.next_token()?)
}

/// Compile foreach loop: foreach ($array as $value) { body }
///                      foreach ($array as $key => $value) { body }
pub fn compile_foreach(lexer: &mut Lexer, context: &mut CompileContext) -> Result<Token, String> {
    let next_token = lexer.next_token()?;
    if next_token.token_type != TokenType::T_STRING
        || next_token.value.as_ref().map(|s| s.as_str()) != Some("(")
    {
        return Err("Expected '(' after 'foreach'".to_string());
    }

    // Parse array/iterable expression (next token is often `as` — do not drop it)
    let (array_expr, after_expr) = parse_expression(lexer, context)?;
    if after_expr.token_type != TokenType::T_AS {
        return Err("Expected 'as' after foreach array expression".to_string());
    }

    // Parse value variable (and optionally key variable)
    let value_token = lexer.next_token()?;
    if value_token.token_type != TokenType::T_VARIABLE {
        return Err("Expected variable after 'as' in foreach".to_string());
    }

    let value_var_name = value_token.value.as_ref().unwrap().as_str();

    let next_after_value = lexer.next_token()?;
    if next_after_value.token_type == TokenType::T_DOUBLE_ARROW {
        return Err("foreach with key => value is not yet supported".to_string());
    }
    if next_after_value.token_type != TokenType::T_STRING
        || next_after_value.value.as_ref().map(|s| s.as_str()) != Some(")")
    {
        return Err("Expected ')' after foreach value variable".to_string());
    }

    let iterator_slot = context.alloc_temp();
    let value_slot = context.alloc_temp();

    context.emit_opcode(
        Opcode::FeReset,
        array_expr.clone(),
        null_val(),
        temp_var_ref(iterator_slot),
    );

    // Loop head: re-enter here (not before FeReset, or the iterator would reset every iteration).
    let loop_start_index = context.current_op_index();

    let end_jmp_idx = context.emit_opcode_with_index(
        Opcode::FeFetch,
        array_expr,
        temp_var_ref(value_slot),
        temp_var_ref(iterator_slot),
    );

    let value_var = crate::engine::vm::var_ref(value_var_name);
    let value_ref = temp_var_ref(value_slot);
    let value_ref_assign = clone_val(&value_ref);
    context.emit_opcode(
        Opcode::Assign,
        value_var,
        value_ref,
        value_ref_assign,
    );

    // Parse foreach body (statements in braces)
    let brace_token = lexer.next_token()?;
    if brace_token.token_type != TokenType::T_STRING
        || brace_token.value.as_ref().map(|s| s.as_str()) != Some("{")
    {
        return Err("Expected '{' after foreach loop".to_string());
    }

    // Parse statements in the foreach body
    parse_statement_block(lexer, context)?;

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
    let jmp_back_index =
        context.emit_opcode_with_index(Opcode::Jmp, jmp_zero_val1, jmp_zero_val2, jmp_result_val);
    context.update_jump_target(jmp_back_index, loop_start_index as u32);

    // FeFetch jumps here when the iterator is exhausted (must be *after* the back-jmp).
    let after_foreach = context.current_op_index() as u32;
    context.update_jump_target(end_jmp_idx, after_foreach);

    Ok(lexer.next_token()?)
}

/// Compile try-catch-finally: try { body } catch (ExceptionClass $e) { body } [finally { body }]
pub fn compile_try_catch(lexer: &mut Lexer, context: &mut CompileContext) -> Result<Token, String> {
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
    let try_begin_index =
        context.emit_opcode_with_index(Opcode::TryCatchBegin, try_zero1, try_zero2, try_result);

    // Parse try body
    parse_statement_block(lexer, context)?;

    // Emit TryCatchEnd - marks the end of the try block
    let try_end_z1 = zero_val();
    let try_end_z2 = zero_val();
    let try_end_r = zero_val();
    let try_end_index =
        context.emit_opcode_with_index(Opcode::TryCatchEnd, try_end_z1, try_end_z2, try_end_r);

    // Emit jump to skip catch/finally blocks (for normal execution)
    let skip_z1 = zero_val();
    let skip_z2 = zero_val();
    let skip_r = zero_val();
    let skip_jmp_index = context.emit_opcode_with_index(Opcode::Jmp, skip_z1, skip_z2, skip_r);

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
pub fn compile_throw(lexer: &mut Lexer, context: &mut CompileContext) -> Result<Token, String> {
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
