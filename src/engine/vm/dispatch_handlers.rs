//! Optimized opcode handlers for direct dispatch
//!
//! This module contains individual handler functions for each opcode,
//! designed for maximum performance with direct function calls and JIT optimization.

use super::builtins::execute_builtin_function;
use super::execute_data::{
    clone_val, is_temp_ref, is_var_ref, resolve_operand, result_slot, ExecResult, ExecuteData,
};
use super::opcodes::{Op, OpArray, Opcode};

use crate::engine::jit::{increment_execution_counter, try_inline_operation};
use crate::engine::types::{PhpType, PhpValue, Val};

/// Call __toString magic method on an object if it exists
#[inline]
fn call_magic_tostring(val: &Val, execute_data: &mut ExecuteData) -> Option<crate::engine::types::PhpString> {
    if let PhpValue::Object(ref obj) = val.value {
        if let Some(ce) = execute_data.class_table.get(&obj.class_name) {
            if let Some(magic) = ce.methods.get("__toString") {
                let ops: Vec<Op> = magic
                    .op_array
                    .ops
                    .iter()
                    .map(|op| {
                        Op::new(
                            op.opcode,
                            clone_val(&op.op1),
                            clone_val(&op.op2),
                            clone_val(&op.result),
                            op.extended_value,
                        )
                    })
                    .collect();

                let saved_current_op = execute_data.current_op;
                let saved_op_array = execute_data.op_array.take();
                let saved_called_class = execute_data.called_class.clone();
                execute_data.called_class = Some(obj.class_name.clone());
                execute_data.set_var("this", clone_val(val));

                let mut method_op_array = OpArray::new(format!("{}::__toString", obj.class_name));
                method_op_array.ops = ops;
                let (_status, return_val) =
                    super::execute::execute_ex_returning(execute_data, &method_op_array);
                execute_data.op_array = saved_op_array;
                execute_data.current_op = saved_current_op;
                execute_data.called_class = saved_called_class;

                if let Some(ret) = return_val {
                    return Some(crate::engine::operators::zval_get_string(&ret));
                }
            }
        }
    }
    None
}

/// Resolve a relative include/require path like PHP: try the path relative to the process
/// current working directory first, then relative to the including script's directory.
#[inline]
fn resolve_include_path(path_str: &str, script_dir: Option<&str>) -> String {
    use std::path::Path;
    if path_str.starts_with('/')
        || (path_str.len() >= 2 && path_str.get(1..2) == Some(":"))
    {
        return path_str.to_string();
    }
    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join(path_str);
        if p.is_file() {
            return p.to_string_lossy().into_owned();
        }
    }
    if let Some(dir) = script_dir {
        return Path::new(dir).join(path_str).to_string_lossy().into_owned();
    }
    path_str.to_string()
}

#[inline]
pub fn execute_nop(_op: &Op, _execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_fetch_var(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, val);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_add(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);

    // Try JIT inline optimization first
    if let Some(result) = try_inline_operation(Opcode::Add, &op1, &op2) {
        if let Some(slot) = result_slot(op) {
            execute_data.set_temp(slot, result);
        }
        return Ok(ExecResult::Continue);
    }

    // Fallback to regular implementation
    let result = crate::engine::operators::zval_add(&op1, &op2);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_sub(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let result = crate::engine::operators::zval_sub(&op1, &op2);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_mul(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let result = crate::engine::operators::zval_mul(&op1, &op2);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_div(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    match crate::engine::operators::zval_div(&op1, &op2) {
        Ok(result) => {
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, result);
            }
            Ok(ExecResult::Continue)
        }
        Err(e) => Err(e),
    }
}

#[inline]
pub fn execute_concat(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);

    // Try JIT inline optimization first
    if let Some(result) = try_inline_operation(Opcode::Concat, &op1, &op2) {
        if let Some(slot) = result_slot(op) {
            execute_data.set_temp(slot, result);
        }
        return Ok(ExecResult::Continue);
    }

    // Optimized string concatenation - pre-allocate exact capacity
    let s1 = if let Some(tostr) = call_magic_tostring(&op1, execute_data) {
        tostr
    } else {
        crate::engine::operators::zval_get_string(&op1)
    };
    let s2 = if let Some(tostr) = call_magic_tostring(&op2, execute_data) {
        tostr
    } else {
        crate::engine::operators::zval_get_string(&op2)
    };
    let s1_len = s1.val.len();
    let s2_len = s2.val.len();
    let mut combined = String::with_capacity(s1_len + s2_len);
    combined.push_str(s1.as_str());
    combined.push_str(s2.as_str());

    let result = Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(
            &combined, false,
        ))),
        PhpType::String,
    );
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_mod(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    match crate::engine::operators::zval_mod(&op1, &op2) {
        Ok(result) => {
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, result);
            }
            Ok(ExecResult::Continue)
        }
        Err(e) => Err(e),
    }
}

