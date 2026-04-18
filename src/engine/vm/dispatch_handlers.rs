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
    let s1 = crate::engine::operators::zval_get_string(&op1);
    let s2 = crate::engine::operators::zval_get_string(&op2);
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
    let key = resolve_operand(&op.result, execute_data);
    if let PhpValue::String(var_s) = &op.op1.value {
        let name = var_s.as_str();
        let clean = if name.starts_with('$') {
            &name[1..]
        } else {
            name
        };
        let mut container = execute_data.get_var(clean);
        if container.get_type() == PhpType::Null {
            container = Val::new(
                PhpValue::Array(Box::new(crate::engine::types::PhpArray::new())),
                PhpType::Array,
            );
        }
        if let PhpValue::Array(ref mut arr) = container.value {
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
            execute_data.set_var(clean, container);
        }
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_echo(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let s = crate::engine::operators::zval_get_string(&val);
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
    let func_name = if is_var_ref(&op.op1) || is_temp_ref(&op.op1) {
        let resolved = resolve_operand(&op.op1, execute_data);
        crate::engine::operators::zval_get_string(&resolved)
            .as_str()
            .to_string()
    } else {
        crate::engine::operators::zval_get_string(&op.op1)
            .as_str()
            .to_string()
    };

    // Check JIT compilation for hot functions
    increment_execution_counter(&func_name);

    let args: Vec<Val> = execute_data.call_args.drain(..).collect();

    match execute_builtin_function(&func_name, &args, execute_data)? {
        Some(result) => {
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, result);
            }
            Ok(ExecResult::Continue)
        }
        None => {
            // Try JIT compilation first
            let jit = crate::engine::jit::get_jit_compiler();
            let jit = jit.read().unwrap();
            let jit_result = jit.get_compiled_function(&func_name);
            if let Some(jit_result) = jit_result {
                execute_data.call_args.extend(args);
                let _result = {
                    let func = jit_result.clone();
                    func(execute_data)?
                };
                if let Some(return_val) = execute_data.call_args.pop() {
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, return_val);
                    }
                }
                return Ok(ExecResult::Continue);
            }

            // Try user-defined function table - optimized lookup
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

                // Set up fresh symbol table for function scope
                execute_data.symbol_table = Some(crate::engine::types::PhpArray::new());

                // Determine the number of regular (non-variadic) parameters
                let regular_count = if variadic_param.is_some() {
                    param_names.len().saturating_sub(1)
                } else {
                    param_names.len()
                };

                // Bind arguments to parameter names - optimized
                for (i, arg) in args.iter().enumerate() {
                    if i < regular_count {
                    if let Some(name) = param_names.get(i) {
                        if !name.is_empty() {
                            let clean = if name.starts_with('$') {
                                &name[1..]
                            } else {
                                name.as_str()
                            };
                            execute_data.set_var(clean, clone_val(arg));
                        }
                    }
                    }
                }

                // Pack extra arguments into an array for the variadic parameter
                if let Some(ref var_name) = variadic_param {
                    let mut arr = crate::engine::types::PhpArray::new();
                    let mut idx: u64 = 0;
                    for arg in args.iter().skip(regular_count) {
                        let _ = crate::engine::hash::hash_add_or_update(
                            &mut arr,
                            None,
                            idx,
                            clone_val(arg),
                            0,
                        );
                        idx += 1;
                    }
                    let arr_val = Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array);
                    let clean = if var_name.starts_with('$') {
                        &var_name[1..]
                    } else {
                        var_name.as_str()
                    };
                    execute_data.set_var(clean, arr_val);
                }

                let saved_script_dir = execute_data.current_script_dir.clone();
                let saved_magic_dir = execute_data.constants.get("__DIR__").map(clone_val);
                let saved_magic_file = execute_data.constants.get("__FILE__").map(clone_val);
                // Execute the function and capture return value
                let (_status, return_val) =
                    super::execute::execute_ex_returning(execute_data, &func_op_array);

                // Restore execution state
                execute_data.symbol_table = saved_symbol_table;
                execute_data.temp_vars = saved_temps;
                execute_data.op_array = saved_op_array;
                execute_data.current_op = saved_op;
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
    execute_data.call_args.clear();
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_send_val(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    execute_data.call_args.push(val);
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_include(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let path_val = resolve_operand(&op.op1, execute_data);
    let path = crate::engine::operators::zval_get_string(&path_val);
    let path_str = path.as_str();
    let resolved =
        if path_str.starts_with('/') || (path_str.len() >= 2 && path_str.get(1..2) == Some(":")) {
            path_str.to_string()
        } else if let Some(ref dir) = execute_data.current_script_dir {
            let mut p = std::path::PathBuf::from(dir);
            p.push(path_str);
            p.to_string_lossy().into_owned()
        } else {
            path_str.to_string()
        };

    let is_once = op.extended_value == 2 || op.extended_value == 3;
    if is_once && execute_data.included_files.contains(&resolved) {
        return Ok(ExecResult::Continue);
    }
    match crate::engine::compile::compile_file(&resolved) {
        Ok(included_op_array) => {
            execute_data.included_files.insert(resolved.clone());
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
        obj.properties
            .get(prop_name.as_str())
            .map(|v| clone_val(v))
            .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null))
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
pub fn execute_init_method_call(
    _op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    execute_data.call_args.clear();
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
            // Set up $this
            execute_data.set_var("this", clone_val(&obj_val));

            // Set up method parameters
            let args: Vec<Val> = execute_data
                .call_args
                .iter()
                .map(|a| clone_val(a))
                .collect();
            for (i, param_name) in params.iter().enumerate() {
                if let Some(arg) = args.get(i) {
                    execute_data.set_var(param_name, clone_val(arg));
                }
            }

            // Execute method
            let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
            method_op_array.ops = ops;
            let (_status, return_val) =
                super::execute::execute_ex_returning(execute_data, &method_op_array);
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

            // Store return value
            execute_data.call_args.clear();
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

    execute_data.call_args.clear();
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
    }
    Ok(ExecResult::Continue)
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
        Opcode::Include => execute_include(op, execute_data),
        Opcode::InitArray => execute_init_array(op, execute_data),
        Opcode::AddArrayElement => execute_add_array_element(op, execute_data),
        Opcode::FetchDim => execute_fetch_dim(op, execute_data),
        Opcode::NewObj => execute_new_obj(op, execute_data),
        Opcode::FetchObjProp => execute_fetch_obj_prop(op, execute_data),
        Opcode::AssignObjProp => execute_assign_obj_prop(op, execute_data),
        Opcode::InitMethodCall => execute_init_method_call(op, execute_data),
        Opcode::DoMethodCall => execute_do_method_call(op, execute_data),
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
