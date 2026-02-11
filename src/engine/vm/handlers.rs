//! Opcode execution handlers
//!
//! Each handler resolves operands, performs the operation, and stores the result.

use crate::engine::types::{PhpType, PhpValue, Val};
use crate::engine::facade::{bool_val, long_val};
use super::opcodes::{Op, OpArray, Opcode};
use super::execute_data::{
    clone_val, is_temp_ref, is_var_ref, resolve_operand, result_slot,
    ExecResult, ExecuteData,
};
use super::builtins::execute_builtin_function;

/// Extract variable name from a Val (for get_var/remove_var calls)
fn val_to_var_name(val: &Val) -> Option<String> {
    if let PhpValue::String(ref s) = val.value {
        let name = s.as_str();
        Some(if name.starts_with('$') { name[1..].to_string() } else { name.to_string() })
    } else {
        None
    }
}

/// Execute a single opcode
pub(crate) fn execute_opcode(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    match op.opcode {
        Opcode::Nop => Ok(ExecResult::Continue),

        // Variable fetch: load $var into temp slot
        Opcode::FetchVar => {
            let val = resolve_operand(&op.op1, execute_data);
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, val);
            }
            Ok(ExecResult::Continue)
        }

        // Arithmetic operations
        Opcode::Add => execute_binary_op(op, execute_data, |a, b| crate::engine::operators::zval_add(a, b)),
        Opcode::Sub => execute_binary_op(op, execute_data, |a, b| crate::engine::operators::zval_sub(a, b)),
        Opcode::Mul => execute_binary_op(op, execute_data, |a, b| crate::engine::operators::zval_mul(a, b)),
        Opcode::Div => {
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
        Opcode::Mod => {
            let op1 = resolve_operand(&op.op1, execute_data);
            let op2 = resolve_operand(&op.op2, execute_data);
            let v1 = crate::engine::operators::zval_get_long(&op1);
            let v2 = crate::engine::operators::zval_get_long(&op2);
            if v2 == 0 { return Err("Division by zero".into()); }
            let result = Val::new(PhpValue::Long(v1 % v2), PhpType::Long);
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, result);
            }
            Ok(ExecResult::Continue)
        }
        Opcode::Pow => {
            let op1 = resolve_operand(&op.op1, execute_data);
            let op2 = resolve_operand(&op.op2, execute_data);
            let v1 = crate::engine::operators::zval_get_double(&op1);
            let v2 = crate::engine::operators::zval_get_double(&op2);
            let result = Val::new(PhpValue::Double(v1.powf(v2)), PhpType::Double);
            if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
            Ok(ExecResult::Continue)
        }
        Opcode::Sl => execute_bitwise_op(op, execute_data, |a, b| a << b),
        Opcode::Sr => execute_bitwise_op(op, execute_data, |a, b| a >> b),

        // Assignment: op1 = variable name, op2 = value
        Opcode::Assign => {
            let val = resolve_operand(&op.op2, execute_data);
            if let PhpValue::String(var_name) = &op.op1.value {
                let name = var_name.as_str();
                let clean = if name.starts_with('$') { &name[1..] } else { name };
                execute_data.set_var(clean, clone_val(&val));
            }
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, val);
            }
            Ok(ExecResult::Continue)
        }

        // String concatenation
        Opcode::Concat => {
            let op1 = resolve_operand(&op.op1, execute_data);
            let op2 = resolve_operand(&op.op2, execute_data);
            let s1 = crate::engine::operators::zval_get_string(&op1);
            let s2 = crate::engine::operators::zval_get_string(&op2);
            let combined = format!("{}{}", s1.as_str(), s2.as_str());
            let result = Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(&combined, false))),
                PhpType::String,
            );
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, result);
            }
            Ok(ExecResult::Continue)
        }

        // I/O
        Opcode::Echo => {
            let val = resolve_operand(&op.op1, execute_data);
            let s = crate::engine::operators::zval_get_string(&val);
            let _ = crate::php::output::php_output_write(s.as_bytes());
            Ok(ExecResult::Continue)
        }
        Opcode::Return => {
            let val = resolve_operand(&op.op1, execute_data);
            Ok(ExecResult::Return(val))
        }

        // Control flow
        Opcode::Jmp => Ok(ExecResult::Jump(op.extended_value)),
        Opcode::JmpZ => {
            let val = resolve_operand(&op.op1, execute_data);
            let b = crate::engine::operators::zval_get_bool(&val);
            if !b { Ok(ExecResult::Jump(op.extended_value)) } else { Ok(ExecResult::Continue) }
        }
        Opcode::JmpNZ => {
            let val = resolve_operand(&op.op1, execute_data);
            let b = crate::engine::operators::zval_get_bool(&val);
            if b { Ok(ExecResult::Jump(op.extended_value)) } else { Ok(ExecResult::Continue) }
        }

        // Comparison operations
        Opcode::IsEqual => execute_cmp_op(op, execute_data, |c| c == 0),
        Opcode::IsNotEqual => execute_cmp_op(op, execute_data, |c| c != 0),
        Opcode::IsSmaller => execute_cmp_op(op, execute_data, |c| c < 0),
        Opcode::IsSmallerOrEqual => execute_cmp_op(op, execute_data, |c| c <= 0),
        Opcode::IsIdentical => {
            let op1 = resolve_operand(&op.op1, execute_data);
            let op2 = resolve_operand(&op.op2, execute_data);
            let same_type = op1.get_type() == op2.get_type();
            let eq = same_type && crate::engine::operators::zval_compare(&op1, &op2) == 0;
            let result = Val::new(PhpValue::Long(if eq { 1 } else { 0 }), if eq { PhpType::True } else { PhpType::False });
            if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
            Ok(ExecResult::Continue)
        }
        Opcode::IsNotIdentical => {
            let op1 = resolve_operand(&op.op1, execute_data);
            let op2 = resolve_operand(&op.op2, execute_data);
            let same_type = op1.get_type() == op2.get_type();
            let neq = !same_type || crate::engine::operators::zval_compare(&op1, &op2) != 0;
            let result = Val::new(PhpValue::Long(if neq { 1 } else { 0 }), if neq { PhpType::True } else { PhpType::False });
            if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
            Ok(ExecResult::Continue)
        }

        // Bitwise / boolean operations
        Opcode::BwOr => execute_bitwise_op(op, execute_data, |a, b| a | b),
        Opcode::BwAnd => execute_bitwise_op(op, execute_data, |a, b| a & b),
        Opcode::BwXor => execute_bitwise_op(op, execute_data, |a, b| a ^ b),
        Opcode::BoolNot => {
            let val = resolve_operand(&op.op1, execute_data);
            let b = !crate::engine::operators::zval_get_bool(&val);
            let result = Val::new(PhpValue::Long(if b { 1 } else { 0 }), if b { PhpType::True } else { PhpType::False });
            if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
            Ok(ExecResult::Continue)
        }
        Opcode::BoolXor => {
            let op1 = resolve_operand(&op.op1, execute_data);
            let op2 = resolve_operand(&op.op2, execute_data);
            let r = crate::engine::operators::zval_get_bool(&op1) ^ crate::engine::operators::zval_get_bool(&op2);
            let result = Val::new(PhpValue::Long(if r { 1 } else { 0 }), if r { PhpType::True } else { PhpType::False });
            if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
            Ok(ExecResult::Continue)
        }

        // Function call operations
        Opcode::InitFCall => {
            execute_data.call_args.clear();
            Ok(ExecResult::Continue)
        }
        Opcode::SendVal => {
            let val = resolve_operand(&op.op1, execute_data);
            execute_data.call_args.push(val);
            Ok(ExecResult::Continue)
        }
        Opcode::DoFCall => execute_do_fcall(op, execute_data),

        // Include/require
        Opcode::Include => execute_include(op, execute_data),

        // Array operations
        Opcode::InitArray => execute_init_array(op, execute_data),
        Opcode::AddArrayElement => execute_add_array_element(op, execute_data),
        Opcode::FetchDim => execute_fetch_dim(op, execute_data),

        // OOP operations
        Opcode::NewObj => execute_new_obj(op, execute_data),
        Opcode::FetchObjProp => execute_fetch_obj_prop(op, execute_data),
        Opcode::AssignObjProp => execute_assign_obj_prop(op, execute_data),
        Opcode::InitMethodCall => {
            execute_data.call_args.clear();
            Ok(ExecResult::Continue)
        }
        Opcode::DoMethodCall => execute_do_method_call(op, execute_data),

        // Variable type checking
        Opcode::TypeCheck => {
            let var = resolve_operand(&op.op1, execute_data);
            let result = Val::new(PhpValue::Long(var.get_type() as i64), PhpType::Long);
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, result);
            }
            Ok(ExecResult::Continue)
        }

        // IsSet/IsEmpty/Unset operations
        Opcode::IsSet => {
            let var = resolve_operand(&op.op1, execute_data);
            let is_set = if let Some(name) = val_to_var_name(&var) {
                let v = execute_data.get_var(&name);
                bool_val(v.get_type() != PhpType::Null && v.get_type() != PhpType::Undef)
            } else {
                bool_val(false)
            };
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, is_set);
            }
            Ok(ExecResult::Continue)
        }

        Opcode::Empty => {
            let var = resolve_operand(&op.op1, execute_data);
            let is_empty = if let Some(name) = val_to_var_name(&var) {
                let v = execute_data.get_var(&name);
                bool_val(v.get_type() == PhpType::Null || v.get_type() == PhpType::Undef)
            } else {
                bool_val(true)
            };
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, is_empty);
            }
            Ok(ExecResult::Continue)
        }

        Opcode::Unset => {
            let var = resolve_operand(&op.op1, execute_data);
            if let Some(name) = val_to_var_name(&var) {
                execute_data.remove_var(&name);
            }
            Ok(ExecResult::Continue)
        }

        // Array/Count operations
        Opcode::Count => {
            let arr = resolve_operand(&op.op1, execute_data);
            let count = match &arr.value {
                PhpValue::Array(a) => long_val(a.n_num_of_elements as i64),
                PhpValue::String(s) => long_val(s.len as i64),
                _ => long_val(0),
            };
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, count);
            }
            Ok(ExecResult::Continue)
        }

        Opcode::Keys => {
            let arr = resolve_operand(&op.op1, execute_data);
            match &arr.value {
                PhpValue::Array(a) => {
                    let mut result_arr = crate::engine::types::PhpArray::new();
                    for bucket in &a.ar_data {
                        if let Some(key) = &bucket.key {
                            let key_val = Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(key.as_str(), false))), PhpType::String);
                            let idx = result_arr.n_num_of_elements as u64;
                            let _ = crate::engine::hash::hash_add_or_update(&mut result_arr, None, idx, key_val, 0);
                        }
                    }
                    let result = Val::new(PhpValue::Array(Box::new(result_arr)), PhpType::Array);
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, result);
                    }
                    Ok(ExecResult::Continue)
                }
                _ => Err("Keys() requires an array".into()),
            }
        }

        Opcode::Values => {
            let arr = resolve_operand(&op.op1, execute_data);
            match &arr.value {
                PhpValue::Array(a) => {
                    let mut result_arr = crate::engine::types::PhpArray::new();
                    for bucket in &a.ar_data {
                        let cloned = clone_val(&bucket.val);
                        let idx = result_arr.n_num_of_elements as u64;
                        let _ = crate::engine::hash::hash_add_or_update(&mut result_arr, None, idx, cloned, 0);
                    }
                    let result = Val::new(PhpValue::Array(Box::new(result_arr)), PhpType::Array);
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, result);
                    }
                    Ok(ExecResult::Continue)
                }
                _ => Err("Values() requires an array".into()),
            }
        }

        Opcode::ArrayDiff => {
            // For simplicity, compute whether arrays are equal (can be extended later)
            let arr1 = resolve_operand(&op.op1, execute_data);
            let arr2 = resolve_operand(&op.op2, execute_data);
            let result = match (&arr1.value, &arr2.value) {
                (PhpValue::Array(a), PhpValue::Array(b)) => {
                    let is_equal = a.n_num_of_elements == b.n_num_of_elements;
                    bool_val(is_equal)
                }
                _ => bool_val(false),
            };
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, result);
            }
            Ok(ExecResult::Continue)
        }

        // Null coalescing: if op1 is not null, result=op1, else result=op2
        Opcode::Coalesce => {
            let left = resolve_operand(&op.op1, execute_data);
            let result = if left.get_type() != PhpType::Null && left.get_type() != PhpType::Undef {
                left
            } else {
                resolve_operand(&op.op2, execute_data)
            };
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, result);
            }
            Ok(ExecResult::Continue)
        }

        // Jump if null (for ?? short-circuit)
        Opcode::JmpNullZ => {
            let val = resolve_operand(&op.op1, execute_data);
            if val.get_type() == PhpType::Null || val.get_type() == PhpType::Undef {
                if let PhpValue::Long(target) = &op.op2.value {
                    return Ok(ExecResult::Jump(*target as u32));
                }
            } else {
                // Not null — store value in result temp
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, val);
                }
            }
            Ok(ExecResult::Continue)
        }

        // Ternary assign: resolve op1, store in result temp slot
        Opcode::QmAssign => {
            let val = resolve_operand(&op.op1, execute_data);
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, val);
            }
            Ok(ExecResult::Continue)
        }

        _ => Ok(ExecResult::Continue),
    }
}