#[inline]
pub fn execute_bool_not(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let b = !crate::engine::operators::zval_get_bool(&val);
    let result = Val::new(
        PhpValue::Long(if b { 1 } else { 0 }),
        if b { PhpType::True } else { PhpType::False },
    );
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_bool_and(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let r = crate::engine::operators::zval_get_bool(&op1)
        && crate::engine::operators::zval_get_bool(&op2);
    let result = Val::new(
        PhpValue::Long(if r { 1 } else { 0 }),
        if r { PhpType::True } else { PhpType::False },
    );
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_bool_or(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let r = crate::engine::operators::zval_get_bool(&op1)
        || crate::engine::operators::zval_get_bool(&op2);
    let result = Val::new(
        PhpValue::Long(if r { 1 } else { 0 }),
        if r { PhpType::True } else { PhpType::False },
    );
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_bool_xor(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let r = crate::engine::operators::zval_get_bool(&op1)
        ^ crate::engine::operators::zval_get_bool(&op2);
    let result = Val::new(
        PhpValue::Long(if r { 1 } else { 0 }),
        if r { PhpType::True } else { PhpType::False },
    );
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_jmp(op: &Op, _execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    Ok(ExecResult::Jump(op.extended_value))
}

#[inline]
pub fn execute_pow(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let v1 = crate::engine::operators::zval_get_double(&op1);
    let v2 = crate::engine::operators::zval_get_double(&op2);
    let result = Val::new(PhpValue::Double(v1.powf(v2)), PhpType::Double);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_assign(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op2, execute_data);
    if let PhpValue::String(var_name) = &op.op1.value {
        let name = var_name.as_str();
        let clean = if name.starts_with('$') {
            &name[1..]
        } else {
            name
        };
        execute_data.set_var(clean, clone_val(&val));
    }
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, val);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_assign_dim(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op2, execute_data);
    let append = op.extended_value == 1;
    let key = if append {
        None
    } else {
        Some(resolve_operand(&op.result, execute_data))
    };

    let write_back = |execute_data: &mut ExecuteData, container: Val, op1: &Val| {
        if is_temp_ref(op1) {
            if let PhpValue::Long(idx) = op1.value {
                execute_data.set_temp(idx as usize, container);
            }
        } else if let PhpValue::String(var_s) = &op1.value {
            let name = var_s.as_str();
            let clean = if name.starts_with('$') {
                &name[1..]
            } else {
                name
            };
            execute_data.set_var(clean, container);
        }
    };

    let mut container = if is_temp_ref(&op.op1) {
        resolve_operand(&op.op1, execute_data)
    } else if let PhpValue::String(var_s) = &op.op1.value {
        let name = var_s.as_str();
        let clean = if name.starts_with('$') {
            &name[1..]
        } else {
            name
        };
        execute_data.get_var(clean)
    } else {
        return Ok(ExecResult::Continue);
    };

    if container.get_type() == PhpType::Null {
        container = Val::new(
            PhpValue::Array(Box::new(crate::engine::types::PhpArray::new())),
            PhpType::Array,
        );
    }

    if let PhpValue::Array(ref mut arr) = container.value {
        if append {
            let next_idx = arr.n_num_used as u64;
            let _ = crate::engine::hash::hash_add_or_update(arr, None, next_idx, val, 0);
        } else if let Some(key) = key {
            match &key.value {
                PhpValue::Long(i) => {
                    let _ = crate::engine::hash::hash_add_or_update(arr, None, *i as u64, val, 0);
                }
                PhpValue::String(ks) => {
                    let key_zs =
                        Box::new(crate::engine::string::string_init(ks.as_str(), false));
                    let _ = crate::engine::hash::hash_add_or_update(arr, Some(&*key_zs), 0, val, 0);
                }
                _ => {}
            }
        }
        write_back(execute_data, container, &op.op1);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_echo(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let s = if let Some(tostr) = call_magic_tostring(&val, execute_data) {
        tostr
    } else {
        crate::engine::operators::zval_get_string(&val)
    };
    let _ = crate::php::output::php_output_write(s.as_bytes());
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_return(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    Ok(ExecResult::Return(val))
}

#[inline]
pub fn execute_do_fcall(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let resolved_op1 = if is_var_ref(&op.op1) || is_temp_ref(&op.op1) {
        resolve_operand(&op.op1, execute_data)
    } else {
        clone_val(&op.op1)
    };

    // Magic method: __invoke for callable objects
    if let PhpValue::Object(ref obj) = resolved_op1.value {
        if let Some(ce) = execute_data.class_table.get(&obj.class_name) {
            if let Some(magic) = ce.methods.get("__invoke") {
                let params = magic.params.clone();
                let ops: Vec<Op> = magic
                    .op_array
                    .ops
                    .iter()
                    .map(|op| {
                        Op::new(
                            op.opcode,
                            clone_val(&op.op1),
                            clone_val(&op.op2),
                            clone_val(&op.result),
                            op.extended_value,
                        )
                    })
                    .collect();

                let saved_current_op = execute_data.current_op;
                let saved_op_array = execute_data.op_array.take();
                let saved_script_dir = execute_data.current_script_dir.clone();
                let saved_called_class = execute_data.called_class.clone();
                execute_data.called_class = Some(obj.class_name.clone());
                execute_data.set_var("this", clone_val(&resolved_op1));

                let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
                let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
                for (i, param_name) in params.iter().enumerate() {
                    if let Some(arg) = args.get(i) {
                        execute_data.set_var(param_name, clone_val(arg));
                    }
                }

                let mut method_op_array = OpArray::new(format!("{}::__invoke", obj.class_name));
                method_op_array.ops = ops;
                let (_status, return_val) =
                    super::execute::execute_ex_returning(execute_data, &method_op_array);
                execute_data.op_array = saved_op_array;
                execute_data.current_op = saved_current_op;
                execute_data.current_script_dir = saved_script_dir;
                execute_data.called_class = saved_called_class;

                if let Some(slot) = result_slot(op) {
                    if let Some(ret) = return_val {
                        execute_data.set_temp(slot, ret);
                    } else {
                        execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
                    }
                }
                return Ok(ExecResult::Continue);
            }
        }
    }

    let func_name = crate::engine::operators::zval_get_string(&resolved_op1)
        .as_str()
        .to_ascii_lowercase();

    // Check JIT compilation for hot functions
    increment_execution_counter(&func_name);

    let (base, names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
    let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
    let arg_names: Vec<Option<String>> = execute_data.call_arg_names.drain(names_base..).collect();

    match execute_builtin_function(&func_name, &args, execute_data)? {
        Some(result) => {
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, result);
            }
            Ok(ExecResult::Continue)
        }
        None => {
            // User-defined function lookup (skip JIT fallback — generic JIT interpreter can loop)
            let func_data: Option<(Vec<String>, Option<String>, super::opcodes::OpArray)> =
                execute_data
                .function_table
                .as_ref()
                .and_then(|ft| {
                    ft.downcast_ref::<crate::engine::compile::function_table::FunctionTable>()
                })
                .and_then(|ft| ft.lookup_function(&func_name))
                .map(|func_op_array| {
                    // Extract param names from vars - optimized with capacity
                    let param_names: Vec<String> = func_op_array
                        .vars
                        .iter()
                        .map(|v| {
                            if let PhpValue::String(ref s) = v.value {
                                let name = s.as_str();
                                if name.starts_with('$') {
                                    name[1..].to_string()
                                } else {
                                    name.to_string()
                                }
                            } else {
                                String::new()
                            }
                        })
                        .collect();
                        let variadic = func_op_array.variadic_param.clone();
                    // Clone the op array with capacity
                    let mut cloned = super::opcodes::OpArray::with_capacity(
                        func_op_array.ops.len(),
                        func_op_array.filename.clone().unwrap_or_default(),
                    );
                    cloned.function_name = func_op_array.function_name.clone();
                    for op in &func_op_array.ops {
                        cloned.add_op(super::opcodes::Op::new(
                            op.opcode,
                            clone_val(&op.op1),
                            clone_val(&op.op2),
                            clone_val(&op.result),
                            op.extended_value,
                        ));
                    }
                        (param_names, variadic, cloned)
                    });

            if let Some((param_names, variadic_param, func_op_array)) = func_data {
                // Note: JIT compilation check removed to prevent deadlock
                // The function will be JIT compiled on subsequent calls if it's hot enough

                // Save current execution state
                let saved_op = execute_data.current_op;
                let saved_op_array = execute_data.op_array.take();
                let saved_temps = std::mem::take(&mut execute_data.temp_vars);
                let saved_symbol_table = execute_data.symbol_table.take();
                let saved_call_arg_stack = std::mem::take(&mut execute_data.call_arg_stack);
                let saved_call_args = std::mem::take(&mut execute_data.call_args);
                let saved_call_arg_names = std::mem::take(&mut execute_data.call_arg_names);
                let saved_global_imports = std::mem::take(&mut execute_data.global_imports);

                if execute_data.global_script_table.is_none() {
                    if let Some(ref saved) = saved_symbol_table {
                        execute_data.global_script_table =
                            Some(super::execute_data::ExecuteData::clone_php_array(saved));
                    }
                }

                // Set up fresh symbol table for function scope
                execute_data.symbol_table = Some(crate::engine::types::PhpArray::new());

                // Bind arguments to parameter names (supports named args and variadic)
                bind_call_args(execute_data, &param_names, &args, &arg_names, &variadic_param);

                let saved_script_dir = execute_data.current_script_dir.clone();
                let saved_magic_dir = execute_data.constants.get("__DIR__").map(clone_val);
                let saved_magic_file = execute_data.constants.get("__FILE__").map(clone_val);
                // Execute the function and capture return value
                let (_status, return_val) =
                    super::execute::execute_ex_returning(execute_data, &func_op_array);

                // Restore execution state
                if let Some(mut saved) = saved_symbol_table {
                    execute_data.merge_globals_into(&mut saved);
                    execute_data.symbol_table = Some(saved);
                } else {
                    execute_data.symbol_table = saved_symbol_table;
                }
                execute_data.global_imports = saved_global_imports;
                execute_data.temp_vars = saved_temps;
                execute_data.op_array = saved_op_array;
                execute_data.current_op = saved_op;
                execute_data.call_arg_stack = saved_call_arg_stack;
                execute_data.call_args = saved_call_args;
                execute_data.call_arg_names = saved_call_arg_names;
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

                // Store return value in result temp slot
                if let Some(ret) = return_val {
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, ret);
                    }
                }

                return Ok(ExecResult::Continue);
            }

            eprintln!("Warning: Call to undefined function {}()", func_name);
            Ok(ExecResult::Continue)
        }
    }
}

#[inline]
pub fn execute_bind_global(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let name_val = resolve_operand(&op.op1, execute_data);
    let name = crate::engine::operators::zval_get_string(&name_val);
    execute_data.bind_global(name.as_str());
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_jmpz(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let b = crate::engine::operators::zval_get_bool(&val);
    if !b {
        Ok(ExecResult::Jump(op.extended_value))
    } else {
        Ok(ExecResult::Continue)
    }
}

#[inline]
pub fn execute_jmpnz(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let b = crate::engine::operators::zval_get_bool(&val);
    if b {
        Ok(ExecResult::Jump(op.extended_value))
    } else {
        Ok(ExecResult::Continue)
    }
}

#[inline]
pub fn execute_init_fcall(_op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    execute_data.call_arg_stack.push((
        execute_data.call_args.len(),
        execute_data.call_arg_names.len(),
    ));
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_send_val(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    execute_data.call_args.push(val);
    execute_data.call_arg_names.push(None);
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_send_val_named(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let name_val = resolve_operand(&op.op2, execute_data);
    let name = crate::engine::operators::zval_get_string(&name_val);
    execute_data.call_args.push(val);
    execute_data.call_arg_names.push(Some(name.as_str().to_string()));
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_include(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let path_val = resolve_operand(&op.op1, execute_data);
    let path = crate::engine::operators::zval_get_string(&path_val);
    let path_str = path.as_str();
    let resolved = resolve_include_path(path_str, execute_data.current_script_dir.as_deref());

    let is_once = op.extended_value == 2 || op.extended_value == 3;
    if is_once && execute_data.included_files.contains(&resolved) {
        return Ok(ExecResult::Continue);
    }
    match crate::engine::compile::compile_file_with_functions(&resolved) {
        Ok((included_op_array, included_ft)) => {
            execute_data.included_files.insert(resolved.clone());
            crate::engine::compile::function_table::merge_into_execute_data(
                execute_data,
                included_ft,
            );
            let saved_op_array = execute_data.op_array.take();
            let saved_current_op = execute_data.current_op;
            let saved_script_dir = execute_data.current_script_dir.clone();
            let saved_magic_dir = execute_data.constants.get("__DIR__").map(clone_val);
            let saved_magic_file = execute_data.constants.get("__FILE__").map(clone_val);
            let result = super::execute::execute_ex(execute_data, &included_op_array);
            execute_data.op_array = saved_op_array;
            execute_data.current_op = saved_current_op;
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
            if result == crate::engine::types::PhpResult::Failure {
                return Err(format!("Failed to execute included file: {}", resolved));
            }
            Ok(ExecResult::Continue)
        }
        Err(e) => {
            if op.extended_value == 1 || op.extended_value == 3 {
                return Err(format!("require({}): {}", resolved, e));
            } else {
                eprintln!("Warning: include({}): {}", resolved, e);
                return Ok(ExecResult::Continue);
            }
        }
    }
}

#[inline]
pub fn execute_coalesce(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    if op1.get_type() != PhpType::Null {
        if let Some(slot) = result_slot(op) {
            execute_data.set_temp(slot, op1);
        }
    } else {
        let op2 = resolve_operand(&op.op2, execute_data);
        if let Some(slot) = result_slot(op) {
            execute_data.set_temp(slot, op2);
        }
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_qm_assign(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, val);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_jmp_null_z(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    if val.get_type() == PhpType::Null {
        Ok(ExecResult::Jump(op.extended_value))
    } else {
        Ok(ExecResult::Continue)
    }
}

#[inline]
pub fn execute_init_array(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let arr = crate::engine::types::PhpArray::new();
    let arr_zval = Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, arr_zval);
    }
    Ok(ExecResult::Continue)
}

#[inline]
fn temp_slot_index(v: &Val) -> Option<usize> {
    if is_temp_ref(v) {
        if let PhpValue::Long(i) = v.value {
            return Some(i as usize);
        }
    }
    None
}

/// foreach: op1 = array, result = iterator temp (next numeric index to read)
#[inline]
pub fn execute_fe_reset(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let arr = resolve_operand(&op.op1, execute_data);
    let Some(iter_slot) = result_slot(op) else {
        return Ok(ExecResult::Continue);
    };
    execute_data.fe_key_slot = temp_slot_index(&op.op2).map(|s| s as u32);
    if matches!(arr.value, PhpValue::Array(_)) {
        execute_data.set_temp(
            iter_slot,
            Val::new(PhpValue::Long(0), PhpType::Long),
        );
    }
    Ok(ExecResult::Continue)
}

/// foreach: op1 = array, op2 = temp slot for current element, result = iterator temp
#[inline]
pub fn execute_fe_fetch(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let arr = resolve_operand(&op.op1, execute_data);
    let Some(iter_slot) = result_slot(op) else {
        return Ok(ExecResult::Continue);
    };
    let Some(value_slot) = temp_slot_index(&op.op2) else {
        return Ok(ExecResult::Continue);
    };

    let PhpValue::Array(ref arr) = arr.value else {
        return Ok(ExecResult::Jump(op.extended_value));
    };

    let iter_val = execute_data.get_temp(iter_slot);
    let current_idx = if let PhpValue::Long(i) = iter_val.value {
        i as u64
    } else {
        0
    };

    if current_idx >= arr.n_num_of_elements as u64 {
        return Ok(ExecResult::Jump(op.extended_value));
    }

    let Some(bucket) = arr.ar_data.get(current_idx as usize) else {
        return Ok(ExecResult::Jump(op.extended_value));
    };
    let elem = clone_val(&bucket.val);
    execute_data.set_temp(value_slot, elem);

    if let Some(key_slot) = execute_data.fe_key_slot {
        let key_val = if let Some(ref key_zs) = bucket.key {
            Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(
                    key_zs.as_str(),
                    false,
                ))),
                PhpType::String,
            )
        } else {
            Val::new(PhpValue::Long(bucket.h as i64), PhpType::Long)
        };
        execute_data.set_temp(key_slot as usize, key_val);
    }

    let next_idx = current_idx + 1;
    execute_data.set_temp(
        iter_slot,
        Val::new(PhpValue::Long(next_idx as i64), PhpType::Long),
    );

    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_add_array_element(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    if is_temp_ref(&op.op1) {
        if let PhpValue::Long(slot_idx) = op.op1.value {
            let arr_slot = slot_idx as usize;
            let value = resolve_operand(&op.op2, execute_data);
            let mut arr_zval = execute_data.get_temp(arr_slot);
            if let PhpValue::Array(ref mut arr) = arr_zval.value {
                if op.extended_value != 0 {
                    let key = resolve_operand(&op.result, execute_data);
                    let key_str = crate::engine::operators::zval_get_string(&key);
                    let key_zs =
                        Box::new(crate::engine::string::string_init(key_str.as_str(), false));
                    let _ =
                        crate::engine::hash::hash_add_or_update(arr, Some(&*key_zs), 0, value, 0);
                } else {
                    let next_idx = arr.n_num_used as u64;
                    let _ = crate::engine::hash::hash_add_or_update(arr, None, next_idx, value, 0);
                }
            }
            execute_data.set_temp(arr_slot, arr_zval);
        }
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_fetch_dim(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let arr_val = resolve_operand(&op.op1, execute_data);
    let idx_val = resolve_operand(&op.op2, execute_data);
    let result_val = if let PhpValue::Array(ref arr) = arr_val.value {
        match &idx_val.value {
            PhpValue::Long(i) => crate::engine::hash::hash_index_find(arr, *i as u64)
                .map(|v| clone_val(v))
                .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null)),
            PhpValue::String(s) => crate::engine::hash::hash_find(arr, s)
                .map(|v| clone_val(v))
                .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null)),
            _ => Val::new(PhpValue::Long(0), PhpType::Null),
        }
    } else {
        Val::new(PhpValue::Long(0), PhpType::Null)
    };
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result_val);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_new_obj(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let class_name_val = resolve_operand(&op.op1, execute_data);
    let class_name = crate::engine::operators::zval_get_string(&class_name_val);
    let cn = class_name.as_str();

    let mut obj = crate::engine::types::PhpObject::new(cn);
    if let Some(ce) = execute_data.class_table.get(cn) {
        for (prop_name, prop_val) in &ce.default_properties {
            obj.properties
                .insert(prop_name.clone(), clone_val(prop_val));
        }
    }

    let obj_zval = Val::new(PhpValue::Object(Box::new(obj)), PhpType::Object);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, clone_val(&obj_zval));
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_fetch_obj_prop(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    let obj_val = resolve_operand(&op.op1, execute_data);
    let prop_name_val = resolve_operand(&op.op2, execute_data);
    let prop_name = crate::engine::operators::zval_get_string(&prop_name_val);

    let result_val = if let PhpValue::Object(ref obj) = obj_val.value {
        if let Some(v) = obj.properties.get(prop_name.as_str()) {
            clone_val(v)
        } else {
            let class_name = obj.class_name.clone();

            // Magic method: __isset for undefined properties
            let isset_allowed = execute_data
                .class_table
                .get(&class_name)
                .and_then(|ce| ce.methods.get("__isset"))
                .map(|m| {
                    let params = m.params.clone();
                    let ops: Vec<Op> = m
                        .op_array
                        .ops
                        .iter()
                        .map(|op| {
                            Op::new(
                                op.opcode,
                                clone_val(&op.op1),
                                clone_val(&op.op2),
                                clone_val(&op.result),
                                op.extended_value,
                            )
                        })
                        .collect();
                    let file_label = m
                        .op_array
                        .filename
                        .clone()
                        .filter(|f| !f.is_empty())
                        .unwrap_or_else(|| format!("{}::__isset", class_name));
                    (params, ops, file_label)
                });

            if let Some((params, ops, oparray_filename)) = isset_allowed {
                let saved_current_op = execute_data.current_op;
                let saved_op_array = execute_data.op_array.take();
                let saved_script_dir = execute_data.current_script_dir.clone();
                let saved_magic_dir = execute_data.constants.get("__DIR__").map(clone_val);
                let saved_magic_file = execute_data.constants.get("__FILE__").map(clone_val);
                let saved_called_class = execute_data.called_class.clone();
                execute_data.called_class = Some(class_name.clone());
                execute_data.set_var("this", clone_val(&obj_val));

                let name_val = Val::new(
                    PhpValue::String(Box::new(crate::engine::string::string_init(prop_name.as_str(), false))),
                    PhpType::String,
                );
                bind_call_args(execute_data, &params, &[name_val], &[None], &None);

                let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
                method_op_array.ops = ops;
                let (_status, isset_result) =
                    super::execute::execute_ex_returning(execute_data, &method_op_array);
                execute_data.op_array = saved_op_array;
                execute_data.current_op = saved_current_op;
                execute_data.current_script_dir = saved_script_dir;
                execute_data.called_class = saved_called_class;
                match saved_magic_dir {
                    Some(v) => { execute_data.constants.insert("__DIR__".to_string(), v); }
                    None => { execute_data.constants.remove("__DIR__"); }
                }
                match saved_magic_file {
                    Some(v) => { execute_data.constants.insert("__FILE__".to_string(), v); }
                    None => { execute_data.constants.remove("__FILE__"); }
                }

                let is_true = isset_result.map(|v| {
                    match v.get_type() {
                        PhpType::True | PhpType::Object | PhpType::Array => true,
                        PhpType::String => {
                            let s = crate::engine::operators::zval_get_string(&v);
                            !s.as_str().is_empty() && s.as_str() != "0"
                        }
                        PhpType::Long => crate::engine::operators::zval_get_long(&v) != 0,
                        PhpType::Double => crate::engine::operators::zval_get_double(&v) != 0.0,
                        _ => false,
                    }
                }).unwrap_or(false);

                if !is_true {
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
                    }
                    return Ok(ExecResult::Continue);
                }
            }

            let magic_info = execute_data
                .class_table
                .get(&class_name)
                .and_then(|ce| ce.methods.get("__get"))
                .map(|m| {
                    let params = m.params.clone();
                    let ops: Vec<Op> = m
                        .op_array
                        .ops
                        .iter()
                        .map(|op| {
                            Op::new(
                                op.opcode,
                                clone_val(&op.op1),
                                clone_val(&op.op2),
                                clone_val(&op.result),
                                op.extended_value,
                            )
                        })
                        .collect();
                    let file_label = m
                        .op_array
                        .filename
                        .clone()
                        .filter(|f| !f.is_empty())
                        .unwrap_or_else(|| format!("{}::__get", class_name));
                    (params, ops, file_label)
                });
            if let Some((params, ops, oparray_filename)) = magic_info {
                let saved_current_op = execute_data.current_op;
                let saved_op_array = execute_data.op_array.take();
                let saved_script_dir = execute_data.current_script_dir.clone();
                let saved_magic_dir = execute_data.constants.get("__DIR__").map(clone_val);
                let saved_magic_file = execute_data.constants.get("__FILE__").map(clone_val);
                let saved_called_class = execute_data.called_class.clone();
                execute_data.called_class = Some(class_name.clone());
                execute_data.set_var("this", clone_val(&obj_val));

                let name_val = Val::new(
                    PhpValue::String(Box::new(crate::engine::string::string_init(prop_name.as_str(), false))),
                    PhpType::String,
                );
                bind_call_args(execute_data, &params, &[name_val], &[None], &None);

                let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
                method_op_array.ops = ops;
                let (_status, return_val) =
                    super::execute::execute_ex_returning(execute_data, &method_op_array);
                execute_data.op_array = saved_op_array;
                execute_data.current_op = saved_current_op;
                execute_data.current_script_dir = saved_script_dir;
                execute_data.called_class = saved_called_class;
                match saved_magic_dir {
                    Some(v) => { execute_data.constants.insert("__DIR__".to_string(), v); }
                    None => { execute_data.constants.remove("__DIR__"); }
                }
                match saved_magic_file {
                    Some(v) => { execute_data.constants.insert("__FILE__".to_string(), v); }
                    None => { execute_data.constants.remove("__FILE__"); }
                }

                return_val.unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null))
            } else {
                Val::new(PhpValue::Long(0), PhpType::Null)
            }
        }
    } else {
        Val::new(PhpValue::Long(0), PhpType::Null)
    };

    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result_val);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_assign_obj_prop(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    let var_name_val = &op.op1;
    let prop_name_val = resolve_operand(&op.op2, execute_data);
    let prop_name = crate::engine::operators::zval_get_string(&prop_name_val);
    let value = resolve_operand(&op.result, execute_data);

    // Determine if __set should be invoked
    let (class_name_opt, has_prop) = {
        let obj_val = if is_var_ref(var_name_val) {
            if let PhpValue::String(ref s) = var_name_val.value {
                let vname = s.as_str();
                let name = if vname.starts_with('$') { &vname[1..] } else { vname };
                execute_data.get_var(name)
            } else {
                Val::new(PhpValue::Long(0), PhpType::Null)
            }
        } else if is_temp_ref(var_name_val) {
            if let PhpValue::Long(slot_idx) = var_name_val.value {
                let slot = slot_idx as usize;
                execute_data.get_temp(slot)
            } else {
                Val::new(PhpValue::Long(0), PhpType::Null)
            }
        } else {
            Val::new(PhpValue::Long(0), PhpType::Null)
        };
        if let PhpValue::Object(ref obj) = obj_val.value {
            (Some(obj.class_name.clone()), obj.properties.contains_key(prop_name.as_str()))
        } else {
            (None, false)
        }
    };

    let use_magic = if let Some(ref class_name) = class_name_opt {
        execute_data.class_table.get(class_name)
            .map(|ce| ce.methods.contains_key("__set"))
            .unwrap_or(false) && !has_prop
    } else {
        false
    };

    if use_magic {
        if let Some(ref class_name) = class_name_opt {
            let magic_info = execute_data
                .class_table
                .get(class_name)
                .and_then(|ce| ce.methods.get("__set"))
                .map(|m| {
                    let params = m.params.clone();
                    let ops: Vec<Op> = m
                        .op_array
                        .ops
                        .iter()
                        .map(|op| {
                            Op::new(
                                op.opcode,
                                clone_val(&op.op1),
                                clone_val(&op.op2),
                                clone_val(&op.result),
                                op.extended_value,
                            )
                        })
                        .collect();
                    let file_label = m
                        .op_array
                        .filename
                        .clone()
                        .filter(|f| !f.is_empty())
                        .unwrap_or_else(|| format!("{}::__set", class_name));
                    (params, ops, file_label)
                });
            if let Some((params, ops, oparray_filename)) = magic_info {
                let obj_val = if is_var_ref(var_name_val) {
                    if let PhpValue::String(ref s) = var_name_val.value {
                        let vname = s.as_str();
                        let name = if vname.starts_with('$') { &vname[1..] } else { vname };
                        execute_data.get_var(name)
                    } else {
                        Val::new(PhpValue::Long(0), PhpType::Null)
                    }
                } else if is_temp_ref(var_name_val) {
                    if let PhpValue::Long(slot_idx) = var_name_val.value {
                        let slot = slot_idx as usize;
                        execute_data.get_temp(slot)
                    } else {
                        Val::new(PhpValue::Long(0), PhpType::Null)
                    }
                } else {
                    Val::new(PhpValue::Long(0), PhpType::Null)
                };
                let saved_current_op = execute_data.current_op;
                let saved_op_array = execute_data.op_array.take();
                let saved_script_dir = execute_data.current_script_dir.clone();
                let saved_magic_dir = execute_data.constants.get("__DIR__").map(clone_val);
                let saved_magic_file = execute_data.constants.get("__FILE__").map(clone_val);
                let saved_called_class = execute_data.called_class.clone();
                execute_data.called_class = Some(class_name.clone());
                execute_data.set_var("this", clone_val(&obj_val));

                let name_val = Val::new(
                    PhpValue::String(Box::new(crate::engine::string::string_init(prop_name.as_str(), false))),
                    PhpType::String,
                );
                bind_call_args(execute_data, &params, &[name_val, clone_val(&value)], &[None, None], &None);

                let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
                method_op_array.ops = ops;
                let (_status, _return_val) =
                    super::execute::execute_ex_returning(execute_data, &method_op_array);
                execute_data.op_array = saved_op_array;
                execute_data.current_op = saved_current_op;
                execute_data.current_script_dir = saved_script_dir;
                execute_data.called_class = saved_called_class;
                match saved_magic_dir {
                    Some(v) => { execute_data.constants.insert("__DIR__".to_string(), v); }
                    None => { execute_data.constants.remove("__DIR__"); }
                }
                match saved_magic_file {
                    Some(v) => { execute_data.constants.insert("__FILE__".to_string(), v); }
                    None => { execute_data.constants.remove("__FILE__"); }
                }
                return Ok(ExecResult::Continue);
            }
        }
    }

    // Normal property assignment
    if is_var_ref(var_name_val) {
        if let PhpValue::String(ref s) = var_name_val.value {
            let vname = s.as_str();
            let name = if vname.starts_with('$') {
                &vname[1..]
            } else {
                vname
            };
            let mut obj_val = execute_data.get_var(name);
            if let PhpValue::Object(ref mut obj) = obj_val.value {
                obj.properties.insert(prop_name.as_str().to_string(), value);
            }
            execute_data.set_var(name, obj_val);
        }
    } else if is_temp_ref(var_name_val) {
        if let PhpValue::Long(slot_idx) = var_name_val.value {
            let slot = slot_idx as usize;
            let mut obj_val = execute_data.get_temp(slot);
            if let PhpValue::Object(ref mut obj) = obj_val.value {
                obj.properties.insert(prop_name.as_str().to_string(), value);
            }
            execute_data.set_temp(slot, obj_val);
        }
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_assign_static_prop(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    let class_name_val = resolve_operand(&op.op1, execute_data);
    let mut class_name = crate::engine::operators::zval_get_string(&class_name_val).as_str().to_string();
    if class_name == "static" || class_name == "self" {
        class_name = execute_data.called_class.clone().unwrap_or_default();
    }
    let prop_name_val = resolve_operand(&op.op2, execute_data);
    let prop_name_raw = crate::engine::operators::zval_get_string(&prop_name_val);
    let prop_name_str = prop_name_raw.as_str();
    let prop_name = if prop_name_str.starts_with('$') { &prop_name_str[1..] } else { prop_name_str };
    let value = resolve_operand(&op.result, execute_data);

    if let Some(ce) = execute_data.class_table.get_mut(&class_name) {
        ce.static_properties.insert(prop_name.to_string(), value);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_init_method_call(
    _op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    execute_data.call_arg_stack.push((
        execute_data.call_args.len(),
        execute_data.call_arg_names.len(),
    ));
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_do_method_call(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    let method_name_val = resolve_operand(&op.op1, execute_data);
    let method_name = crate::engine::operators::zval_get_string(&method_name_val);
    let obj_val = resolve_operand(&op.op2, execute_data);

    if let PhpValue::Object(ref obj) = obj_val.value {
        let class_name = obj.class_name.clone();

        // Handle built-in reflection classes
        if class_name == "ReflectionClass" || class_name == "ReflectionMethod" || class_name == "ReflectionProperty" {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            execute_data.called_class = Some(class_name.clone());
            execute_data.set_var("this", clone_val(&obj_val));

            let ret = match class_name.as_str() {
                "ReflectionClass" => crate::engine::vm::reflection::execute_reflection_class_method(method_name.as_str(), &args, execute_data),
                "ReflectionMethod" => crate::engine::vm::reflection::execute_reflection_method(method_name.as_str(), &args, execute_data),
                "ReflectionProperty" => crate::engine::vm::reflection::execute_reflection_property(method_name.as_str(), &args, execute_data),
                _ => None,
            };

            // Copy modified $this back
            let this_val = execute_data.get_var("this");
            if is_temp_ref(&op.op2) {
                if let PhpValue::Long(slot_idx) = op.op2.value {
                    execute_data.set_temp(slot_idx as usize, this_val);
                }
            } else if is_var_ref(&op.op2) {
                if let PhpValue::String(ref s) = op.op2.value {
                    let vname = s.as_str();
                    let name = if vname.starts_with('$') { &vname[1..] } else { vname };
                    execute_data.set_var(name, this_val);
                }
            }

            if let Some(slot) = result_slot(op) {
                if let Some(val) = ret {
                    execute_data.set_temp(slot, val);
                } else {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
                }
            }
            return Ok(ExecResult::Continue);
        }

        // Extract method info (owned copies to avoid borrow conflict)
        let method_info: Option<(Vec<String>, Vec<Op>, String)> = execute_data
            .class_table
            .get(&class_name)
            .and_then(|ce| ce.methods.get(method_name.as_str()))
            .map(|m| {
                let params = m.params.clone();
                let ops: Vec<Op> = m
                    .op_array
                    .ops
                    .iter()
                    .map(|op| {
                        Op::new(
                            op.opcode,
                            clone_val(&op.op1),
                            clone_val(&op.op2),
                            clone_val(&op.result),
                            op.extended_value,
                        )
                    })
                    .collect();
                let file_label = m
                    .op_array
                    .filename
                    .clone()
                    .filter(|f| !f.is_empty())
                    .unwrap_or_else(|| format!("{}::{}", class_name, method_name.as_str()));
                (params, ops, file_label)
            });

        if let Some((params, ops, oparray_filename)) = method_info {
            let saved_current_op = execute_data.current_op;
            let saved_op_array = execute_data.op_array.take();
            let saved_script_dir = execute_data.current_script_dir.clone();
            let saved_magic_dir = execute_data.constants.get("__DIR__").map(clone_val);
            let saved_magic_file = execute_data.constants.get("__FILE__").map(clone_val);
            let saved_called_class = execute_data.called_class.clone();
            execute_data.called_class = Some(class_name.clone());
            // Set up $this
            execute_data.set_var("this", clone_val(&obj_val));

            // Set up method parameters (supports named args and variadic)
            let (base, names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let arg_names: Vec<Option<String>> = execute_data.call_arg_names.drain(names_base..).collect();
            let variadic = execute_data
                .class_table
                .get(&class_name)
                .and_then(|ce| ce.methods.get(method_name.as_str()))
                .and_then(|m| m.op_array.variadic_param.clone());
            bind_call_args(execute_data, &params, &args, &arg_names, &variadic);

            // Execute method
            let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
            method_op_array.ops = ops;
            let (_status, return_val) =
                super::execute::execute_ex_returning(execute_data, &method_op_array);
            execute_data.op_array = saved_op_array;
            execute_data.current_op = saved_current_op;
            execute_data.current_script_dir = saved_script_dir;
            execute_data.called_class = saved_called_class;
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

            // Copy modified $this back to the original object location (objects are reference-like in PHP)
            let this_val = execute_data.get_var("this");
            if let PhpValue::Object(_) = this_val.value {
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value {
                        execute_data.set_temp(slot_idx as usize, this_val);
                    }
                } else if is_var_ref(&op.op2) {
                    if let PhpValue::String(ref s) = op.op2.value {
                        let vname = s.as_str();
                        let name = if vname.starts_with('$') { &vname[1..] } else { vname };
                        execute_data.set_var(name, this_val);
                    }
                }
            }

            // Store return value
            if let Some(slot) = result_slot(op) {
                if let Some(ret) = return_val {
                    execute_data.set_temp(slot, ret);
                } else {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
                }
            }
            return Ok(ExecResult::Continue);
        }

        // Method not found — try __call magic method
        let (base, names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
        let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
        let _arg_names: Vec<Option<String>> = execute_data.call_arg_names.drain(names_base..).collect();

        let magic_info = execute_data
            .class_table
            .get(&class_name)
            .and_then(|ce| ce.methods.get("__call"))
            .map(|m| {
                let params = m.params.clone();
                let ops: Vec<Op> = m
                    .op_array
                    .ops
                    .iter()
                    .map(|op| {
                        Op::new(
                            op.opcode,
                            clone_val(&op.op1),
                            clone_val(&op.op2),
                            clone_val(&op.result),
                            op.extended_value,
                        )
                    })
                    .collect();
                let file_label = m
                    .op_array
                    .filename
                    .clone()
                    .filter(|f| !f.is_empty())
                    .unwrap_or_else(|| format!("{}::__call", class_name));
                (params, ops, file_label)
            });

        if let Some((params, ops, oparray_filename)) = magic_info {
            let saved_current_op = execute_data.current_op;
            let saved_op_array = execute_data.op_array.take();
            let saved_script_dir = execute_data.current_script_dir.clone();
            let saved_magic_dir = execute_data.constants.get("__DIR__").map(clone_val);
            let saved_magic_file = execute_data.constants.get("__FILE__").map(clone_val);
            let saved_called_class = execute_data.called_class.clone();
            execute_data.called_class = Some(class_name.clone());
            execute_data.set_var("this", clone_val(&obj_val));

            let name_val = Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(method_name.as_str(), false))),
                PhpType::String,
            );
            let mut arr = crate::engine::types::PhpArray::new();
            for (i, arg) in args.iter().enumerate() {
                let bucket = crate::engine::types::Bucket {
                    val: clone_val(arg),
                    h: i as u64,
                    key: None,
                };
                arr.ar_data.push(bucket);
                arr.n_num_used += 1;
                arr.n_num_of_elements += 1;
            }
            arr.n_next_free_element = args.len() as i64;
            let args_val = Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array);
            bind_call_args(execute_data, &params, &[name_val, args_val], &[None, None], &None);

            let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
            method_op_array.ops = ops;
            let (_status, return_val) =
                super::execute::execute_ex_returning(execute_data, &method_op_array);
            execute_data.op_array = saved_op_array;
            execute_data.current_op = saved_current_op;
            execute_data.current_script_dir = saved_script_dir;
            execute_data.called_class = saved_called_class;
            match saved_magic_dir {
                Some(v) => { execute_data.constants.insert("__DIR__".to_string(), v); }
                None => { execute_data.constants.remove("__DIR__"); }
            }
            match saved_magic_file {
                Some(v) => { execute_data.constants.insert("__FILE__".to_string(), v); }
                None => { execute_data.constants.remove("__FILE__"); }
            }

            if let Some(slot) = result_slot(op) {
                if let Some(ret) = return_val {
                    execute_data.set_temp(slot, ret);
                } else {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
                }
            }
            return Ok(ExecResult::Continue);
        }
    }

    let _ = execute_data.call_arg_stack.pop();
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_fetch_static_prop(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    let class_name_val = resolve_operand(&op.op1, execute_data);
    let mut class_name = crate::engine::operators::zval_get_string(&class_name_val).as_str().to_string();

    if class_name == "static" || class_name == "self" {
        class_name = execute_data.called_class.clone().unwrap_or_default();
    }

    let prop_name_val = resolve_operand(&op.op2, execute_data);
    let prop_name_raw = crate::engine::operators::zval_get_string(&prop_name_val);
    let prop_name_str = prop_name_raw.as_str();
    let prop_name = if prop_name_str.starts_with('$') { &prop_name_str[1..] } else { prop_name_str };

    let result_val = if let Some(ce) = execute_data.class_table.get(&class_name) {
        if prop_name == "class" {
            Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&class_name, false))), PhpType::String)
        } else {
            ce.static_properties
                .get(prop_name)
                .map(|v| clone_val(v))
                .or_else(|| ce.constants.get(prop_name).map(|v| clone_val(v)))
                .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null))
        }
    } else {
        Val::new(PhpValue::Long(0), PhpType::Null)
    };

    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result_val);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_do_static_call(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    let method_name_val = resolve_operand(&op.op1, execute_data);
    let method_name = crate::engine::operators::zval_get_string(&method_name_val);

    let class_name_val = resolve_operand(&op.op2, execute_data);
    let mut class_name = crate::engine::operators::zval_get_string(&class_name_val).as_str().to_string();

    if class_name == "static" {
        class_name = execute_data.called_class.clone().unwrap_or_default();
    }

    let resolved_class = class_name.clone();

    let method_info: Option<(Vec<String>, Vec<Op>, String)> = execute_data
        .class_table
        .get(&resolved_class)
        .and_then(|ce| ce.methods.get(method_name.as_str()))
        .map(|m| {
            let params = m.params.clone();
            let ops: Vec<Op> = m
                .op_array
                .ops
                .iter()
                .map(|op| {
                    Op::new(
                        op.opcode,
                        clone_val(&op.op1),
                        clone_val(&op.op2),
                        clone_val(&op.result),
                        op.extended_value,
                    )
                })
                .collect();
            let file_label = m
                .op_array
                .filename
                .clone()
                .filter(|f| !f.is_empty())
                .unwrap_or_else(|| format!("{}::{}", resolved_class, method_name.as_str()));
            (params, ops, file_label)
        });

    if let Some((params, ops, oparray_filename)) = method_info {
        let saved_current_op = execute_data.current_op;
        let saved_op_array = execute_data.op_array.take();
        let saved_script_dir = execute_data.current_script_dir.clone();
        let saved_magic_dir = execute_data.constants.get("__DIR__").map(clone_val);
        let saved_magic_file = execute_data.constants.get("__FILE__").map(clone_val);
        let saved_called_class = execute_data.called_class.clone();
        execute_data.called_class = Some(resolved_class.clone());

        let (base, names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
        let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
        let arg_names: Vec<Option<String>> = execute_data.call_arg_names.drain(names_base..).collect();
        let variadic = execute_data
            .class_table
            .get(&resolved_class)
            .and_then(|ce| ce.methods.get(method_name.as_str()))
            .and_then(|m| m.op_array.variadic_param.clone());
        bind_call_args(execute_data, &params, &args, &arg_names, &variadic);

        let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
        method_op_array.ops = ops;
        let (_status, return_val) =
            super::execute::execute_ex_returning(execute_data, &method_op_array);
        execute_data.op_array = saved_op_array;
        execute_data.current_op = saved_current_op;
        execute_data.current_script_dir = saved_script_dir;
        execute_data.called_class = saved_called_class;
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

        if let Some(slot) = result_slot(op) {
            if let Some(ret) = return_val {
                execute_data.set_temp(slot, ret);
            } else {
                execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
            }
        }
        return Ok(ExecResult::Continue);
    }

    // Magic method: __callStatic
    if let Some(ce) = execute_data.class_table.get(&resolved_class) {
        if let Some(magic) = ce.methods.get("__callStatic") {
            let saved_current_op = execute_data.current_op;
            let saved_op_array = execute_data.op_array.take();
            let saved_script_dir = execute_data.current_script_dir.clone();
            let saved_magic_dir = execute_data.constants.get("__DIR__").map(clone_val);
            let saved_magic_file = execute_data.constants.get("__FILE__").map(clone_val);
            let saved_called_class = execute_data.called_class.clone();
            execute_data.called_class = Some(resolved_class.clone());

            let params = magic.params.clone();
            let ops: Vec<Op> = magic
                .op_array
                .ops
                .iter()
                .map(|op| {
                    Op::new(
                        op.opcode,
                        clone_val(&op.op1),
                        clone_val(&op.op2),
                        clone_val(&op.result),
                        op.extended_value,
                    )
                })
                .collect();
            let file_label = magic
                .op_array
                .filename
                .clone()
                .filter(|f| !f.is_empty())
                .unwrap_or_else(|| format!("{}::__callStatic", resolved_class));

            let (base, names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let _ = execute_data.call_args.drain(base..);
            let _ = execute_data.call_arg_names.drain(names_base..);
            if let Some(p0) = params.get(0) {
                execute_data.set_var(p0, Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(method_name.as_str(), false))), PhpType::String));
            }
            if let Some(p1) = params.get(1) {
                let arr = crate::engine::types::PhpArray::new();
                let arr_val = Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array);
                execute_data.set_var(p1, arr_val);
            }

            let mut method_op_array = OpArray::with_capacity(ops.len(), file_label);
            method_op_array.ops = ops;
            let (_status, return_val) =
                super::execute::execute_ex_returning(execute_data, &method_op_array);
            execute_data.op_array = saved_op_array;
            execute_data.current_op = saved_current_op;
            execute_data.current_script_dir = saved_script_dir;
            execute_data.called_class = saved_called_class;
            match saved_magic_dir {
                Some(v) => { execute_data.constants.insert("__DIR__".to_string(), v); }
                None => { execute_data.constants.remove("__DIR__"); }
            }
            match saved_magic_file {
                Some(v) => { execute_data.constants.insert("__FILE__".to_string(), v); }
                None => { execute_data.constants.remove("__FILE__"); }
            }

            if let Some(slot) = result_slot(op) {
                if let Some(ret) = return_val {
                    execute_data.set_temp(slot, ret);
                } else {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
                }
            }
            return Ok(ExecResult::Continue);
        }
    }

    let _ = execute_data.call_arg_stack.pop();
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_clone_obj(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    let obj_val = resolve_operand(&op.op1, execute_data);

    let cloned = if let PhpValue::Object(ref obj) = obj_val.value {
        let mut new_obj = crate::engine::types::PhpObject::new(&obj.class_name);
        for (k, v) in &obj.properties {
            new_obj.properties.insert(k.clone(), clone_val(v));
        }
        new_obj.handle = obj.handle;
        Val::new(PhpValue::Object(Box::new(new_obj)), PhpType::Object)
    } else {
        Val::new(PhpValue::Long(0), PhpType::Null)
    };

    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, cloned);
    }
    Ok(ExecResult::Continue)
}

