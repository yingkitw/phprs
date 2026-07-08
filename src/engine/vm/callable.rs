//! Callable invocation helper.
//!
//! Reusable entry point for invoking a PHP callable (string-named builtin or
//! user function) from built-in functions such as `array_map`, `array_filter`,
//! and `call_user_func`. It mirrors the save/restore discipline used by the
//! `DoFCall` opcode handler so that re-entering the VM is safe.
//!
//! Limitations: only string callables are supported. Closure objects and
//! `[$obj, 'method']` array callables are not handled here yet.

use super::builtins::{execute_builtin_function, is_builtin_function};
use super::execute::execute_ex_returning;
use super::execute_data::{clone_val, ExecuteData};
use super::opcodes::{Op, OpArray};
use crate::engine::compile::function_table::FunctionTable;
use crate::engine::types::{PhpValue, Val};

/// Resolve a callable value to a plain function name string.
///
/// Returns `None` for callables this helper cannot invoke (closures, array
/// callables, objects). Callers decide how to treat that case.
pub fn callable_name(callable: &Val) -> Option<String> {
    match &callable.value {
        PhpValue::String(s) => Some(s.as_str().to_string()),
        _ => None,
    }
}

/// Invoke a callable with positional arguments.
///
/// Tries built-in functions first, then user-defined functions. Returns
/// `Ok(Some(value))` when the callable returns a value, `Ok(None)` for a
/// void/empty return, and `Err` if the callable cannot be found or fails.
pub fn invoke_callable(
    execute_data: &mut ExecuteData,
    callable: &Val,
    args: &[Val],
) -> Result<Option<Val>, String> {
    let name = match callable_name(callable) {
        Some(n) => n,
        None => {
            return Err(
                "phprs: only string callables are supported in callbacks so far".to_string(),
            )
        }
    };

    if is_builtin_function(&name) {
        return execute_builtin_function(&name, args, execute_data);
    }

    invoke_user_function(execute_data, &name, args)
}

/// Invoke a user-defined function by name with positional arguments.
pub fn invoke_user_function(
    execute_data: &mut ExecuteData,
    name: &str,
    args: &[Val],
) -> Result<Option<Val>, String> {
    let func_data: Option<(Vec<String>, Option<String>, OpArray)> = execute_data
        .function_table
        .as_ref()
        .and_then(|ft| ft.downcast_ref::<FunctionTable>())
        .and_then(|ft| ft.lookup_function(name))
        .map(|func_op_array| {
            let param_names: Vec<String> = func_op_array
                .vars
                .iter()
                .map(|v| match &v.value {
                    PhpValue::String(s) => {
                        let n = s.as_str();
                        if let Some(rest) = n.strip_prefix('$') {
                            rest.to_string()
                        } else {
                            n.to_string()
                        }
                    }
                    _ => String::new(),
                })
                .collect();
            let variadic = func_op_array.variadic_param.clone();

            let mut cloned = OpArray::with_capacity(
                func_op_array.ops.len(),
                func_op_array.filename.clone().unwrap_or_default(),
            );
            cloned.function_name = func_op_array.function_name.clone();
            cloned.ref_params = func_op_array.ref_params.clone();
            cloned.variadic_param = func_op_array.variadic_param.clone();
            for op in &func_op_array.ops {
                cloned.add_op(Op::new(
                    op.opcode,
                    clone_val(&op.op1),
                    clone_val(&op.op2),
                    clone_val(&op.result),
                    op.extended_value,
                ));
            }
            (param_names, variadic, cloned)
        });

    let Some((param_names, variadic_param, func_op_array)) = func_data else {
        return Err(format!("Call to undefined function {name}()"));
    };

    let arg_names: Vec<Option<String>> = vec![None; args.len()];
    let arg_by_ref: Vec<bool> = vec![false; args.len()];
    let ref_params = func_op_array.ref_params.clone();

    // Save current execution state (mirrors the DoFCall handler).
    let saved_op = execute_data.current_op;
    let saved_op_array = execute_data.op_array.take();
    let saved_temps = std::mem::take(&mut execute_data.temp_vars);
    let saved_ref_caller_scope = execute_data.ref_caller_scope.take();
    let saved_symbol_table = execute_data.symbol_table.take();
    let saved_call_arg_stack = std::mem::take(&mut execute_data.call_arg_stack);
    let saved_call_args = std::mem::take(&mut execute_data.call_args);
    let saved_call_arg_names = std::mem::take(&mut execute_data.call_arg_names);
    let saved_call_arg_by_ref = std::mem::take(&mut execute_data.call_arg_by_ref);
    let saved_global_imports = std::mem::take(&mut execute_data.global_imports);
    let saved_ref_bindings = std::mem::take(&mut execute_data.ref_param_bindings);

    if execute_data.global_script_table.is_none() {
        if let Some(ref saved) = saved_symbol_table {
            execute_data.global_script_table =
                Some(ExecuteData::clone_php_array(saved));
        }
    }

    execute_data.ref_caller_scope = saved_symbol_table;
    execute_data.symbol_table = Some(crate::engine::types::PhpArray::new());

    super::dispatch_handlers::bind_call_args(
        execute_data,
        &param_names,
        args,
        &arg_names,
        &variadic_param,
        &ref_params,
        &arg_by_ref,
    );

    let saved_script_dir = execute_data.current_script_dir.clone();
    let saved_magic_dir = execute_data.constants.get("__DIR__").map(clone_val);
    let saved_magic_file = execute_data.constants.get("__FILE__").map(clone_val);

    let (_status, return_val) = execute_ex_returning(execute_data, &func_op_array);

    // Restore execution state.
    execute_data.symbol_table = execute_data.ref_caller_scope.take();
    if let Some(mut saved) = execute_data.symbol_table.take() {
        execute_data.merge_globals_into(&mut saved);
        execute_data.symbol_table = Some(saved);
    }
    execute_data.ref_caller_scope = saved_ref_caller_scope;
    execute_data.global_imports = saved_global_imports;
    execute_data.ref_param_bindings = saved_ref_bindings;
    execute_data.temp_vars = saved_temps;
    execute_data.op_array = saved_op_array;
    execute_data.current_op = saved_op;
    execute_data.call_arg_stack = saved_call_arg_stack;
    execute_data.call_args = saved_call_args;
    execute_data.call_arg_names = saved_call_arg_names;
    execute_data.call_arg_by_ref = saved_call_arg_by_ref;
    execute_data.current_script_dir = saved_script_dir;
    match saved_magic_dir {
        Some(v) => {
            execute_data.constants.insert("__DIR__".to_string(), v);
        }
        None => {
            execute_data.constants.remove("__DIR__");
        }
    }
    match saved_magic_file {
        Some(v) => {
            execute_data.constants.insert("__FILE__".to_string(), v);
        }
        None => {
            execute_data.constants.remove("__FILE__");
        }
    }

    Ok(return_val)
}