// --- Helper functions for common handler patterns ---

fn execute_binary_op<F>(op: &Op, execute_data: &mut ExecuteData, operation: F) -> Result<ExecResult, String>
where
    F: FnOnce(&Val, &Val) -> Val,
{
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let result = operation(&op1, &op2);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

fn execute_cmp_op<F>(op: &Op, execute_data: &mut ExecuteData, predicate: F) -> Result<ExecResult, String>
where
    F: FnOnce(i32) -> bool,
{
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let cmp = crate::engine::operators::zval_compare(&op1, &op2);
    let b = predicate(cmp);
    let result = Val::new(PhpValue::Long(if b { 1 } else { 0 }), if b { PhpType::True } else { PhpType::False });
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

fn execute_bitwise_op<F>(op: &Op, execute_data: &mut ExecuteData, operation: F) -> Result<ExecResult, String>
where
    F: FnOnce(i64, i64) -> i64,
{
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let v1 = crate::engine::operators::zval_get_long(&op1);
    let v2 = crate::engine::operators::zval_get_long(&op2);
    let result = Val::new(PhpValue::Long(operation(v1, v2)), PhpType::Long);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

// --- Complex handler functions ---

fn execute_include(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let path_val = resolve_operand(&op.op1, execute_data);
    let path = crate::engine::operators::zval_get_string(&path_val);
    let is_once = op.extended_value == 2 || op.extended_value == 3;
    if is_once && execute_data.included_files.contains(path.as_str()) {
        return Ok(ExecResult::Continue);
    }
    match crate::engine::compile::compile_file(path.as_str()) {
        Ok(included_op_array) => {
            execute_data.included_files.insert(path.as_str().to_string());
            let result = super::execute::execute_ex(execute_data, &included_op_array);
            if result == crate::engine::types::PhpResult::Failure {
                return Err(format!("Failed to execute included file: {}", path.as_str()));
            }
            Ok(ExecResult::Continue)
        }
        Err(e) => {
            if op.extended_value == 1 || op.extended_value == 3 {
                Err(format!("require({}): {}", path.as_str(), e))
            } else {
                eprintln!("Warning: include({}): {}", path.as_str(), e);
                Ok(ExecResult::Continue)
            }
        }
    }
}

fn execute_init_array(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let arr = crate::engine::types::PhpArray::new();
    let arr_zval = Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, arr_zval);
    }
    Ok(ExecResult::Continue)
}

fn execute_add_array_element(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    if is_temp_ref(&op.op1) {
        if let PhpValue::Long(slot_idx) = op.op1.value {
            let arr_slot = slot_idx as usize;
            let value = resolve_operand(&op.op2, execute_data);
            let mut arr_zval = execute_data.get_temp(arr_slot);
            if let PhpValue::Array(ref mut arr) = arr_zval.value {
                if op.extended_value != 0 {
                    let key = resolve_operand(&op.result, execute_data);
                    let key_str = crate::engine::operators::zval_get_string(&key);
                    let key_zs = Box::new(crate::engine::string::string_init(key_str.as_str(), false));
                    let _ = crate::engine::hash::hash_add_or_update(arr, Some(&*key_zs), 0, value, 0);
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

fn execute_fetch_dim(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let arr_val = resolve_operand(&op.op1, execute_data);
    let idx_val = resolve_operand(&op.op2, execute_data);
    let result_val = if let PhpValue::Array(ref arr) = arr_val.value {
        match &idx_val.value {
            PhpValue::Long(i) => {
                crate::engine::hash::hash_index_find(arr, *i as u64)
                    .map(|v| clone_val(v))
                    .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null))
            }
            PhpValue::String(s) => {
                crate::engine::hash::hash_find(arr, s)
                    .map(|v| clone_val(v))
                    .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null))
            }
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

fn execute_new_obj(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let class_name_val = resolve_operand(&op.op1, execute_data);
    let class_name = crate::engine::operators::zval_get_string(&class_name_val);
    let cn = class_name.as_str();

    let mut obj = crate::engine::types::PhpObject::new(cn);
    if let Some(ce) = execute_data.class_table.get(cn) {
        for (prop_name, prop_val) in &ce.default_properties {
            obj.properties.insert(prop_name.clone(), clone_val(prop_val));
        }
    }

    let obj_zval = Val::new(PhpValue::Object(Box::new(obj)), PhpType::Object);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, clone_val(&obj_zval));
    }
    Ok(ExecResult::Continue)
}

fn execute_fetch_obj_prop(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let obj_val = resolve_operand(&op.op1, execute_data);
    let prop_name_val = resolve_operand(&op.op2, execute_data);
    let prop_name = crate::engine::operators::zval_get_string(&prop_name_val);

    let result_val = if let PhpValue::Object(ref obj) = obj_val.value {
        obj.properties.get(prop_name.as_str())
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

fn execute_assign_obj_prop(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let var_name_val = &op.op1;
    let prop_name_val = resolve_operand(&op.op2, execute_data);
    let prop_name = crate::engine::operators::zval_get_string(&prop_name_val);
    let value = resolve_operand(&op.result, execute_data);

    if is_var_ref(var_name_val) {
        if let PhpValue::String(ref s) = var_name_val.value {
            let vname = s.as_str();
            let name = if vname.starts_with('$') { &vname[1..] } else { vname };
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

fn execute_do_method_call(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let method_name_val = resolve_operand(&op.op1, execute_data);
    let method_name = crate::engine::operators::zval_get_string(&method_name_val);
    let obj_val = resolve_operand(&op.op2, execute_data);

    if let PhpValue::Object(ref obj) = obj_val.value {
        let class_name = obj.class_name.clone();
        // Extract method info (owned copies to avoid borrow conflict)
        let method_info: Option<(Vec<String>, Vec<Op>)> = execute_data.class_table.get(&class_name)
            .and_then(|ce| ce.methods.get(method_name.as_str()))
            .map(|m| {
                let params = m.params.clone();
                let ops: Vec<Op> = m.op_array.ops.iter().map(|op| {
                    Op::new(op.opcode, clone_val(&op.op1), clone_val(&op.op2), clone_val(&op.result), op.extended_value)
                }).collect();
                (params, ops)
            });

        if let Some((params, ops)) = method_info {
            // Set up $this
            execute_data.set_var("this", clone_val(&obj_val));

            // Set up method parameters
            let args: Vec<Val> = execute_data.call_args.iter().map(|a| clone_val(a)).collect();
            for (i, param_name) in params.iter().enumerate() {
                if let Some(arg) = args.get(i) {
                    execute_data.set_var(param_name, clone_val(arg));
                }
            }

            // Execute method
            let mut method_op_array = OpArray::new(format!("{}::{}", class_name, method_name.as_str()));
            method_op_array.ops = ops;
            let method_result = super::execute::execute_ex(execute_data, &method_op_array);
            if method_result == crate::engine::types::PhpResult::Failure {
                return Err(format!("Method {}::{} failed", class_name, method_name.as_str()));
            }
        }
    }

    execute_data.call_args.clear();
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
    }
    Ok(ExecResult::Continue)
}

fn execute_do_fcall(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let func_name = if is_var_ref(&op.op1) || is_temp_ref(&op.op1) {
        let resolved = resolve_operand(&op.op1, execute_data);
        crate::engine::operators::zval_get_string(&resolved).as_str().to_string()
    } else {
        crate::engine::operators::zval_get_string(&op.op1).as_str().to_string()
    };

    let args: Vec<Val> = execute_data.call_args.drain(..).collect();

    match execute_builtin_function(&func_name, &args, execute_data)? {
        Some(result) => {
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, result);
            }
            Ok(ExecResult::Continue)
        }
        None => {
            // Try user-defined function table
            if let Some(ref function_table) = execute_data.function_table {
                if let Some(ft) = function_table.downcast_ref::<crate::engine::compile::function_table::FunctionTable>() {
                    if let Some(func_op_array) = ft.lookup_function(&func_name) {
                        let mut child = ExecuteData::new();
                        child.function_table = execute_data.function_table.clone();
                        let _result = super::execute::execute_ex(&mut child, func_op_array);
                        return Ok(ExecResult::Continue);
                    }
                }
            }
            eprintln!("Warning: Call to undefined function {}()", func_name);
            Ok(ExecResult::Continue)
        }
    }
}