/// Bind call arguments to parameters, supporting named args and variadic params
#[inline]
fn bind_call_args(
    execute_data: &mut ExecuteData,
    param_names: &[String],
    args: &[Val],
    arg_names: &[Option<String>],
    variadic_param: &Option<String>,
) {
    let regular_count = if variadic_param.is_some() {
        param_names.len().saturating_sub(1)
    } else {
        param_names.len()
    };

    let mut bound = vec![false; regular_count];

    // First pass: bind named arguments
    for (i, name_opt) in arg_names.iter().enumerate() {
        if let Some(name) = name_opt {
            if let Some(pos) = param_names[..regular_count].iter().position(|p| {
                let clean = if p.starts_with('$') { &p[1..] } else { p.as_str() };
                clean == name.as_str()
            }) {
                if let Some(arg) = args.get(i) {
                    let p = &param_names[pos];
                    let clean = if p.starts_with('$') { &p[1..] } else { p.as_str() };
                    execute_data.set_var(clean, clone_val(arg));
                    bound[pos] = true;
                }
            }
        }
    }

    // Second pass: bind positional arguments to remaining params
    let mut param_idx = 0;
    for (i, name_opt) in arg_names.iter().enumerate() {
        if name_opt.is_none() {
            // Skip already-bound params
            while param_idx < regular_count && bound[param_idx] {
                param_idx += 1;
            }
            if param_idx < regular_count {
                if let Some(arg) = args.get(i) {
                    let p = &param_names[param_idx];
                    let clean = if p.starts_with('$') { &p[1..] } else { p.as_str() };
                    execute_data.set_var(clean, clone_val(arg));
                    bound[param_idx] = true;
                    param_idx += 1;
                }
            }
        }
    }

    // Pack extra arguments into variadic array
    if let Some(var_name) = variadic_param {
        let mut arr = crate::engine::types::PhpArray::new();
        let mut idx: u64 = 0;
        for (i, arg) in args.iter().enumerate() {
            let is_extra = if let Some(ref name) = arg_names[i] {
                // Named arg is extra if not matched to a regular param
                param_names[..regular_count].iter().position(|p| {
                    let clean = if p.starts_with('$') { &p[1..] } else { p.as_str() };
                    clean == name.as_str()
                }).is_none()
            } else {
                // Positional arg is extra if beyond regular_count
                let pos = arg_names[..i].iter().filter(|n| n.is_none()).count();
                pos >= regular_count
            };
            if is_extra {
                let _ = crate::engine::hash::hash_add_or_update(
                    &mut arr,
                    None,
                    idx,
                    clone_val(arg),
                    0,
                );
                idx += 1;
            }
        }
        let arr_val = Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array);
        let clean = if var_name.starts_with('$') {
            &var_name[1..]
        } else {
            var_name.as_str()
        };
        execute_data.set_var(clean, arr_val);
    }
}

/// Generic opcode dispatch function for JIT compilation
pub fn dispatch_opcode(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    match op.opcode {
        Opcode::Nop => execute_nop(op, execute_data),
        Opcode::Add => execute_add(op, execute_data),
        Opcode::Sub => execute_sub(op, execute_data),
        Opcode::Mul => execute_mul(op, execute_data),
        Opcode::Div => execute_div(op, execute_data),
        Opcode::Mod => execute_mod(op, execute_data),
        Opcode::Pow => execute_pow(op, execute_data),
        Opcode::BoolNot => execute_bool_not(op, execute_data),
        Opcode::BoolAnd => execute_bool_and(op, execute_data),
        Opcode::BoolOr => execute_bool_or(op, execute_data),
        Opcode::BoolXor => execute_bool_xor(op, execute_data),
        Opcode::Concat => execute_concat(op, execute_data),
        Opcode::Assign => execute_assign(op, execute_data),
        Opcode::AssignDim => execute_assign_dim(op, execute_data),
        Opcode::Echo => execute_echo(op, execute_data),
        Opcode::Return => execute_return(op, execute_data),
        Opcode::Jmp => execute_jmp(op, execute_data),
        Opcode::JmpZ => execute_jmpz(op, execute_data),
        Opcode::JmpNZ => execute_jmpnz(op, execute_data),
        Opcode::InitFCall => execute_init_fcall(op, execute_data),
        Opcode::DoFCall => execute_do_fcall(op, execute_data),
        Opcode::FetchVar => execute_fetch_var(op, execute_data),
        Opcode::SendVal => execute_send_val(op, execute_data),
        Opcode::SendValNamed => execute_send_val_named(op, execute_data),
        Opcode::BindGlobal => execute_bind_global(op, execute_data),
        Opcode::Include => execute_include(op, execute_data),
        Opcode::InitArray => execute_init_array(op, execute_data),
        Opcode::AddArrayElement => execute_add_array_element(op, execute_data),
        Opcode::FetchDim => execute_fetch_dim(op, execute_data),
        Opcode::NewObj => execute_new_obj(op, execute_data),
        Opcode::FetchObjProp => execute_fetch_obj_prop(op, execute_data),
        Opcode::AssignObjProp => execute_assign_obj_prop(op, execute_data),
        Opcode::InitMethodCall => execute_init_method_call(op, execute_data),
        Opcode::DoMethodCall => execute_do_method_call(op, execute_data),
        Opcode::FetchStaticProp => execute_fetch_static_prop(op, execute_data),
        Opcode::DoStaticCall => execute_do_static_call(op, execute_data),
        Opcode::CloneObj => execute_clone_obj(op, execute_data),
        Opcode::Coalesce => execute_coalesce(op, execute_data),
        Opcode::QmAssign => execute_qm_assign(op, execute_data),
        Opcode::JmpNullZ => execute_jmp_null_z(op, execute_data),
        Opcode::IsIdentical => execute_is_identical(op, execute_data),
        Opcode::IsNotIdentical => execute_is_not_identical(op, execute_data),
        Opcode::IsEqual => execute_is_equal(op, execute_data),
        Opcode::IsNotEqual => execute_is_not_equal(op, execute_data),
        Opcode::IsSmaller => execute_is_smaller(op, execute_data),
        Opcode::IsSmallerOrEqual => execute_is_smaller_or_equal(op, execute_data),
        _ => Ok(ExecResult::Continue),
    }
}

fn make_bool(val: bool) -> Val {
    Val::new(
        PhpValue::Long(if val { 1 } else { 0 }),
        if val { PhpType::True } else { PhpType::False },
    )
}

#[inline]
pub fn execute_is_identical(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let result = op1.value == op2.value && op1.get_type() == op2.get_type();
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, make_bool(result));
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_is_not_identical(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let result = !(op1.value == op2.value && op1.get_type() == op2.get_type());
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, make_bool(result));
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_is_equal(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let result = crate::engine::operators::zval_is_equal(&op1, &op2);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, make_bool(result));
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_is_not_equal(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let result = !crate::engine::operators::zval_is_equal(&op1, &op2);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, make_bool(result));
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_is_smaller(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let result = crate::engine::operators::zval_compare(&op1, &op2) < 0;
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, make_bool(result));
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_is_smaller_or_equal(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let result = crate::engine::operators::zval_compare(&op1, &op2) <= 0;
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, make_bool(result));
    }
    Ok(ExecResult::Continue)
}
