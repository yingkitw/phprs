//! Optimized opcode handlers for direct dispatch
//!
//! This module contains individual handler functions for each opcode,
//! designed for maximum performance with direct function calls and JIT optimization.

use super::builtins::execute_builtin_function;
use super::execute_data::{
    ExecResult, ExecuteData, clone_val, is_temp_ref, is_var_ref, resolve_operand, result_slot,
};
use super::opcodes::{Op, OpArray, Opcode};

use crate::engine::hash::hash_find;
use crate::engine::jit::{increment_execution_counter, try_inline_operation};
use crate::engine::string::string_init;
use crate::engine::types::{PhpArray, PhpType, PhpValue, Val};

/// Serialize a SimpleXMLElement PHP object back to XML string.
fn simplexml_object_to_string(obj: &crate::engine::types::PhpObject) -> String {
    fn serialize_obj(o: &crate::engine::types::PhpObject, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        let name = o.properties.get("__name")
            .map(|v| crate::engine::operators::zval_get_string(v).as_str().to_string())
            .unwrap_or_else(|| "root".to_string());
        let text = o.properties.get("__text")
            .map(|v| crate::engine::operators::zval_get_string(v).as_str().to_string())
            .unwrap_or_default();
        let mut attrs = String::new();
        let mut children = String::new();
        for (k, v) in &o.properties {
            if let Some(attr_name) = k.strip_prefix('@') {
                let val_str = crate::engine::operators::zval_get_string(v).as_str().to_string();
                attrs.push_str(&format!(" {attr_name}=\"{val_str}\""));
            } else if !k.starts_with("__") {
                if let PhpValue::Object(child) = &v.value {
                    children.push_str(&serialize_obj(child, indent + 1));
                    children.push('\n');
                } else if let PhpValue::Array(arr) = &v.value {
                    for bucket in &arr.ar_data {
                        if let PhpValue::Object(child) = &bucket.val.value {
                            children.push_str(&serialize_obj(child, indent + 1));
                            children.push('\n');
                        }
                    }
                }
            }
        }
        if children.is_empty() && text.is_empty() {
            format!("{pad}<{name}{attrs} />")
        } else if children.is_empty() {
            format!("{pad}<{name}{attrs}>{text}</{name}>")
        } else {
            format!("{pad}<{name}{attrs}>\n{children}{pad}</{name}>")
        }
    }
    serialize_obj(obj, 0)
}

/// Convert an XmlNode to a SimpleXMLElement Val.
fn simplexml_node_to_val(node: &crate::php::xml::XmlNode) -> Val {
    let mut obj = crate::engine::types::PhpObject::new("SimpleXMLElement");
    obj.properties.insert("__name".to_string(), Val::new(
        PhpValue::String(Box::new(string_init(&node.name, false))), PhpType::String));
    if !node.text.is_empty() {
        obj.properties.insert("__text".to_string(), Val::new(
            PhpValue::String(Box::new(string_init(&node.text, false))), PhpType::String));
    }
    for (k, v) in &node.attributes {
        let key = format!("@{k}");
        obj.properties.insert(key, Val::new(
            PhpValue::String(Box::new(string_init(v, false))), PhpType::String));
    }
    let mut child_map: std::collections::HashMap<String, Vec<&crate::php::xml::XmlNode>> = std::collections::HashMap::new();
    for child in &node.children {
        child_map.entry(child.name.clone()).or_default().push(child);
    }
    for (name, children) in child_map {
        if children.len() == 1 {
            obj.properties.insert(name, simplexml_node_to_val(children[0]));
        } else {
            let mut arr = PhpArray::new();
            for (i, child) in children.iter().enumerate() {
                let _ = crate::engine::hash::hash_add_or_update(
                    &mut arr, None, i as u64, simplexml_node_to_val(child), 0);
            }
            obj.properties.insert(name, Val::new(
                PhpValue::Array(Box::new(arr)), PhpType::Array));
        }
    }
    Val::new(PhpValue::Object(Box::new(obj)), PhpType::Object)
}

/// Call __toString magic method on an object if it exists
#[inline]
fn call_magic_tostring(
    val: &Val,
    execute_data: &mut ExecuteData,
) -> Option<crate::engine::types::PhpString> {
    // Built-in classes with __toString handled directly
    if let PhpValue::Object(ref obj) = val.value
        && obj.class_name == "SimpleXMLElement" {
            return obj.properties.get("__text").map(|v| {
                crate::engine::operators::zval_get_string(v)
            });
        }
    if let PhpValue::Object(ref obj) = val.value
        && let Some(ce) = execute_data.class_table.get(&obj.class_name)
        && let Some(magic) = ce.methods.get("__toString")
    {
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
        let saved_try_depth = execute_data.try_stack.len();
        let (_status, return_val) =
            super::execute::execute_ex_returning(execute_data, &method_op_array);
        execute_data.op_array = saved_op_array;
        execute_data.current_op = saved_current_op;
        execute_data.called_class = saved_called_class;
        execute_data.try_stack.truncate(saved_try_depth);

        if let Some(ret) = return_val {
            return Some(crate::engine::operators::zval_get_string(&ret));
        }
    }
    None
}

/// Resolve a relative include/require path like PHP: try the path relative to the process
/// current working directory first, then relative to the including script's directory.
#[inline]
fn resolve_include_path(path_str: &str, script_dir: Option<&str>) -> String {
    use std::path::Path;
    if path_str.starts_with('/') || (path_str.len() >= 2 && path_str.get(1..2) == Some(":")) {
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
    if execute_data.pending_exception.is_some() {
        // __toString threw: unwind the frame (propagation handled by callers).
        return Ok(ExecResult::Return(op1.clone()));
    }
    let s2 = if let Some(tostr) = call_magic_tostring(&op2, execute_data) {
        tostr
    } else {
        crate::engine::operators::zval_get_string(&op2)
    };
    if execute_data.pending_exception.is_some() {
        return Ok(ExecResult::Return(op1.clone()));
    }
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
pub fn execute_bw_and(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    bitwise_binary(op, execute_data, |a, b| a & b)
}

#[inline]
pub fn execute_bw_or(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    bitwise_binary(op, execute_data, |a, b| a | b)
}

#[inline]
pub fn execute_bw_xor(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    bitwise_binary(op, execute_data, |a, b| a ^ b)
}

#[inline]
fn bitwise_binary(
    op: &Op,
    execute_data: &mut ExecuteData,
    f: impl Fn(i64, i64) -> i64,
) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let result = Val::new(
        PhpValue::Long(f(
            crate::engine::operators::zval_get_long(&op1),
            crate::engine::operators::zval_get_long(&op2),
        )),
        PhpType::Long,
    );
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_bw_not(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let n = !crate::engine::operators::zval_get_long(&val);
    let result = Val::new(PhpValue::Long(n), PhpType::Long);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_sl(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    shift(op, execute_data, |a, s| {
        a.checked_shl(s as u32).unwrap_or(0)
    })
}

#[inline]
pub fn execute_spaceship(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let cmp = crate::engine::operators::zval_compare(&op1, &op2).signum() as i64;
    let result = Val::new(PhpValue::Long(cmp), PhpType::Long);
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, result);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_sr(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    // Arithmetic shift right (sign-preserving); PHP masks the shift count
    shift(op, execute_data, |a, s| a >> (s & 63))
}

#[inline]
fn shift(
    op: &Op,
    execute_data: &mut ExecuteData,
    f: impl Fn(i64, i64) -> i64,
) -> Result<ExecResult, String> {
    let op1 = resolve_operand(&op.op1, execute_data);
    let op2 = resolve_operand(&op.op2, execute_data);
    let a = crate::engine::operators::zval_get_long(&op1);
    let s = crate::engine::operators::zval_get_long(&op2) & 63; // PHP masks shift count on 64-bit
    let result = Val::new(PhpValue::Long(f(a, s)), PhpType::Long);
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
    // int ** non-negative int stays int (PHP semantics)
    let result = if let (PhpValue::Long(base), PhpValue::Long(exp)) = (&op1.value, &op2.value)
        && (0..=63).contains(exp)
    {
        Val::new(
            PhpValue::Long(base.wrapping_pow(*exp as u32)),
            PhpType::Long,
        )
    } else {
        let v1 = crate::engine::operators::zval_get_double(&op1);
        let v2 = crate::engine::operators::zval_get_double(&op2);
        Val::new(PhpValue::Double(v1.powf(v2)), PhpType::Double)
    };
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
        let clean = name.strip_prefix('$').unwrap_or(name);
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
            let clean = name.strip_prefix('$').unwrap_or(name);
            execute_data.set_var(clean, container);
        }
    };

    let mut container = if is_temp_ref(&op.op1) {
        resolve_operand(&op.op1, execute_data)
    } else if let PhpValue::String(var_s) = &op.op1.value {
        let name = var_s.as_str();
        let clean = name.strip_prefix('$').unwrap_or(name);
        execute_data.get_var(clean)
    } else {
        return Ok(ExecResult::Continue);
    };

    if container.get_type() == PhpType::Null {
        container = Val::new(PhpValue::Array(Box::default()), PhpType::Array);
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
                PhpValue::Double(d) if d.fract() == 0.0 => {
                    let _ = crate::engine::hash::hash_add_or_update(
                        arr,
                        None,
                        *d as i64 as u64,
                        val,
                        0,
                    );
                }
                PhpValue::String(ks) => {
                    let key_zs = Box::new(crate::engine::string::string_init(ks.as_str(), false));
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
    let had_pending_before = execute_data.pending_exception.is_some();
    let s = if let Some(tostr) = call_magic_tostring(&val, execute_data) {
        tostr
    } else {
        crate::engine::operators::zval_get_string(&val)
    };
    if !had_pending_before && execute_data.pending_exception.is_some() {
        // __toString threw: unwind the frame (propagation handled by callers).
        return Ok(ExecResult::Return(val));
    }
    let _ = crate::php::output::php_output_write(s.as_bytes());
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_return(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    Ok(ExecResult::Return(val))
}

fn fcc_array_string_field(arr: &crate::engine::types::PhpArray, key: &str) -> Option<String> {
    let k = string_init(key, false);
    hash_find(arr, &k).map(|v| {
        crate::engine::operators::zval_get_string(v)
            .as_str()
            .to_string()
    })
}

fn try_invoke_first_class_callable(
    callable: &Val,
    args: Vec<Val>,
    arg_names: Vec<Option<String>>,
    arg_by_ref: Vec<bool>,
    execute_data: &mut ExecuteData,
) -> Result<Option<Val>, String> {
    if let PhpValue::Array(ref arr) = callable.value {
        let Some(kind) = fcc_array_string_field(arr, "type") else {
            return Ok(None);
        };
        return match kind.as_str() {
            "method" => {
                let method = fcc_array_string_field(arr, "method")
                    .ok_or_else(|| "Invalid method first-class callable".to_string())?;
                let k = string_init("object", false);
                let obj = hash_find(arr, &k).ok_or_else(|| {
                    "Invalid method first-class callable: missing object".to_string()
                })?;
                fcc_invoke_instance_method(
                    execute_data,
                    clone_val(obj),
                    &method,
                    args,
                    arg_names,
                    arg_by_ref,
                )
            }
            "static" => {
                let class = fcc_array_string_field(arr, "class")
                    .ok_or_else(|| "Invalid static first-class callable".to_string())?;
                let method = fcc_array_string_field(arr, "method")
                    .ok_or_else(|| "Invalid static first-class callable".to_string())?;
                fcc_invoke_static_method(execute_data, &class, &method, args, arg_names, arg_by_ref)
            }
            _ => Ok(None),
        };
    }

    if callable.get_type() != PhpType::Callable {
        return Ok(None);
    }

    let PhpValue::String(ref name) = callable.value else {
        return Ok(None);
    };
    let target = name.as_str();

    if let Some((class_name, method_name)) = target.split_once("::") {
        return fcc_invoke_static_method(
            execute_data,
            class_name,
            method_name,
            args,
            arg_names,
            arg_by_ref,
        );
    }

    let func_name = target.to_ascii_lowercase();
    if let Some(result) = execute_builtin_function(&func_name, &args, execute_data)? {
        return Ok(Some(result));
    }

    Err(format!("Call to undefined function {target}()"))
}

fn fcc_invoke_static_method(
    execute_data: &mut ExecuteData,
    class_name: &str,
    method_name: &str,
    args: Vec<Val>,
    arg_names: Vec<Option<String>>,
    arg_by_ref: Vec<bool>,
) -> Result<Option<Val>, String> {
    let resolved_class = class_name.to_string();
    let method_info = execute_data
        .class_table
        .get(&resolved_class)
        .and_then(|ce| ce.methods.get(method_name))
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
                .unwrap_or_else(|| format!("{resolved_class}::{method_name}"));
            (
                params,
                ops,
                file_label,
                m.op_array.variadic_param.clone(),
                m.op_array.ref_params.clone(),
            )
        });

    let Some((params, ops, oparray_filename, variadic, ref_params)) = method_info else {
        return Err(format!(
            "Call to undefined static method {resolved_class}::{method_name}()"
        ));
    };

    let saved_current_op = execute_data.current_op;
    let saved_op_array = execute_data.op_array.take();
    let saved_script_dir = execute_data.current_script_dir.clone();
    let saved_called_class = execute_data.called_class.clone();
    execute_data.called_class = Some(resolved_class);

    bind_call_args(
        execute_data,
        &params,
        &args,
        &arg_names,
        &variadic,
        &ref_params,
        &arg_by_ref,
    );

    let saved_try_depth = execute_data.try_stack.len();
    let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
    method_op_array.ops = ops;
    let (_status, return_val) =
        super::execute::execute_ex_returning(execute_data, &method_op_array);

    execute_data.op_array = saved_op_array;
    execute_data.current_op = saved_current_op;
    execute_data.current_script_dir = saved_script_dir;
    execute_data.called_class = saved_called_class;
    // An uncaught throw in the callable's frame: drop its try frames and
    // leave the exception pending — DoFCall re-dispatches it against this
    // frame via propagate_after_call.
    execute_data.try_stack.truncate(saved_try_depth);

    Ok(return_val.or_else(|| Some(Val::new(PhpValue::Long(0), PhpType::Null))))
}

fn fcc_invoke_instance_method(
    execute_data: &mut ExecuteData,
    obj_val: Val,
    method_name: &str,
    args: Vec<Val>,
    arg_names: Vec<Option<String>>,
    arg_by_ref: Vec<bool>,
) -> Result<Option<Val>, String> {
    let PhpValue::Object(ref obj) = obj_val.value else {
        return Err("First-class method callable requires an object".to_string());
    };
    let class_name = obj.class_name.clone();

    let method_info = execute_data
        .class_table
        .get(&class_name)
        .and_then(|ce| ce.methods.get(method_name))
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
                .unwrap_or_else(|| format!("{class_name}::{method_name}"));
            (
                params,
                ops,
                file_label,
                m.op_array.variadic_param.clone(),
                m.op_array.ref_params.clone(),
            )
        });

    let Some((params, ops, oparray_filename, variadic, ref_params)) = method_info else {
        return Err(format!(
            "Call to undefined method {class_name}::{method_name}()"
        ));
    };

    let saved_current_op = execute_data.current_op;
    let saved_op_array = execute_data.op_array.take();
    let saved_script_dir = execute_data.current_script_dir.clone();
    let saved_called_class = execute_data.called_class.clone();
    execute_data.called_class = Some(class_name.clone());
    execute_data.set_var("this", clone_val(&obj_val));

    bind_call_args(
        execute_data,
        &params,
        &args,
        &arg_names,
        &variadic,
        &ref_params,
        &arg_by_ref,
    );

    let saved_try_depth = execute_data.try_stack.len();
    let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
    method_op_array.ops = ops;
    let (_status, return_val) =
        super::execute::execute_ex_returning(execute_data, &method_op_array);

    execute_data.op_array = saved_op_array;
    execute_data.current_op = saved_current_op;
    execute_data.current_script_dir = saved_script_dir;
    execute_data.called_class = saved_called_class;
    // An uncaught throw in the callable's frame: drop its try frames and
    // leave the exception pending — DoFCall re-dispatches it against this
    // frame via propagate_after_call.
    execute_data.try_stack.truncate(saved_try_depth);

    Ok(return_val.or_else(|| Some(Val::new(PhpValue::Long(0), PhpType::Null))))
}

#[inline]
pub fn execute_do_fcall(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let resolved_op1 = if is_var_ref(&op.op1) || is_temp_ref(&op.op1) {
        resolve_operand(&op.op1, execute_data)
    } else {
        clone_val(&op.op1)
    };
    let fcall_try_depth = execute_data.try_stack.len();

    // Magic method: __invoke for callable objects
    if let PhpValue::Object(ref obj) = resolved_op1.value
        && let Some(ce) = execute_data.class_table.get(&obj.class_name)
        && let Some(magic) = ce.methods.get("__invoke")
    {
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
        let saved_try_depth = execute_data.try_stack.len();
        let (_status, return_val) =
            super::execute::execute_ex_returning(execute_data, &method_op_array);
        execute_data.op_array = saved_op_array;
        execute_data.current_op = saved_current_op;
        execute_data.current_script_dir = saved_script_dir;
        execute_data.called_class = saved_called_class;
        if let Some(er) = crate::engine::vm::exception_dispatch::propagate_after_call(
            execute_data,
            saved_try_depth,
        ) {
            return Ok(er);
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

    let (base, names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
    let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
    let arg_names: Vec<Option<String>> = execute_data.call_arg_names.drain(names_base..).collect();
    let arg_by_ref: Vec<bool> = execute_data.call_arg_by_ref.drain(base..).collect();

    if let Some(result) = try_invoke_first_class_callable(
        &resolved_op1,
        args.clone(),
        arg_names.clone(),
        arg_by_ref.clone(),
        execute_data,
    )? {
        // A first-class callable (method string, closure, …) may have thrown.
        if let Some(er) = crate::engine::vm::exception_dispatch::propagate_after_call(
            execute_data,
            fcall_try_depth,
        ) {
            return Ok(er);
        }
        if let Some(slot) = result_slot(op) {
            execute_data.set_temp(slot, result);
        }
        return Ok(ExecResult::Continue);
    }

    let func_name = crate::engine::operators::zval_get_string(&resolved_op1)
        .as_str()
        .to_ascii_lowercase();

    increment_execution_counter(&func_name);

    // Builtins may re-enter the VM for user callbacks (call_user_func,
    // array_map, …). If such a callback threw uncaught, re-dispatch the
    // pending exception against this frame.
    let builtin_try_depth = execute_data.try_stack.len();
    let fcall_result = execute_builtin_function(&func_name, &args, execute_data);
    if let Some(er) =
        crate::engine::vm::exception_dispatch::propagate_after_call(execute_data, builtin_try_depth)
    {
        return Ok(er);
    }
    match fcall_result? {
        Some(result) => {
            if let Some(slot) = result_slot(op) {
                execute_data.set_temp(slot, result);
            }
            Ok(ExecResult::Continue)
        }
        None => {
            // A known builtin returning None is a void builtin (e.g. var_dump,
            // echo, unset) whose side effects already ran. Don't treat it as
            // "undefined" or fall through to user-function lookup.
            if super::builtins::is_builtin_function(&func_name) {
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
                }
                return Ok(ExecResult::Continue);
            }
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
                                    name.strip_prefix('$').unwrap_or(name).to_string()
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
                        cloned.ref_params = func_op_array.ref_params.clone();
                        cloned.variadic_param = func_op_array.variadic_param.clone();
                        cloned.is_generator = func_op_array.is_generator;
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
                let ref_params = func_op_array.ref_params.clone();

                // Generator function: return a Generator object instead of executing.
                // The generator stores the op_array and initial state; execution
                // begins on the first next()/rewind() call.
                if func_op_array.is_generator {
                    let mut gen_obj = crate::engine::types::PhpObject::new("Generator");
                    gen_obj.properties.insert("__op_array".to_string(),
                        Val::new(PhpValue::Object(Box::new(
                            crate::engine::types::PhpObject::new("__GeneratorOpArray")
                        )), PhpType::Object));
                    // Store the op_array in the generator frames slot
                    let frame_idx = execute_data.fiber_frames.len();
                    // Save the generator's op_array and initial state in a FiberFrame
                    let gen_frame = super::execute_data::FiberFrame {
                        op_array: func_op_array,
                        current_op: 0,
                        temp_vars: Vec::new(),
                        symbol_table: None,
                        call_args: args.clone(),
                        call_arg_stack: Vec::new(),
                        call_arg_names: arg_names.clone(),
                        call_arg_by_ref: arg_by_ref.clone(),
                        ref_caller_scope: None,
                        ref_param_bindings: std::collections::HashMap::new(),
                        global_imports: std::collections::HashSet::new(),
                        try_stack: Vec::new(),
                    };
                    execute_data.fiber_frames.push(gen_frame);
                    gen_obj.properties.insert("__frame_index".to_string(),
                        Val::new(PhpValue::Long(frame_idx as i64), PhpType::Long));
                    gen_obj.properties.insert("__state".to_string(),
                        Val::new(PhpValue::Long(0), PhpType::Long)); // 0=pending, 1=running, 2=suspended, 3=terminated
                    gen_obj.properties.insert("__current_value".to_string(),
                        Val::new(PhpValue::Long(0), PhpType::Null));
                    gen_obj.properties.insert("__current_key".to_string(),
                        Val::new(PhpValue::Long(0), PhpType::Null));
                    gen_obj.properties.insert("__return_value".to_string(),
                        Val::new(PhpValue::Long(0), PhpType::Null));
                    gen_obj.properties.insert("__send_value".to_string(),
                        Val::new(PhpValue::Long(0), PhpType::Null));
                    gen_obj.properties.insert("__param_names".to_string(),
                        Val::new(PhpValue::String(Box::new(
                            crate::engine::string::string_init(
                                &param_names.iter().map(|n| n.as_str()).collect::<Vec<_>>().join(","),
                                false,
                            )
                        )), PhpType::String));
                    gen_obj.properties.insert("__variadic_param".to_string(),
                        Val::new(PhpValue::String(Box::new(
                            crate::engine::string::string_init(
                                variadic_param.as_deref().unwrap_or(""), false)
                        )), PhpType::String));
                    gen_obj.properties.insert("__ref_params".to_string(),
                        Val::new(PhpValue::String(Box::new(
                            crate::engine::string::string_init(
                                &ref_params.iter().map(|b| if *b { "1" } else { "0" })
                                    .collect::<Vec<_>>().join(","), false)
                        )), PhpType::String));
                    let gen_val = Val::new(PhpValue::Object(Box::new(gen_obj)), PhpType::Object);
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, gen_val);
                    }
                    return Ok(ExecResult::Continue);
                }

                // Note: JIT compilation check removed to prevent deadlock

                // Save current execution state
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

                if execute_data.global_script_table.is_none()
                    && let Some(ref saved) = saved_symbol_table
                {
                    execute_data.global_script_table =
                        Some(super::execute_data::ExecuteData::clone_php_array(saved));
                }

                execute_data.ref_caller_scope = saved_symbol_table;
                execute_data.symbol_table = Some(crate::engine::types::PhpArray::new());

                bind_call_args(
                    execute_data,
                    &param_names,
                    &args,
                    &arg_names,
                    &variadic_param,
                    &ref_params,
                    &arg_by_ref,
                );

                let saved_script_dir = execute_data.current_script_dir.clone();
                let saved_magic_dir = execute_data.constants.get("__DIR__").map(clone_val);
                let saved_magic_file = execute_data.constants.get("__FILE__").map(clone_val);
                let saved_try_depth = execute_data.try_stack.len();
                // Execute the function and capture return value
                let (_status, return_val) =
                    super::execute::execute_ex_returning(execute_data, &func_op_array);

                // Restore execution state
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

                // A throw in the callee with no local catch lands here.
                if let Some(er) = crate::engine::vm::exception_dispatch::propagate_after_call(
                    execute_data,
                    saved_try_depth,
                ) {
                    return Ok(er);
                }

                // Store return value in result temp slot
                if let Some(ret) = return_val
                    && let Some(slot) = result_slot(op)
                {
                    execute_data.set_temp(slot, ret);
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
    execute_data.call_arg_by_ref.push(false);
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_send_var_ref(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    execute_data.call_args.push(clone_val(&op.op1));
    execute_data.call_arg_names.push(None);
    execute_data.call_arg_by_ref.push(true);
    Ok(ExecResult::Continue)
}

// --- Exception handling ---

#[inline]
pub fn execute_try_catch_begin(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    let _ = op;
    let filename = execute_data
        .op_array
        .as_ref()
        .and_then(|o| o.filename.clone());
    execute_data
        .try_stack
        .push((execute_data.current_op, filename));
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_try_catch_end(
    _op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    // Normal exit from the try body: drop the matching frame if it is on top.
    if let Some((idx, _)) = execute_data.try_stack.last()
        && *idx == execute_data.current_op
    {
        execute_data.try_stack.pop();
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_throw(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let thrown = resolve_operand(&op.op1, execute_data);
    let (class, _message) =
        crate::engine::vm::exception_dispatch::thrown_class_and_message(&thrown);
    execute_data.pending_exception = Some(clone_val(&thrown));

    match crate::engine::vm::exception_dispatch::dispatch_exception(execute_data, &class) {
        crate::engine::vm::exception_dispatch::ExceptionOutcome::Caught { body_start, var } => {
            execute_data.pending_exception = None;
            execute_data.set_var(&var, thrown);
            Ok(ExecResult::Jump(body_start))
        }
        crate::engine::vm::exception_dispatch::ExceptionOutcome::FinallyRun { body_start } => {
            // Jump to the finally body; exception stays pending for
            // re-dispatch at FinallyEnd.
            Ok(ExecResult::Jump(body_start))
        }
        crate::engine::vm::exception_dispatch::ExceptionOutcome::Uncaught => {
            // No enclosing catch in this frame: pending_exception is left set
            // and the frame unwinds. The caller's propagate_after_call (or the
            // top-level runner) catches or reports the fatal error.
            Ok(ExecResult::Return(thrown))
        }
    }
}

#[inline]
pub fn execute_finally_end(_op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    // On the normal path (no pending exception), finally just falls through.
    // During exception unwinding, re-dispatch the pending exception to either
    // find an outer catch, an outer finally, or unwind the frame.
    if let Some(thrown) = execute_data.pending_exception.take() {
        let (class, _) = crate::engine::vm::exception_dispatch::thrown_class_and_message(&thrown);
        match crate::engine::vm::exception_dispatch::dispatch_exception(execute_data, &class) {
            crate::engine::vm::exception_dispatch::ExceptionOutcome::Caught { body_start, var } => {
                execute_data.set_var(&var, thrown);
                Ok(ExecResult::Jump(body_start))
            }
            crate::engine::vm::exception_dispatch::ExceptionOutcome::FinallyRun { body_start } => {
                execute_data.pending_exception = Some(thrown);
                Ok(ExecResult::Jump(body_start))
            }
            crate::engine::vm::exception_dispatch::ExceptionOutcome::Uncaught => {
                execute_data.pending_exception = Some(clone_val(&thrown));
                Ok(ExecResult::Return(thrown))
            }
        }
    } else {
        Ok(ExecResult::Continue)
    }
}

#[inline]
pub fn execute_catch_marker(
    _op: &Op,
    _execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    // CatchBegin / CatchEnd / FinallyBegin are structural markers that are
    // no-ops during normal linear execution. Catch dispatch happens in the
    // Throw handler; finally runs on both normal and exception paths.
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_send_val_named(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let name_val = resolve_operand(&op.op2, execute_data);
    let name = crate::engine::operators::zval_get_string(&name_val);
    execute_data.call_args.push(val);
    execute_data
        .call_arg_names
        .push(Some(name.as_str().to_string()));
    execute_data.call_arg_by_ref.push(false);
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
            let saved_try_depth = execute_data.try_stack.len();
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
            // A throw inside the included file with no local catch lands here.
            if let Some(er) = crate::engine::vm::exception_dispatch::propagate_after_call(
                execute_data,
                saved_try_depth,
            ) {
                return Ok(er);
            }
            if result == crate::engine::types::PhpResult::Failure {
                return Err(format!("Failed to execute included file: {}", resolved));
            }
            Ok(ExecResult::Continue)
        }
        Err(e) => {
            if op.extended_value == 1 || op.extended_value == 3 {
                Err(format!("require({}): {}", resolved, e))
            } else {
                eprintln!("Warning: include({}): {}", resolved, e);
                Ok(ExecResult::Continue)
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
    if is_temp_ref(v)
        && let PhpValue::Long(i) = v.value
    {
        return Some(i as usize);
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
        execute_data.set_temp(iter_slot, Val::new(PhpValue::Long(0), PhpType::Long));
    } else if matches!(arr.value, PhpValue::Object(_)) {
        // For objects (including Generator), store the object itself as the iterator.
        // FeFetch will dispatch to the appropriate method calls.
        execute_data.set_temp(iter_slot, arr);
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

    // Handle Generator objects: foreach over a generator is not directly
    // supported via FeFetch because advancing the generator requires VM
    // re-entry (saving/restoring state). Users should use the
    // while($g->valid()) { ... $g->next(); } pattern instead.
    if let PhpValue::Object(ref obj) = arr.value
        && obj.class_name == "Generator"
    {
        return Ok(ExecResult::Jump(op.extended_value));
    }

    let PhpValue::Array(arr) = &arr.value else {
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
    if is_temp_ref(&op.op1)
        && let PhpValue::Long(slot_idx) = op.op1.value
    {
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
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_fetch_dim(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let arr_val = resolve_operand(&op.op1, execute_data);
    let idx_val = resolve_operand(&op.op2, execute_data);
    // Missing keys read as null (PHP emits a notice but continues)
    let missing = || Val::new(PhpValue::Long(0), PhpType::Null);
    let result_val = if let PhpValue::Array(ref arr) = arr_val.value {
        match &idx_val.value {
            PhpValue::Long(i) => crate::engine::hash::hash_index_find(arr, *i as u64)
                .map(clone_val)
                .unwrap_or_else(missing),
            PhpValue::Double(d) if d.fract() == 0.0 => {
                crate::engine::hash::hash_index_find(arr, *d as i64 as u64)
                    .map(clone_val)
                    .unwrap_or_else(missing)
            }
            PhpValue::String(s) => crate::engine::hash::hash_find(arr, s)
                .map(clone_val)
                .unwrap_or_else(missing),
            _ => missing(),
        }
    } else if let PhpValue::Object(ref obj) = arr_val.value {
        // SimpleXMLElement attribute access: $xml["attr"] → @attr property
        if obj.class_name == "SimpleXMLElement" {
            let key = crate::engine::operators::zval_get_string(&idx_val);
            let attr_key = format!("@{}", key.as_str());
            obj.properties.get(&attr_key).map(clone_val).unwrap_or_else(missing)
        } else {
            missing()
        }
    } else {
        missing()
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
    // Built-in DateTime: initialize with current timestamp (constructor
    // args are applied later in execute_do_method_call for __construct).
    if cn == "DateTime" || cn == "DateTimeImmutable" {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        obj.properties.insert(
            "timestamp".to_string(),
            Val::new(PhpValue::Long(now), PhpType::Long),
        );
        obj.properties.insert(
            "timezone_offset".to_string(),
            Val::new(PhpValue::Long(0), PhpType::Long),
        );
        obj.properties.insert(
            "timezone_name".to_string(),
            Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init("UTC", false))),
                PhpType::String,
            ),
        );
    }
    // Built-in DateTimeZone: store the timezone name. Constructor arg applied
    // in execute_do_method_call for __construct.
    if cn == "DateTimeZone" {
        obj.properties.insert(
            "name".to_string(),
            Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init("UTC", false))),
                PhpType::String,
            ),
        );
    }
    // Built-in ArrayIterator: initialize with an empty array and index 0.
    // The constructor argument (if any) is applied in execute_do_method_call.
    if cn == "ArrayIterator" {
        obj.properties.insert(
            "__storage".to_string(),
            Val::new(PhpValue::Array(Box::default()), PhpType::Array),
        );
        obj.properties.insert(
            "__index".to_string(),
            Val::new(PhpValue::Long(0), PhpType::Long),
        );
    }
    // Built-in Fiber: initialize with state=pending(0), no frame, no return value.
    // The constructor argument (callable) is applied in execute_do_method_call.
    if cn == "Fiber" {
        obj.properties.insert(
            "__state".to_string(),
            Val::new(PhpValue::Long(0), PhpType::Long), // 0=pending, 1=started, 2=suspended, 3=terminated
        );
        obj.properties.insert(
            "__frame_index".to_string(),
            Val::new(PhpValue::Long(-1), PhpType::Long),
        );
        obj.properties.insert(
            "__return_value".to_string(),
            Val::new(PhpValue::Long(0), PhpType::Null),
        );
        obj.properties.insert(
            "__callable".to_string(),
            Val::new(PhpValue::Long(0), PhpType::Null),
        );
    }
    // Built-in SplStack/SplQueue: initialize with an empty array.
    if cn == "SplStack" || cn == "SplQueue" {
        obj.properties.insert(
            "__storage".to_string(),
            Val::new(PhpValue::Array(Box::default()), PhpType::Array),
        );
    }
    // Built-in SplHeap/SplPriorityQueue: initialize with empty storage and index.
    if cn == "SplHeap" || cn == "SplPriorityQueue" || cn == "SplMinHeap" || cn == "SplMaxHeap" {
        obj.properties.insert(
            "__storage".to_string(),
            Val::new(PhpValue::Array(Box::default()), PhpType::Array),
        );
        obj.properties.insert(
            "__index".to_string(),
            Val::new(PhpValue::Long(0), PhpType::Long),
        );
    }
    // Built-in DirectoryIterator: stores path, entries, and index.
    if cn == "DirectoryIterator" {
        obj.properties.insert(
            "__path".to_string(),
            Val::new(PhpValue::Long(0), PhpType::Null),
        );
        obj.properties.insert(
            "__entries".to_string(),
            Val::new(PhpValue::Array(Box::default()), PhpType::Array),
        );
        obj.properties.insert(
            "__index".to_string(),
            Val::new(PhpValue::Long(0), PhpType::Long),
        );
    }
    // Built-in SplFileInfo/SplFileObject: stores path, content, and read position.
    if cn == "SplFileInfo" || cn == "SplFileObject" {
        obj.properties.insert(
            "__path".to_string(),
            Val::new(PhpValue::Long(0), PhpType::Null),
        );
        obj.properties.insert(
            "__content".to_string(),
            Val::new(PhpValue::Long(0), PhpType::Null),
        );
        obj.properties.insert(
            "__pos".to_string(),
            Val::new(PhpValue::Long(0), PhpType::Long),
        );
        obj.properties.insert(
            "__mode".to_string(),
            Val::new(PhpValue::Long(0), PhpType::Null),
        );
    }
    // Built-in SimpleXMLElement: parse XML string in __construct
    if cn == "SimpleXMLElement" {
        obj.properties.insert("__text".to_string(), Val::new(PhpValue::Long(0), PhpType::Null));
        obj.properties.insert("__name".to_string(), Val::new(
            PhpValue::String(Box::new(string_init("root", false))), PhpType::String));
    }
    // Built-in PDO: stores DSN for later connection in __construct
    if cn == "PDO" {
        obj.properties.insert("__dsn".to_string(), Val::new(PhpValue::Long(0), PhpType::Null));
        obj.properties.insert("__connected".to_string(), Val::new(PhpValue::Long(0), PhpType::False));
        obj.properties.insert("__driver".to_string(), Val::new(PhpValue::Long(0), PhpType::Null));
        obj.properties.insert("__last_insert_id".to_string(), Val::new(PhpValue::Long(0), PhpType::Long));
        obj.properties.insert("__error".to_string(), Val::new(
            PhpValue::String(Box::new(string_init("", false))), PhpType::String));
    }
    // Built-in PDOStatement
    if cn == "PDOStatement" {
        obj.properties.insert("__sql".to_string(), Val::new(PhpValue::Long(0), PhpType::Null));
        obj.properties.insert("__rows".to_string(), Val::new(
            PhpValue::Array(Box::default()), PhpType::Array));
        obj.properties.insert("__pos".to_string(), Val::new(PhpValue::Long(0), PhpType::Long));
        obj.properties.insert("__params".to_string(), Val::new(
            PhpValue::Array(Box::default()), PhpType::Array));
        obj.properties.insert("__row_count".to_string(), Val::new(PhpValue::Long(0), PhpType::Long));
    }
    // Built-in Throwable constructors (Exception, RuntimeException, …) store
    // their `message`/`code`/`file` from the constructor call's args. Those
    // args are not on `call_args` yet at NewObj time (SendVal runs after
    // NewObj), so the actual write happens in `execute_do_method_call` for
    // `__construct` when the class is a standard throwable.

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
                    PhpValue::String(Box::new(crate::engine::string::string_init(
                        prop_name.as_str(),
                        false,
                    ))),
                    PhpType::String,
                );
                bind_call_args(
                    execute_data,
                    &params,
                    &[name_val],
                    &[None],
                    &None,
                    &[],
                    &[false],
                );

                let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
                method_op_array.ops = ops;
                let saved_try_depth = execute_data.try_stack.len();
                let (_status, isset_result) =
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
                if let Some(er) = crate::engine::vm::exception_dispatch::propagate_after_call(
                    execute_data,
                    saved_try_depth,
                ) {
                    return Ok(er);
                }

                let is_true = isset_result
                    .map(|v| match v.get_type() {
                        PhpType::True | PhpType::Object | PhpType::Array => true,
                        PhpType::String => {
                            let s = crate::engine::operators::zval_get_string(&v);
                            !s.as_str().is_empty() && s.as_str() != "0"
                        }
                        PhpType::Long => crate::engine::operators::zval_get_long(&v) != 0,
                        PhpType::Double => crate::engine::operators::zval_get_double(&v) != 0.0,
                        _ => false,
                    })
                    .unwrap_or(false);

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
                    PhpValue::String(Box::new(crate::engine::string::string_init(
                        prop_name.as_str(),
                        false,
                    ))),
                    PhpType::String,
                );
                bind_call_args(
                    execute_data,
                    &params,
                    &[name_val],
                    &[None],
                    &None,
                    &[],
                    &[false],
                );

                let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
                method_op_array.ops = ops;
                let saved_try_depth = execute_data.try_stack.len();
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
                if let Some(er) = crate::engine::vm::exception_dispatch::propagate_after_call(
                    execute_data,
                    saved_try_depth,
                ) {
                    return Ok(er);
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
                let name = vname.strip_prefix('$').unwrap_or(vname);
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
            (
                Some(obj.class_name.clone()),
                obj.properties.contains_key(prop_name.as_str()),
            )
        } else {
            (None, false)
        }
    };

    let use_magic = if let Some(ref class_name) = class_name_opt {
        execute_data
            .class_table
            .get(class_name)
            .map(|ce| ce.methods.contains_key("__set"))
            .unwrap_or(false)
            && !has_prop
    } else {
        false
    };

    if use_magic && let Some(ref class_name) = class_name_opt {
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
                    let name = vname.strip_prefix('$').unwrap_or(vname);
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
                PhpValue::String(Box::new(crate::engine::string::string_init(
                    prop_name.as_str(),
                    false,
                ))),
                PhpType::String,
            );
            bind_call_args(
                execute_data,
                &params,
                &[name_val, clone_val(&value)],
                &[None, None],
                &None,
                &[],
                &[false, false],
            );

            let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
            method_op_array.ops = ops;
            let saved_try_depth = execute_data.try_stack.len();
            let (_status, _return_val) =
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
            if let Some(er) = crate::engine::vm::exception_dispatch::propagate_after_call(
                execute_data,
                saved_try_depth,
            ) {
                return Ok(er);
            }
            return Ok(ExecResult::Continue);
        }
    }

    // Normal property assignment
    if is_var_ref(var_name_val) {
        if let PhpValue::String(ref s) = var_name_val.value {
            let vname = s.as_str();
            let name = vname.strip_prefix('$').unwrap_or(vname);
            let mut obj_val = execute_data.get_var(name);
            if let PhpValue::Object(ref mut obj) = obj_val.value {
                obj.properties.insert(prop_name.as_str().to_string(), value);
            }
            execute_data.set_var(name, obj_val);
        }
    } else if is_temp_ref(var_name_val)
        && let PhpValue::Long(slot_idx) = var_name_val.value
    {
        let slot = slot_idx as usize;
        let mut obj_val = execute_data.get_temp(slot);
        if let PhpValue::Object(ref mut obj) = obj_val.value {
            obj.properties.insert(prop_name.as_str().to_string(), value);
        }
        execute_data.set_temp(slot, obj_val);
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_assign_static_prop(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    let class_name_val = resolve_operand(&op.op1, execute_data);
    let mut class_name = crate::engine::operators::zval_get_string(&class_name_val)
        .as_str()
        .to_string();
    if class_name == "static" || class_name == "self" {
        class_name = execute_data.called_class.clone().unwrap_or_default();
    }
    let prop_name_val = resolve_operand(&op.op2, execute_data);
    let prop_name_raw = crate::engine::operators::zval_get_string(&prop_name_val);
    let prop_name_str = prop_name_raw.as_str();
    let prop_name = prop_name_str.strip_prefix('$').unwrap_or(prop_name_str);
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

        // Built-in Throwable getters: $e->getMessage(), getCode(), …
        if crate::engine::vm::exception_dispatch::is_standard_throwable(&class_name)
            && crate::engine::vm::exception_dispatch::execute_throwable_getter(
                &class_name,
                method_name.as_str(),
                &obj_val,
            )
            .map(|getter_result| {
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, getter_result);
                }
                true
            })
            .unwrap_or(false)
        {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            execute_data.call_args.drain(base..);
            return Ok(ExecResult::Continue);
        }

        // Handle built-in reflection classes
        if class_name == "ReflectionClass"
            || class_name == "ReflectionMethod"
            || class_name == "ReflectionProperty"
            || class_name == "ReflectionFunction"
            || class_name == "ReflectionParameter"
        {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            execute_data.called_class = Some(class_name.clone());
            execute_data.set_var("this", clone_val(&obj_val));

            let ret = match class_name.as_str() {
                "ReflectionClass" => {
                    crate::engine::vm::reflection::execute_reflection_class_method(
                        method_name.as_str(),
                        &args,
                        execute_data,
                    )
                }
                "ReflectionMethod" => crate::engine::vm::reflection::execute_reflection_method(
                    method_name.as_str(),
                    &args,
                    execute_data,
                ),
                "ReflectionProperty" => crate::engine::vm::reflection::execute_reflection_property(
                    method_name.as_str(),
                    &args,
                    execute_data,
                ),
                "ReflectionFunction" => crate::engine::vm::reflection::execute_reflection_function(
                    method_name.as_str(),
                    &args,
                    execute_data,
                ),
                "ReflectionParameter" => {
                    crate::engine::vm::reflection::execute_reflection_parameter(
                        method_name.as_str(),
                        &args,
                        execute_data,
                    )
                }
                _ => None,
            };

            // Copy modified $this back
            let this_val = execute_data.get_var("this");
            if is_temp_ref(&op.op2) {
                if let PhpValue::Long(slot_idx) = op.op2.value {
                    execute_data.set_temp(slot_idx as usize, this_val);
                }
            } else if is_var_ref(&op.op2)
                && let PhpValue::String(ref s) = op.op2.value
            {
                let vname = s.as_str();
                let name = vname.strip_prefix('$').unwrap_or(vname);
                execute_data.set_var(name, this_val);
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

        // Built-in DateTime methods: format(), __construct()
        if class_name == "DateTime" || class_name == "DateTimeImmutable" {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let mn = method_name.as_str();
            if mn == "__construct" {
                // DateTime constructor: parse time string, store timestamp
                let time_str = args
                    .first()
                    .map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| crate::engine::string::string_init("now", false));
                let ts = crate::php::datetime::parse_datetime_string(time_str.as_str())
                    .unwrap_or_else(|| {
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0)
                    });
                // Second argument: DateTimeZone object or timezone string
                let (tz_offset, tz_name) = if let Some(tz_arg) = args.get(1) {
                    if let PhpValue::Object(ref o) = tz_arg.value {
                        let name = o.properties.get("name")
                            .map(crate::engine::operators::zval_get_string)
                            .map(|s| s.as_str().to_string())
                            .unwrap_or_else(|| "UTC".to_string());
                        (crate::php::datetime::timezone_offset_at(&name, ts), name)
                    } else {
                        let name = crate::engine::operators::zval_get_string(tz_arg).as_str().to_string();
                        (crate::php::datetime::timezone_offset_at(&name, ts), name)
                    }
                } else {
                    (0, "UTC".to_string())
                };
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert(
                        "timestamp".to_string(),
                        Val::new(PhpValue::Long(ts), PhpType::Long),
                    );
                    o.properties.insert(
                        "timezone_offset".to_string(),
                        Val::new(PhpValue::Long(tz_offset), PhpType::Long),
                    );
                    o.properties.insert(
                        "timezone_name".to_string(),
                        Val::new(
                            PhpValue::String(Box::new(crate::engine::string::string_init(&tz_name, false))),
                            PhpType::String,
                        ),
                    );
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value {
                        execute_data.set_temp(slot_idx as usize, updated);
                    }
                } else if is_var_ref(&op.op2)
                    && let PhpValue::String(ref s) = op.op2.value
                {
                    let vname = s.as_str();
                    let name = vname.strip_prefix('$').unwrap_or(vname);
                    execute_data.set_var(name, updated);
                }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
                }
                return Ok(ExecResult::Continue);
            }
            if mn == "format" {
                let fmt = args
                    .first()
                    .map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| crate::engine::string::string_init("", false));
                let ts = obj
                    .properties
                    .get("timestamp")
                    .map(crate::engine::operators::zval_get_long)
                    .unwrap_or(0);
                let tz_offset = obj
                    .properties
                    .get("timezone_offset")
                    .map(crate::engine::operators::zval_get_long)
                    .unwrap_or(0);
                let tz_name = obj
                    .properties
                    .get("timezone_name")
                    .map(crate::engine::operators::zval_get_string)
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_else(|| "UTC".to_string());
                // Apply timezone offset to the timestamp before formatting
                let dt = crate::php::datetime::timestamp_to_datetime_struct((ts + tz_offset) as u64);
                let formatted = crate::php::datetime::format_datetime_tz(fmt.as_str(), &dt, tz_offset, &tz_name);
                let result = Val::new(
                    PhpValue::String(Box::new(crate::engine::string::string_init(
                        &formatted, false,
                    ))),
                    PhpType::String,
                );
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, result);
                }
                return Ok(ExecResult::Continue);
            }
            if mn == "getTimestamp" || mn == "getTimeStamp" {
                let ts = obj
                    .properties
                    .get("timestamp")
                    .map(clone_val)
                    .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Long));
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, ts);
                }
                return Ok(ExecResult::Continue);
            }
            if mn == "getTimezone" || mn == "getTimeZone" {
                let tz_name = obj
                    .properties
                    .get("timezone_name")
                    .map(crate::engine::operators::zval_get_string)
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_else(|| "UTC".to_string());
                let mut tz_obj = crate::engine::types::PhpObject::new("DateTimeZone");
                tz_obj.properties.insert(
                    "name".to_string(),
                    Val::new(
                        PhpValue::String(Box::new(crate::engine::string::string_init(&tz_name, false))),
                        PhpType::String,
                    ),
                );
                let result = Val::new(PhpValue::Object(Box::new(tz_obj)), PhpType::Object);
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, result);
                }
                return Ok(ExecResult::Continue);
            }
            if mn == "setTimezone" {
                let tz_name = args
                    .first()
                    .and_then(|v| {
                        if let PhpValue::Object(ref o) = v.value {
                            o.properties.get("name").map(crate::engine::operators::zval_get_string)
                        } else {
                            Some(crate::engine::operators::zval_get_string(v))
                        }
                    })
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_else(|| "UTC".to_string());
                let tz_offset = {
                    let ts = obj.properties.get("timestamp")
                        .map(crate::engine::operators::zval_get_long)
                        .unwrap_or(0);
                    crate::php::datetime::timezone_offset_at(&tz_name, ts)
                };
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert(
                        "timezone_offset".to_string(),
                        Val::new(PhpValue::Long(tz_offset), PhpType::Long),
                    );
                    o.properties.insert(
                        "timezone_name".to_string(),
                        Val::new(
                            PhpValue::String(Box::new(crate::engine::string::string_init(&tz_name, false))),
                            PhpType::String,
                        ),
                    );
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value {
                        execute_data.set_temp(slot_idx as usize, updated);
                    }
                } else if is_var_ref(&op.op2)
                    && let PhpValue::String(ref s) = op.op2.value
                {
                    let vname = s.as_str();
                    let name = vname.strip_prefix('$').unwrap_or(vname);
                    execute_data.set_var(name, updated);
                }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
                }
                return Ok(ExecResult::Continue);
            }
            if mn == "diff" {
                // DateTime::diff(DateTime $other): DateInterval
                let ts1 = obj
                    .properties
                    .get("timestamp")
                    .map(crate::engine::operators::zval_get_long)
                    .unwrap_or(0);
                let ts2 = args
                    .first()
                    .and_then(|v| {
                        if let PhpValue::Object(ref o) = v.value {
                            o.properties.get("timestamp")
                        } else {
                            None
                        }
                    })
                    .map(crate::engine::operators::zval_get_long)
                    .unwrap_or(0);
                let (y, m, d, h, i, s, days, invert) = crate::php::datetime::compute_diff(ts1, ts2);
                let mut interval_obj = crate::engine::types::PhpObject::new("DateInterval");
                interval_obj
                    .properties
                    .insert("y".to_string(), Val::new(PhpValue::Long(y), PhpType::Long));
                interval_obj
                    .properties
                    .insert("m".to_string(), Val::new(PhpValue::Long(m), PhpType::Long));
                interval_obj
                    .properties
                    .insert("d".to_string(), Val::new(PhpValue::Long(d), PhpType::Long));
                interval_obj
                    .properties
                    .insert("h".to_string(), Val::new(PhpValue::Long(h), PhpType::Long));
                interval_obj
                    .properties
                    .insert("i".to_string(), Val::new(PhpValue::Long(i), PhpType::Long));
                interval_obj
                    .properties
                    .insert("s".to_string(), Val::new(PhpValue::Long(s), PhpType::Long));
                interval_obj.properties.insert(
                    "days".to_string(),
                    Val::new(PhpValue::Long(days), PhpType::Long),
                );
                interval_obj.properties.insert(
                    "invert".to_string(),
                    Val::new(PhpValue::Long(if invert { 1 } else { 0 }), PhpType::Long),
                );
                let result = Val::new(PhpValue::Object(Box::new(interval_obj)), PhpType::Object);
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, result);
                }
                return Ok(ExecResult::Continue);
            }
        }

        // Built-in DateTimeZone methods
        if class_name == "DateTimeZone" {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let mn = method_name.as_str();
            if mn == "__construct" {
                let tz_name = args
                    .first()
                    .map(crate::engine::operators::zval_get_string)
                    .map(|s| s.as_str().to_string())
                    .unwrap_or_else(|| "UTC".to_string());
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert(
                        "name".to_string(),
                        Val::new(
                            PhpValue::String(Box::new(crate::engine::string::string_init(&tz_name, false))),
                            PhpType::String,
                        ),
                    );
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value {
                        execute_data.set_temp(slot_idx as usize, updated);
                    }
                } else if is_var_ref(&op.op2)
                    && let PhpValue::String(ref s) = op.op2.value
                {
                    let vname = s.as_str();
                    let name = vname.strip_prefix('$').unwrap_or(vname);
                    execute_data.set_var(name, updated);
                }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
                }
                return Ok(ExecResult::Continue);
            }
            if mn == "getName" {
                let name = obj
                    .properties
                    .get("name")
                    .map(clone_val)
                    .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null));
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, name);
                }
                return Ok(ExecResult::Continue);
            }
        }

        // Built-in Generator methods: current, next, send, getReturn, valid, rewind, key
        if class_name == "Generator" {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let m_args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let mn = method_name.as_str();
            let state = obj.properties.get("__state")
                .map(crate::engine::operators::zval_get_long)
                .unwrap_or(0);
            let null_val = Val::new(PhpValue::Long(0), PhpType::Null);

            if mn == "valid" {
                let result = make_bool(state != 3);
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, result.clone());
                }
                return Ok(ExecResult::Continue);
            }
            if mn == "current" {
                let result = obj.properties.get("__current_value")
                    .map(clone_val).unwrap_or_else(|| clone_val(&null_val));
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, result.clone());
                }
                return Ok(ExecResult::Continue);
            }
            if mn == "key" {
                let result = obj.properties.get("__current_key")
                    .map(clone_val).unwrap_or_else(|| clone_val(&null_val));
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, result.clone());
                }
                return Ok(ExecResult::Continue);
            }
            if mn == "getReturn" {
                let result = obj.properties.get("__return_value")
                    .map(clone_val).unwrap_or_else(|| clone_val(&null_val));
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, result.clone());
                }
                return Ok(ExecResult::Continue);
            }
            if mn == "rewind" || mn == "next" || mn == "send" {
                // For send(), get the value to pass to the yield expression
                let send_val = if mn == "send" {
                    Some(if !m_args.is_empty() { clone_val(&m_args[0]) }
                         else { clone_val(&null_val) })
                } else {
                    None
                };

                // Get the generator's frame index
                let frame_idx = obj.properties.get("__frame_index")
                    .map(crate::engine::operators::zval_get_long)
                    .unwrap_or(0) as usize;

                if frame_idx >= execute_data.fiber_frames.len() {
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, clone_val(&null_val));
                    }
                    return Ok(ExecResult::Continue);
                }

                // Save current VM state (caller's state)
                let saved_op = execute_data.current_op;
                let saved_op_array = execute_data.op_array.take();
                let saved_temps = std::mem::take(&mut execute_data.temp_vars);
                let saved_symbol_table = execute_data.symbol_table.take();
                let saved_ref_caller_scope = execute_data.ref_caller_scope.take();
                let saved_call_arg_stack = std::mem::take(&mut execute_data.call_arg_stack);
                let saved_call_args = std::mem::take(&mut execute_data.call_args);
                let saved_call_arg_names = std::mem::take(&mut execute_data.call_arg_names);
                let saved_call_arg_by_ref = std::mem::take(&mut execute_data.call_arg_by_ref);
                let saved_global_imports = std::mem::take(&mut execute_data.global_imports);
                let saved_ref_bindings = std::mem::take(&mut execute_data.ref_param_bindings);
                let saved_script_dir = execute_data.current_script_dir.clone();
                let saved_try_stack = std::mem::take(&mut execute_data.try_stack);

                // Take the generator frame out
                let gen_frame = std::mem::replace(&mut execute_data.fiber_frames[frame_idx],
                    super::execute_data::FiberFrame {
                        op_array: super::opcodes::OpArray::new(String::new()),
                        current_op: 0,
                        temp_vars: Vec::new(),
                        symbol_table: None,
                        call_args: Vec::new(),
                        call_arg_stack: Vec::new(),
                        call_arg_names: Vec::new(),
                        call_arg_by_ref: Vec::new(),
                        ref_caller_scope: None,
                        ref_param_bindings: std::collections::HashMap::new(),
                        global_imports: std::collections::HashSet::new(),
                        try_stack: Vec::new(),
                    });

                // Install the generator's state into ExecuteData
                execute_data.op_array = Some(gen_frame.op_array);
                execute_data.current_op = gen_frame.current_op;
                execute_data.temp_vars = gen_frame.temp_vars;
                execute_data.symbol_table = gen_frame.symbol_table
                    .or_else(|| Some(crate::engine::types::PhpArray::new()));
                execute_data.ref_caller_scope = gen_frame.ref_caller_scope;
                execute_data.call_args = gen_frame.call_args;
                execute_data.call_arg_stack = gen_frame.call_arg_stack;
                execute_data.call_arg_names = gen_frame.call_arg_names;
                execute_data.call_arg_by_ref = gen_frame.call_arg_by_ref;
                execute_data.ref_param_bindings = gen_frame.ref_param_bindings;
                execute_data.global_imports = gen_frame.global_imports;
                execute_data.try_stack = gen_frame.try_stack;

                // If this is the first call (rewind/next from state 0), bind the parameters
                if state == 0 {
                    let param_str = obj.properties.get("__param_names")
                        .map(|v| crate::engine::operators::zval_get_string(v).as_str().to_string())
                        .unwrap_or_default();
                    let variadic = obj.properties.get("__variadic_param")
                        .map(|v| crate::engine::operators::zval_get_string(v).as_str().to_string())
                        .unwrap_or_default();
                    let variadic_opt: Option<String> = if variadic.is_empty() { None } else { Some(variadic.clone()) };
                    let ref_params_str = obj.properties.get("__ref_params")
                        .map(|v| crate::engine::operators::zval_get_string(v).as_str().to_string())
                        .unwrap_or_default();
                    let ref_params: Vec<bool> = if ref_params_str.is_empty() {
                        Vec::new()
                    } else {
                        ref_params_str.split(',').map(|s| s == "1").collect()
                    };
                    let param_names: Vec<String> = if param_str.is_empty() {
                        Vec::new()
                    } else {
                        param_str.split(',').map(|s| s.to_string()).collect()
                    };
                    let init_args = std::mem::take(&mut execute_data.call_args);
                    let init_arg_names = std::mem::take(&mut execute_data.call_arg_names);
                    let init_arg_by_ref = std::mem::take(&mut execute_data.call_arg_by_ref);
                    bind_call_args(
                        execute_data,
                        &param_names,
                        &init_args,
                        &init_arg_names,
                        &variadic_opt,
                        &ref_params,
                        &init_arg_by_ref,
                    );
                }

                // If send(), we need to set the sent value as the result of the
                // Yield opcode. The Yield opcode's result temp slot gets the sent value.
                // We do this by setting it in the temp_vars before resuming.
                if let Some(ref sv) = send_val {
                    // The Yield opcode's result slot is the temp var that will receive
                    // the sent value. We need to find it and set it.
                    // Since we can't easily know which temp slot the Yield uses,
                    // we'll store the send value and let the next Yield pick it up.
                    // For simplicity, we'll set it as a special var.
                    execute_data.set_var("__generator_send_value", clone_val(sv));
                }

                // Clear the yield flag before resuming
                execute_data.generator_yield_requested = None;

                // Resume execution
                let (_status, return_val) = super::execute::execute_ex_resume(execute_data);

                // Check if we yielded or finished
                let yielded = execute_data.generator_yield_requested.take();
                let yielded_key = execute_data.generator_yield_key.take();
                let new_state = if yielded.is_some() {
                    2 // suspended
                } else {
                    3 // terminated
                };

                // Extract the generator's current state
                let gen_op_array = execute_data.op_array.take().unwrap_or_else(|| super::opcodes::OpArray::new(String::new()));
                let gen_current_op = execute_data.current_op;
                let gen_temps = std::mem::take(&mut execute_data.temp_vars);
                let gen_symbol_table = execute_data.symbol_table.take();
                let gen_ref_caller_scope = execute_data.ref_caller_scope.take();
                let gen_call_args = std::mem::take(&mut execute_data.call_args);
                let gen_call_arg_stack = std::mem::take(&mut execute_data.call_arg_stack);
                let gen_call_arg_names = std::mem::take(&mut execute_data.call_arg_names);
                let gen_call_arg_by_ref = std::mem::take(&mut execute_data.call_arg_by_ref);
                let gen_ref_param_bindings = std::mem::take(&mut execute_data.ref_param_bindings);
                let gen_global_imports = std::mem::take(&mut execute_data.global_imports);
                let gen_try_stack = std::mem::take(&mut execute_data.try_stack);

                // Save the generator frame back
                execute_data.fiber_frames[frame_idx] = super::execute_data::FiberFrame {
                    op_array: gen_op_array,
                    current_op: gen_current_op,
                    temp_vars: gen_temps,
                    symbol_table: gen_symbol_table,
                    call_args: gen_call_args,
                    call_arg_stack: gen_call_arg_stack,
                    call_arg_names: gen_call_arg_names,
                    call_arg_by_ref: gen_call_arg_by_ref,
                    ref_caller_scope: gen_ref_caller_scope,
                    ref_param_bindings: gen_ref_param_bindings,
                    global_imports: gen_global_imports,
                    try_stack: gen_try_stack,
                };

                // Restore caller's VM state
                execute_data.temp_vars = saved_temps;
                execute_data.op_array = saved_op_array;
                execute_data.current_op = saved_op;
                execute_data.symbol_table = saved_symbol_table;
                execute_data.ref_caller_scope = saved_ref_caller_scope;
                execute_data.call_arg_stack = saved_call_arg_stack;
                execute_data.call_args = saved_call_args;
                execute_data.call_arg_names = saved_call_arg_names;
                execute_data.call_arg_by_ref = saved_call_arg_by_ref;
                execute_data.global_imports = saved_global_imports;
                execute_data.ref_param_bindings = saved_ref_bindings;
                execute_data.current_script_dir = saved_script_dir;
                execute_data.try_stack = saved_try_stack;

                // Update the Generator object's properties
                let mut updated = crate::engine::types::PhpObject::new("Generator");
                updated.properties = obj.properties.clone();
                updated.properties.insert("__state".to_string(),
                    Val::new(PhpValue::Long(new_state), PhpType::Long));
                if let Some(ref yv) = yielded {
                    updated.properties.insert("__current_value".to_string(), clone_val(yv));
                    // Use the yielded key if provided, otherwise auto-increment
                    if let Some(ref yk) = yielded_key {
                        if yk.get_type() != PhpType::Null {
                            updated.properties.insert("__current_key".to_string(), clone_val(yk));
                        } else {
                            let old_key = obj.properties.get("__current_key")
                                .map(crate::engine::operators::zval_get_long)
                                .unwrap_or(-1);
                            updated.properties.insert("__current_key".to_string(),
                                Val::new(PhpValue::Long(old_key + 1), PhpType::Long));
                        }
                    } else {
                        let old_key = obj.properties.get("__current_key")
                            .map(crate::engine::operators::zval_get_long)
                            .unwrap_or(-1);
                        updated.properties.insert("__current_key".to_string(),
                            Val::new(PhpValue::Long(old_key + 1), PhpType::Long));
                    }
                }
                if let Some(rv) = return_val {
                    updated.properties.insert("__return_value".to_string(), rv);
                }

                // Write back the updated object
                if is_var_ref(&op.op2)
                    && let PhpValue::String(ref name) = op.op2.value
                {
                    let n = name.as_str();
                    let clean = n.strip_prefix('$').unwrap_or(n);
                    execute_data.set_var(clean, Val::new(PhpValue::Object(Box::new(updated)), PhpType::Object));
                } else if is_temp_ref(&op.op2)
                    && let PhpValue::Long(idx) = op.op2.value
                {
                    execute_data.set_temp(idx as usize, Val::new(PhpValue::Object(Box::new(updated)), PhpType::Object));
                }

                // Return the yielded value (for send()) or null
                let result = yielded.unwrap_or_else(|| clone_val(&null_val));
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, result.clone());
                }
                return Ok(ExecResult::Continue);
            }
            // Unknown method — fall through
        }

        // Built-in Fiber methods: __construct, start, resume, getReturn,
        // isStarted, isSuspended, isTerminated, isRunning
        if class_name == "Fiber" {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let mn = method_name.as_str();

            if mn == "__construct" {
                let callable = args.first().map(clone_val).unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null));
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert("__callable".to_string(), callable);
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value {
                        execute_data.set_temp(slot_idx as usize, updated);
                    }
                } else if is_var_ref(&op.op2)
                    && let PhpValue::String(ref s) = op.op2.value
                {
                    let vname = s.as_str();
                    let name = vname.strip_prefix('$').unwrap_or(vname);
                    execute_data.set_var(name, updated);
                }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
                }
                return Ok(ExecResult::Continue);
            }

            // State query methods
            if mn == "isStarted" || mn == "isSuspended" || mn == "isTerminated" || mn == "isRunning" {
                let state = obj.properties.get("__state")
                    .map(crate::engine::operators::zval_get_long)
                    .unwrap_or(0);
                let result = match mn {
                    "isStarted" => Val::new(PhpValue::Long(if state >= 1 { 1 } else { 0 }), if state >= 1 { PhpType::True } else { PhpType::False }),
                    "isSuspended" => Val::new(PhpValue::Long(if state == 2 { 1 } else { 0 }), if state == 2 { PhpType::True } else { PhpType::False }),
                    "isTerminated" => Val::new(PhpValue::Long(if state == 3 { 1 } else { 0 }), if state == 3 { PhpType::True } else { PhpType::False }),
                    "isRunning" => Val::new(PhpValue::Long(0), PhpType::False),
                    _ => unreachable!(),
                };
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, result);
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "getReturn" {
                let ret = obj.properties.get("__return_value")
                    .map(clone_val)
                    .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null));
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, ret);
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "start" || mn == "resume" {
                let state = obj.properties.get("__state")
                    .map(crate::engine::operators::zval_get_long)
                    .unwrap_or(0);
                // start: state must be pending(0); resume: state must be suspended(2)
                if (mn == "start" && state != 0) || (mn == "resume" && state != 2) {
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
                    }
                    return Ok(ExecResult::Continue);
                }

                let callable = obj.properties.get("__callable")
                    .map(clone_val)
                    .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null));
                let frame_index = obj.properties.get("__frame_index")
                    .map(crate::engine::operators::zval_get_long)
                    .unwrap_or(-1);

                // Save caller VM state (mirrors invoke_user_function)
                let saved_op = execute_data.current_op;
                let saved_op_array = execute_data.op_array.take();
                let saved_temps = std::mem::take(&mut execute_data.temp_vars);
                let saved_ref_caller_scope = execute_data.ref_caller_scope.take();
                let mut saved_symbol_table = execute_data.symbol_table.take();
                let saved_call_arg_stack = std::mem::take(&mut execute_data.call_arg_stack);
                let saved_call_args = std::mem::take(&mut execute_data.call_args);
                let saved_call_arg_names = std::mem::take(&mut execute_data.call_arg_names);
                let saved_call_arg_by_ref = std::mem::take(&mut execute_data.call_arg_by_ref);
                let mut saved_global_imports = std::mem::take(&mut execute_data.global_imports);
                let mut saved_ref_bindings = std::mem::take(&mut execute_data.ref_param_bindings);
                let saved_script_dir = execute_data.current_script_dir.clone();
                let saved_try_depth = execute_data.try_stack.len();

                if execute_data.global_script_table.is_none()
                    && let Some(ref saved) = saved_symbol_table
                {
                    execute_data.global_script_table = Some(ExecuteData::clone_php_array(saved));
                }

                let suspend_value: Option<Val>;
                // Updated fiber object — written back AFTER caller state restoration
                // to avoid being overwritten by saved_temps.
                let mut fiber_updated: Option<Val> = None;

                if mn == "start" {
                    // Set up the callable for the first time
                    let cb_name = crate::engine::vm::callable::callable_name(&callable);
                    let func_data: Option<(Vec<String>, Option<String>, super::opcodes::OpArray)> =
                        execute_data.function_table.as_ref()
                            .and_then(|ft| ft.downcast_ref::<crate::engine::compile::function_table::FunctionTable>())
                            .and_then(|ft| cb_name.as_deref().and_then(|n| ft.lookup_function(n)))
                            .map(|func_op_array| {
                                let param_names: Vec<String> = func_op_array.vars.iter()
                                    .map(|v| match &v.value {
                                        PhpValue::String(s) => s.as_str().strip_prefix('$').unwrap_or(s.as_str()).to_string(),
                                        _ => String::new(),
                                    })
                                    .collect();
                                let variadic = func_op_array.variadic_param.clone();
                                let mut cloned = super::opcodes::OpArray::with_capacity(
                                    func_op_array.ops.len(),
                                    func_op_array.filename.clone().unwrap_or_default(),
                                );
                                cloned.function_name = func_op_array.function_name.clone();
                                cloned.ref_params = func_op_array.ref_params.clone();
                                cloned.variadic_param = func_op_array.variadic_param.clone();
                                cloned.is_generator = func_op_array.is_generator;
                                for op in &func_op_array.ops {
                                    cloned.add_op(super::opcodes::Op::new(
                                        op.opcode, clone_val(&op.op1), clone_val(&op.op2),
                                        clone_val(&op.result), op.extended_value,
                                    ));
                                }
                                (param_names, variadic, cloned)
                            });

                    if let Some((param_names, variadic_param, func_op_array)) = func_data {
                        execute_data.ref_caller_scope = saved_symbol_table.take();
                        execute_data.symbol_table = Some(crate::engine::types::PhpArray::new());
                        let arg_names: Vec<Option<String>> = vec![None; args.len()];
                        let arg_by_ref: Vec<bool> = vec![false; args.len()];
                        let ref_params = func_op_array.ref_params.clone();
                        super::dispatch_handlers::bind_call_args(
                            execute_data, &param_names, &args, &arg_names,
                            &variadic_param, &ref_params, &arg_by_ref,
                        );
                        let (_status, return_val) = super::execute::execute_ex_returning(execute_data, &func_op_array);
                        if execute_data.fiber_suspend_requested.is_some() {
                            // Suspended: save the fiber frame
                            // Note: ref_caller_scope, global_imports, and ref_param_bindings
                            // belong to the CALLER, not the fiber — don't save them.
                            let frame = crate::engine::vm::execute_data::FiberFrame {
                                op_array: execute_data.op_array.take().unwrap_or(func_op_array),
                                current_op: execute_data.current_op,
                                temp_vars: std::mem::take(&mut execute_data.temp_vars),
                                symbol_table: execute_data.symbol_table.take(),
                                call_args: std::mem::take(&mut execute_data.call_args),
                                call_arg_stack: std::mem::take(&mut execute_data.call_arg_stack),
                                call_arg_names: std::mem::take(&mut execute_data.call_arg_names),
                                call_arg_by_ref: std::mem::take(&mut execute_data.call_arg_by_ref),
                                ref_caller_scope: None,
                                ref_param_bindings: std::collections::HashMap::new(),
                                global_imports: std::collections::HashSet::new(),
                                try_stack: std::mem::take(&mut execute_data.try_stack),
                            };
                            let idx = execute_data.fiber_frames.len();
                            execute_data.fiber_frames.push(frame);
                            suspend_value = execute_data.fiber_suspend_requested.take();
                            let mut updated = clone_val(&obj_val);
                            if let PhpValue::Object(ref mut o) = updated.value {
                                o.properties.insert("__state".to_string(), Val::new(PhpValue::Long(2), PhpType::Long));
                                o.properties.insert("__frame_index".to_string(), Val::new(PhpValue::Long(idx as i64), PhpType::Long));
                            }
                            fiber_updated = Some(updated);
                        } else {
                            // Terminated: capture return value
                            suspend_value = None;
                            let mut updated = clone_val(&obj_val);
                            if let PhpValue::Object(ref mut o) = updated.value {
                                o.properties.insert("__state".to_string(), Val::new(PhpValue::Long(3), PhpType::Long));
                                o.properties.insert("__return_value".to_string(), return_val.unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null)));
                            }
                            fiber_updated = Some(updated);
                        }
                    } else {
                        // Built-in callable or unresolved — run via invoke_callable
                        execute_data.symbol_table = Some(crate::engine::types::PhpArray::new());
                        let _ = crate::engine::vm::callable::invoke_callable(execute_data, &callable, &args);
                        suspend_value = execute_data.fiber_suspend_requested.take();
                        let mut updated = clone_val(&obj_val);
                        if let PhpValue::Object(ref mut o) = updated.value {
                            o.properties.insert("__state".to_string(), Val::new(PhpValue::Long(3), PhpType::Long));
                        }
                        fiber_updated = Some(updated);
                    }
                } else {
                    // resume: restore the fiber frame and continue
                    let idx = frame_index as usize;
                    if idx >= execute_data.fiber_frames.len() {
                        // Frame not found — can't resume
                        suspend_value = None;
                    } else {
                        // Take ownership of the frame (no Clone needed)
                        let empty_frame = crate::engine::vm::execute_data::FiberFrame {
                            op_array: super::opcodes::OpArray::with_capacity(0, String::new()),
                            current_op: 0,
                            temp_vars: Vec::new(),
                            symbol_table: None,
                            call_args: Vec::new(),
                            call_arg_stack: Vec::new(),
                            call_arg_names: Vec::new(),
                            call_arg_by_ref: Vec::new(),
                            ref_caller_scope: None,
                            ref_param_bindings: std::collections::HashMap::new(),
                            global_imports: std::collections::HashSet::new(),
                            try_stack: Vec::new(),
                        };
                        let frame_slot = std::mem::replace(&mut execute_data.fiber_frames[idx], empty_frame);
                        let crate::engine::vm::execute_data::FiberFrame {
                            op_array,
                            current_op,
                            temp_vars,
                            symbol_table,
                            call_args,
                            call_arg_stack,
                            call_arg_names,
                            call_arg_by_ref,
                            ref_caller_scope: _ref_caller_scope,
                            ref_param_bindings: _ref_param_bindings,
                            global_imports: _global_imports,
                            try_stack,
                        } = frame_slot;

                        execute_data.op_array = Some(op_array);
                        execute_data.current_op = current_op;
                        execute_data.temp_vars = temp_vars;
                        execute_data.symbol_table = symbol_table;
                        execute_data.call_args = call_args;
                        execute_data.call_arg_stack = call_arg_stack;
                        execute_data.call_arg_names = call_arg_names;
                        execute_data.call_arg_by_ref = call_arg_by_ref;
                        // ref_caller_scope, ref_param_bindings, global_imports belong to
                        // the caller — set from saved values, not the frame.
                        execute_data.ref_caller_scope = saved_symbol_table.take();
                        execute_data.ref_param_bindings = std::mem::take(&mut saved_ref_bindings);
                        execute_data.global_imports = std::mem::take(&mut saved_global_imports);
                        execute_data.try_stack = try_stack;

                        // Place the resume value in the result slot of the DoFCall
                        // that called Fiber::suspend() (at current_op - 1)
                        let resume_value = args.first().map(clone_val)
                            .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null));
                        let op_array_ref = execute_data.op_array.as_ref().unwrap();
                        if execute_data.current_op > 0 {
                            let prev_op = &op_array_ref.ops[execute_data.current_op - 1];
                            if (prev_op.opcode == super::opcodes::Opcode::DoFCall
                                || prev_op.opcode == super::opcodes::Opcode::DoStaticCall)
                                && let Some(slot) = super::execute_data::result_slot(prev_op)
                            {
                                execute_data.ensure_temp_slots(slot + 1);
                                execute_data.temp_vars[slot] = resume_value;
                            }
                        }

                        // Continue execution using execute_ex_resume (doesn't reset current_op)
                        // execute_ex_resume reads from execute_data.op_array, so keep it installed.
                        let (_status, return_val) = super::execute::execute_ex_resume(execute_data);
                        let op_array_for_exec = execute_data.op_array.take().unwrap_or(super::opcodes::OpArray::with_capacity(0, String::new()));

                        if execute_data.fiber_suspend_requested.is_some() {
                            // Suspended again: save the updated frame
                            // Note: ref_caller_scope, global_imports, ref_param_bindings
                            // belong to the CALLER, not the fiber — don't save them.
                            let new_frame = crate::engine::vm::execute_data::FiberFrame {
                                op_array: execute_data.op_array.take().unwrap_or(op_array_for_exec),
                                current_op: execute_data.current_op,
                                temp_vars: std::mem::take(&mut execute_data.temp_vars),
                                symbol_table: execute_data.symbol_table.take(),
                                call_args: std::mem::take(&mut execute_data.call_args),
                                call_arg_stack: std::mem::take(&mut execute_data.call_arg_stack),
                                call_arg_names: std::mem::take(&mut execute_data.call_arg_names),
                                call_arg_by_ref: std::mem::take(&mut execute_data.call_arg_by_ref),
                                ref_caller_scope: None,
                                ref_param_bindings: std::collections::HashMap::new(),
                                global_imports: std::collections::HashSet::new(),
                                try_stack: std::mem::take(&mut execute_data.try_stack),
                            };
                            execute_data.fiber_frames[idx] = new_frame;
                            suspend_value = execute_data.fiber_suspend_requested.take();
                            let mut updated = clone_val(&obj_val);
                            if let PhpValue::Object(ref mut o) = updated.value {
                                o.properties.insert("__state".to_string(), Val::new(PhpValue::Long(2), PhpType::Long));
                            }
                            fiber_updated = Some(updated);
                        } else {
                            // Terminated: capture return value, free the frame
                            execute_data.fiber_frames[idx] = crate::engine::vm::execute_data::FiberFrame {
                                op_array: super::opcodes::OpArray::with_capacity(0, String::new()),
                                current_op: 0,
                                temp_vars: Vec::new(),
                                symbol_table: None,
                                call_args: Vec::new(),
                                call_arg_stack: Vec::new(),
                                call_arg_names: Vec::new(),
                                call_arg_by_ref: Vec::new(),
                                ref_caller_scope: None,
                                ref_param_bindings: std::collections::HashMap::new(),
                                global_imports: std::collections::HashSet::new(),
                                try_stack: Vec::new(),
                            };
                            suspend_value = None;
                            let mut updated = clone_val(&obj_val);
                            if let PhpValue::Object(ref mut o) = updated.value {
                                o.properties.insert("__state".to_string(), Val::new(PhpValue::Long(3), PhpType::Long));
                                o.properties.insert("__return_value".to_string(), return_val.unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null)));
                                o.properties.insert("__frame_index".to_string(), Val::new(PhpValue::Long(-1), PhpType::Long));
                            }
                            fiber_updated = Some(updated);
                        }
                    }
                }

                // Restore caller VM state
                // ref_caller_scope still holds the caller's symbol table (not saved into frame)
                execute_data.symbol_table = execute_data.ref_caller_scope.take();
                if let Some(mut saved) = execute_data.symbol_table.take() {
                    execute_data.merge_globals_into(&mut saved);
                    execute_data.symbol_table = Some(saved);
                }
                execute_data.ref_caller_scope = saved_ref_caller_scope;
                execute_data.global_imports = std::mem::take(&mut saved_global_imports);
                execute_data.ref_param_bindings = std::mem::take(&mut saved_ref_bindings);
                execute_data.temp_vars = saved_temps;
                execute_data.op_array = saved_op_array;
                execute_data.current_op = saved_op;
                execute_data.call_arg_stack = saved_call_arg_stack;
                execute_data.call_args = saved_call_args;
                execute_data.call_arg_names = saved_call_arg_names;
                execute_data.call_arg_by_ref = saved_call_arg_by_ref;
                execute_data.current_script_dir = saved_script_dir;
                execute_data.try_stack.truncate(saved_try_depth);

                // Write back the updated fiber object AFTER restoring caller state
                // (otherwise saved_temps restoration overwrites it)
                if let Some(updated) = fiber_updated {
                    if is_temp_ref(&op.op2) {
                        if let PhpValue::Long(slot_idx) = op.op2.value {
                            execute_data.set_temp(slot_idx as usize, updated);
                        }
                    } else if is_var_ref(&op.op2)
                        && let PhpValue::String(ref s) = op.op2.value
                    {
                        let vname = s.as_str();
                        let name = vname.strip_prefix('$').unwrap_or(vname);
                        execute_data.set_var(name, updated);
                    }
                }

                // Return the suspend value (or null if terminated)
                let result = suspend_value.unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null));
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, result);
                }
                return Ok(ExecResult::Continue);
            }
        }

        // Built-in ArrayIterator methods
        if class_name == "ArrayIterator" {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let mn = method_name.as_str();
            // Helper: get the storage array (clone-modify-set pattern)
            let storage = obj
                .properties
                .get("__storage")
                .map(clone_val)
                .unwrap_or_else(|| {
                    Val::new(PhpValue::Array(Box::default()), PhpType::Array)
                });
            let index = obj
                .properties
                .get("__index")
                .map(crate::engine::operators::zval_get_long)
                .unwrap_or(0);

            if mn == "__construct" {
                // Set storage from constructor argument (if provided)
                if let Some(first) = args.first() {
                    let new_storage = if let PhpValue::Array(_) = first.value {
                        clone_val(first)
                    } else {
                        let mut arr = PhpArray::new();
                        let _ = crate::engine::hash::hash_add_or_update(
                            &mut arr,
                            None,
                            0,
                            clone_val(first),
                            0,
                        );
                        Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array)
                    };
                    let mut updated = clone_val(&obj_val);
                    if let PhpValue::Object(ref mut o) = updated.value {
                        o.properties.insert("__storage".to_string(), new_storage);
                        o.properties.insert(
                            "__index".to_string(),
                            Val::new(PhpValue::Long(0), PhpType::Long),
                        );
                    }
                    if is_temp_ref(&op.op2) {
                        if let PhpValue::Long(slot_idx) = op.op2.value {
                            execute_data.set_temp(slot_idx as usize, updated);
                        }
                    } else if is_var_ref(&op.op2)
                        && let PhpValue::String(ref s) = op.op2.value
                    {
                        let vname = s.as_str();
                        let name = vname.strip_prefix('$').unwrap_or(vname);
                        execute_data.set_var(name, updated);
                    }
                }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(
                        slot,
                        Val::new(PhpValue::Long(0), PhpType::Null),
                    );
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "count" {
                let cnt = if let PhpValue::Array(ref arr) = storage.value {
                    arr.ar_data.len() as i64
                } else {
                    0
                };
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(cnt), PhpType::Long));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "valid" {
                let valid = if let PhpValue::Array(ref arr) = storage.value {
                    (index as usize) < arr.ar_data.len()
                } else {
                    false
                };
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(
                        slot,
                        Val::new(
                            PhpValue::Long(if valid { 1 } else { 0 }),
                            if valid { PhpType::True } else { PhpType::False },
                        ),
                    );
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "current" {
                let val = if let PhpValue::Array(ref arr) = storage.value {
                    arr.ar_data.get(index as usize).map(|b| clone_val(&b.val))
                } else {
                    None
                };
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(
                        slot,
                        val.unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null)),
                    );
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "key" {
                let key = if let PhpValue::Array(ref arr) = storage.value {
                    arr.ar_data.get(index as usize).and_then(|b| {
                        b.key.as_ref().map(|s| {
                            Val::new(
                                PhpValue::String(Box::new(
                                    crate::engine::string::string_init(s.as_str(), false),
                                )),
                                PhpType::String,
                            )
                        })
                    })
                } else {
                    None
                };
                let key_val = key.unwrap_or_else(|| {
                    if let PhpValue::Array(ref arr) = storage.value {
                        arr.ar_data
                            .get(index as usize)
                            .map(|b| Val::new(PhpValue::Long(b.h as i64), PhpType::Long))
                    } else {
                        None
                    }
                    .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null))
                });
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, key_val);
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "next" {
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert(
                        "__index".to_string(),
                        Val::new(PhpValue::Long(index + 1), PhpType::Long),
                    );
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value {
                        execute_data.set_temp(slot_idx as usize, updated);
                    }
                } else if is_var_ref(&op.op2)
                    && let PhpValue::String(ref s) = op.op2.value
                {
                    let vname = s.as_str();
                    let name = vname.strip_prefix('$').unwrap_or(vname);
                    execute_data.set_var(name, updated);
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "rewind" {
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert(
                        "__index".to_string(),
                        Val::new(PhpValue::Long(0), PhpType::Long),
                    );
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value {
                        execute_data.set_temp(slot_idx as usize, updated);
                    }
                } else if is_var_ref(&op.op2)
                    && let PhpValue::String(ref s) = op.op2.value
                {
                    let vname = s.as_str();
                    let name = vname.strip_prefix('$').unwrap_or(vname);
                    execute_data.set_var(name, updated);
                }
                return Ok(ExecResult::Continue);
            }

            // ArrayAccess methods
            if mn == "offsetExists" {
                let exists = if let PhpValue::Array(ref arr) = storage.value {
                    if let Some(key_val) = args.first() {
                        if let PhpValue::Long(k) = key_val.value {
                            crate::engine::hash::hash_index_find(arr, k as u64).is_some()
                        } else if let PhpValue::String(ref s) = key_val.value {
                            let key = crate::engine::string::string_init(s.as_str(), false);
                            crate::engine::hash::hash_find(arr, &key).is_some()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(
                        slot,
                        Val::new(
                            PhpValue::Long(if exists { 1 } else { 0 }),
                            if exists { PhpType::True } else { PhpType::False },
                        ),
                    );
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "offsetGet" {
                let val = if let PhpValue::Array(ref arr) = storage.value {
                    if let Some(key_val) = args.first() {
                        if let PhpValue::Long(k) = key_val.value {
                            crate::engine::hash::hash_index_find(arr, k as u64)
                                .map(clone_val)
                        } else if let PhpValue::String(ref s) = key_val.value {
                            let key =
                                crate::engine::string::string_init(s.as_str(), false);
                            crate::engine::hash::hash_find(arr, &key).map(clone_val)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(
                        slot,
                        val.unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null)),
                    );
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "offsetSet" {
                let key = args.first();
                let value = args.get(1);
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value
                    && let Some(storage_val) = o.properties.get_mut("__storage")
                    && let PhpValue::Array(ref mut arr) = storage_val.value
                    && let (Some(kv), Some(vv)) = (key, value)
                {
                    if let PhpValue::Long(k) = kv.value {
                        let _ = crate::engine::hash::hash_add_or_update(
                            arr,
                            None,
                            k as u64,
                            clone_val(vv),
                            0,
                        );
                    } else if let PhpValue::String(ref s) = kv.value {
                        let key =
                            crate::engine::string::string_init(s.as_str(), false);
                        let _ = crate::engine::hash::hash_add_or_update(
                            arr,
                            Some(&key),
                            0,
                            clone_val(vv),
                            0,
                        );
                    }
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value {
                        execute_data.set_temp(slot_idx as usize, updated);
                    }
                } else if is_var_ref(&op.op2)
                    && let PhpValue::String(ref s) = op.op2.value
                {
                    let vname = s.as_str();
                    let name = vname.strip_prefix('$').unwrap_or(vname);
                    execute_data.set_var(name, updated);
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "offsetUnset" {
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value
                    && let Some(storage_val) = o.properties.get_mut("__storage")
                    && let PhpValue::Array(ref mut arr) = storage_val.value
                    && let Some(kv) = args.first()
                {
                    if let PhpValue::String(ref s) = kv.value {
                        let key = crate::engine::string::string_init(s.as_str(), false);
                        let _ = crate::engine::hash::hash_del(arr, &key);
                    } else {
                        let k = crate::engine::operators::zval_get_long(kv) as u64;
                        if let Some(pos) = arr
                            .ar_data
                            .iter()
                            .position(|b| b.h == k && b.key.is_none())
                        {
                            arr.ar_data.remove(pos);
                            arr.n_num_of_elements -= 1;
                        }
                    }
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value {
                        execute_data.set_temp(slot_idx as usize, updated);
                    }
                } else if is_var_ref(&op.op2)
                    && let PhpValue::String(ref s) = op.op2.value
                {
                    let vname = s.as_str();
                    let name = vname.strip_prefix('$').unwrap_or(vname);
                    execute_data.set_var(name, updated);
                }
                return Ok(ExecResult::Continue);
            }
        }

        // Built-in SplStack/SplQueue methods (LIFO/FIFO backed by __storage array)
        if class_name == "SplStack" || class_name == "SplQueue" {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let mn = method_name.as_str();
            let mut storage = obj.properties.get("__storage")
                .map(clone_val)
                .unwrap_or_else(|| Val::new(PhpValue::Array(Box::default()), PhpType::Array));

            if mn == "push" || mn == "enqueue" {
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    if let PhpValue::Array(ref mut arr) = storage.value {
                        let next_idx = arr.ar_data.len() as u64;
                        let _ = crate::engine::hash::hash_add_or_update(
                            arr, None, next_idx,
                            args.first().map(clone_val).unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null)),
                            0,
                        );
                    }
                    o.properties.insert("__storage".to_string(), storage);
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value { execute_data.set_temp(slot_idx as usize, updated); }
                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                    execute_data.set_var(name, updated);
                }
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null)); }
                return Ok(ExecResult::Continue);
            }

            if mn == "pop" || mn == "dequeue" {
                let result = if let PhpValue::Array(ref arr) = storage.value {
                    if arr.ar_data.is_empty() {
                        Val::new(PhpValue::Long(0), PhpType::Null)
                    } else if mn == "pop" {
                        clone_val(&arr.ar_data.last().unwrap().val)
                    } else {
                        clone_val(&arr.ar_data.first().unwrap().val)
                    }
                } else { Val::new(PhpValue::Long(0), PhpType::Null) };

                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value
                    && let PhpValue::Array(ref mut arr) = storage.value
                {
                    if !arr.ar_data.is_empty() {
                        if mn == "pop" { arr.ar_data.pop(); }
                        else { arr.ar_data.remove(0); }
                    }
                    o.properties.insert("__storage".to_string(), storage);
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value { execute_data.set_temp(slot_idx as usize, updated); }
                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                    execute_data.set_var(name, updated);
                }
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
                return Ok(ExecResult::Continue);
            }

            if mn == "top" {
                let result = if let PhpValue::Array(ref arr) = storage.value {
                    if arr.ar_data.is_empty() { Val::new(PhpValue::Long(0), PhpType::Null) }
                    else { clone_val(&arr.ar_data.last().unwrap().val) }
                } else { Val::new(PhpValue::Long(0), PhpType::Null) };
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
                return Ok(ExecResult::Continue);
            }

            if mn == "bottom" {
                let result = if let PhpValue::Array(ref arr) = storage.value {
                    if arr.ar_data.is_empty() { Val::new(PhpValue::Long(0), PhpType::Null) }
                    else { clone_val(&arr.ar_data.first().unwrap().val) }
                } else { Val::new(PhpValue::Long(0), PhpType::Null) };
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
                return Ok(ExecResult::Continue);
            }

            if mn == "count" || mn == "isEmpty" {
                let cnt = if let PhpValue::Array(ref arr) = storage.value { arr.ar_data.len() as i64 } else { 0 };
                if mn == "count" {
                    if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, Val::new(PhpValue::Long(cnt), PhpType::Long)); }
                } else {
                    let empty = cnt == 0;
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, Val::new(PhpValue::Long(if empty {1} else {0}), if empty {PhpType::True} else {PhpType::False}));
                    }
                }
                return Ok(ExecResult::Continue);
            }
        }

        // Built-in DirectoryIterator methods
        if class_name == "DirectoryIterator" {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let mn = method_name.as_str();

            if mn == "__construct" {
                let path = args.first().map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| crate::engine::string::string_init(".", false));
                let entries = crate::php::filesystem::php_scandir(path.as_str())
                    .unwrap_or_default();
                let mut arr = PhpArray::new();
                for e in entries {
                    let idx = arr.ar_data.len() as u64;
                    let _ = crate::engine::hash::hash_add_or_update(
                        &mut arr, None, idx,
                        Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&e, false))), PhpType::String),
                        0,
                    );
                }
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert("__path".to_string(),
                        Val::new(PhpValue::String(Box::new(path)), PhpType::String));
                    o.properties.insert("__entries".to_string(),
                        Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array));
                    o.properties.insert("__index".to_string(),
                        Val::new(PhpValue::Long(0), PhpType::Long));
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value { execute_data.set_temp(slot_idx as usize, updated); }
                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                    execute_data.set_var(name, updated);
                }
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null)); }
                return Ok(ExecResult::Continue);
            }

            let entries = obj.properties.get("__entries")
                .map(clone_val)
                .unwrap_or_else(|| Val::new(PhpValue::Array(Box::default()), PhpType::Array));
            let index = obj.properties.get("__index")
                .map(crate::engine::operators::zval_get_long)
                .unwrap_or(0);
            let path = obj.properties.get("__path")
                .map(crate::engine::operators::zval_get_string)
                .unwrap_or_else(|| crate::engine::string::string_init(".", false));

            let get_current_filename = || -> Option<String> {
                if let PhpValue::Array(ref arr) = entries.value
                    && (index as usize) < arr.ar_data.len()
                    && let PhpValue::String(ref s) = arr.ar_data[index as usize].val.value
                {
                    return Some(s.as_str().to_string());
                }
                None
            };

            if mn == "valid" {
                let valid = if let PhpValue::Array(ref arr) = entries.value {
                    (index as usize) < arr.ar_data.len()
                } else { false };
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(if valid {1} else {0}), if valid {PhpType::True} else {PhpType::False}));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "current" || mn == "getFilename" {
                let result = get_current_filename()
                    .map(|s| Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&s, false))), PhpType::String))
                    .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null));
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
                return Ok(ExecResult::Continue);
            }

            if mn == "getPathname" {
                let result = get_current_filename()
                    .map(|f| {
                        let full = if path.as_str().ends_with('/') {
                            format!("{}{}", path.as_str(), f)
                        } else {
                            format!("{}/{}", path.as_str(), f)
                        };
                        Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&full, false))), PhpType::String)
                    })
                    .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null));
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
                return Ok(ExecResult::Continue);
            }

            if mn == "key" {
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, Val::new(PhpValue::Long(index), PhpType::Long)); }
                return Ok(ExecResult::Continue);
            }

            if mn == "next" || mn == "rewind" {
                let new_index = if mn == "next" { index + 1 } else { 0 };
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert("__index".to_string(), Val::new(PhpValue::Long(new_index), PhpType::Long));
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value { execute_data.set_temp(slot_idx as usize, updated); }
                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                    execute_data.set_var(name, updated);
                }
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null)); }
                return Ok(ExecResult::Continue);
            }

            if mn == "getPath" {
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::String(Box::new(path)), PhpType::String));
                }
                return Ok(ExecResult::Continue);
            }
        }

        // Built-in SplHeap/SplPriorityQueue methods
        if class_name == "SplHeap" || class_name == "SplPriorityQueue"
            || class_name == "SplMinHeap" || class_name == "SplMaxHeap"
        {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let mn = method_name.as_str();
            let mut storage = obj.properties.get("__storage")
                .map(clone_val)
                .unwrap_or_else(|| Val::new(PhpValue::Array(Box::default()), PhpType::Array));
            let is_min_heap = class_name == "SplMinHeap";

            // Helper: compare two values for heap ordering.
            // Returns Ordering::Less if a should come before b (a has higher priority).
            let compare_vals = |a: &Val, b: &Val| -> std::cmp::Ordering {
                use crate::engine::operators::{zval_get_long, zval_get_string};
                let ord = match (&a.value, &b.value) {
                    (PhpValue::Long(x), PhpValue::Long(y)) => x.cmp(y),
                    (PhpValue::Double(x), PhpValue::Double(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    (PhpValue::Long(x), PhpValue::Double(y)) => (*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                    (PhpValue::Double(x), PhpValue::Long(y)) => x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal),
                    _ => {
                        let as_l = zval_get_long(a);
                        let bs_l = zval_get_long(b);
                        if as_l != 0 || bs_l != 0 {
                            as_l.cmp(&bs_l)
                        } else {
                            zval_get_string(a).as_str().cmp(zval_get_string(b).as_str())
                        }
                    }
                };
                // Reverse: higher value = higher priority = comes first (Less)
                ord.reverse()
            };

            if mn == "insert" {
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut _o) = updated.value
                    && let PhpValue::Array(ref mut arr) = storage.value
                {
                    if class_name == "SplPriorityQueue" {
                        // Store as nested array [value, priority]
                        let value = args.first().map(clone_val).unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null));
                        let priority = args.get(1).map(clone_val).unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Long));
                        let mut pair = PhpArray::new();
                        let _ = crate::engine::hash::hash_add_or_update(&mut pair, None, 0, value, 0);
                        let _ = crate::engine::hash::hash_add_or_update(&mut pair, None, 1, priority, 0);
                        let idx = arr.ar_data.len() as u64;
                        let _ = crate::engine::hash::hash_add_or_update(arr, None, idx,
                            Val::new(PhpValue::Array(Box::new(pair)), PhpType::Array), 0);
                    } else {
                        let idx = arr.ar_data.len() as u64;
                        let _ = crate::engine::hash::hash_add_or_update(arr, None, idx,
                            args.first().map(clone_val).unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null)), 0);
                    }
                }
                // Write back the modified storage to the updated object
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert("__storage".to_string(), storage);
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value { execute_data.set_temp(slot_idx as usize, updated); }
                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                    execute_data.set_var(name, updated);
                }
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null)); }
                return Ok(ExecResult::Continue);
            }

            if mn == "extract" {
                // Sort and extract the top element
                let result = if let PhpValue::Array(ref mut arr) = storage.value {
                    if arr.ar_data.is_empty() {
                        Val::new(PhpValue::Long(0), PhpType::Null)
                    } else {
                        // Sort: for SplHeap/SplMaxHeap/SplPriorityQueue, max on top; for SplMinHeap, min on top
                        if class_name == "SplPriorityQueue" {
                            arr.ar_data.sort_by(|a, b| {
                                let pa = if let PhpValue::Array(ref p) = a.val.value { p.ar_data.get(1).map(|b| &b.val).cloned() } else { None };
                                let pb = if let PhpValue::Array(ref p) = b.val.value { p.ar_data.get(1).map(|b| &b.val).cloned() } else { None };
                                match (pa, pb) {
                                    (Some(pa), Some(pb)) => {
                                        if is_min_heap { compare_vals(&pb, &pa) } else { compare_vals(&pa, &pb) }
                                    }
                                    _ => std::cmp::Ordering::Equal,
                                }
                            });
                            // Extract first element's value (index 0 of the pair)
                            if let PhpValue::Array(ref pair) = arr.ar_data[0].val.value {
                                if let Some(v) = pair.ar_data.first() { clone_val(&v.val) } else { Val::new(PhpValue::Long(0), PhpType::Null) }
                            } else { Val::new(PhpValue::Long(0), PhpType::Null) }
                        } else {
                            arr.ar_data.sort_by(|a, b| {
                                if is_min_heap { compare_vals(&b.val, &a.val) } else { compare_vals(&a.val, &b.val) }
                            });
                            clone_val(&arr.ar_data[0].val)
                        }
                    }
                } else { Val::new(PhpValue::Long(0), PhpType::Null) };

                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut _o) = updated.value
                    && let PhpValue::Array(ref mut arr) = storage.value
                    && !arr.ar_data.is_empty()
                {
                    arr.ar_data.remove(0);
                }
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert("__storage".to_string(), storage);
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value { execute_data.set_temp(slot_idx as usize, updated); }
                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                    execute_data.set_var(name, updated);
                }
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
                return Ok(ExecResult::Continue);
            }

            if mn == "top" {
                let result = if let PhpValue::Array(ref mut arr) = storage.value {
                    if arr.ar_data.is_empty() {
                        Val::new(PhpValue::Long(0), PhpType::Null)
                    } else {
                        if class_name == "SplPriorityQueue" {
                            arr.ar_data.sort_by(|a, b| {
                                let pa = if let PhpValue::Array(ref p) = a.val.value { p.ar_data.get(1).map(|b| &b.val).cloned() } else { None };
                                let pb = if let PhpValue::Array(ref p) = b.val.value { p.ar_data.get(1).map(|b| &b.val).cloned() } else { None };
                                match (pa, pb) {
                                    (Some(pa), Some(pb)) => {
                                        if is_min_heap { compare_vals(&pb, &pa) } else { compare_vals(&pa, &pb) }
                                    }
                                    _ => std::cmp::Ordering::Equal,
                                }
                            });
                            if let PhpValue::Array(ref pair) = arr.ar_data[0].val.value {
                                if let Some(v) = pair.ar_data.first() { clone_val(&v.val) } else { Val::new(PhpValue::Long(0), PhpType::Null) }
                            } else { Val::new(PhpValue::Long(0), PhpType::Null) }
                        } else {
                            arr.ar_data.sort_by(|a, b| {
                                if is_min_heap { compare_vals(&b.val, &a.val) } else { compare_vals(&a.val, &b.val) }
                            });
                            clone_val(&arr.ar_data[0].val)
                        }
                    }
                } else { Val::new(PhpValue::Long(0), PhpType::Null) };

                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert("__storage".to_string(), storage);
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value { execute_data.set_temp(slot_idx as usize, updated); }
                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                    execute_data.set_var(name, updated);
                }
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
                return Ok(ExecResult::Continue);
            }

            if mn == "count" || mn == "isEmpty" {
                let cnt = if let PhpValue::Array(ref arr) = storage.value { arr.ar_data.len() as i64 } else { 0 };
                if mn == "count" {
                    if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, Val::new(PhpValue::Long(cnt), PhpType::Long)); }
                } else {
                    let empty = cnt == 0;
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, Val::new(PhpValue::Long(if empty {1} else {0}), if empty {PhpType::True} else {PhpType::False}));
                    }
                }
                return Ok(ExecResult::Continue);
            }

            // Iterator interface
            if mn == "rewind" {
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert("__index".to_string(), Val::new(PhpValue::Long(0), PhpType::Long));
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value { execute_data.set_temp(slot_idx as usize, updated); }
                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                    execute_data.set_var(name, updated);
                }
                return Ok(ExecResult::Continue);
            }
            if mn == "valid" {
                let index = obj.properties.get("__index").map(crate::engine::operators::zval_get_long).unwrap_or(0);
                let cnt = if let PhpValue::Array(ref arr) = storage.value { arr.ar_data.len() as i64 } else { 0 };
                let valid = index < cnt;
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(if valid {1} else {0}), if valid {PhpType::True} else {PhpType::False}));
                }
                return Ok(ExecResult::Continue);
            }
            if mn == "current" {
                let index = obj.properties.get("__index").map(crate::engine::operators::zval_get_long).unwrap_or(0);
                let result = if let PhpValue::Array(ref arr) = storage.value {
                    if (index as usize) < arr.ar_data.len() {
                        if class_name == "SplPriorityQueue" {
                            if let PhpValue::Array(ref pair) = arr.ar_data[index as usize].val.value {
                                pair.ar_data.first().map(|b| clone_val(&b.val)).unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null))
                            } else { Val::new(PhpValue::Long(0), PhpType::Null) }
                        } else {
                            clone_val(&arr.ar_data[index as usize].val)
                        }
                    } else { Val::new(PhpValue::Long(0), PhpType::Null) }
                } else { Val::new(PhpValue::Long(0), PhpType::Null) };
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
                return Ok(ExecResult::Continue);
            }
            if mn == "key" {
                let index = obj.properties.get("__index").map(crate::engine::operators::zval_get_long).unwrap_or(0);
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, Val::new(PhpValue::Long(index), PhpType::Long)); }
                return Ok(ExecResult::Continue);
            }
            if mn == "next" {
                let index = obj.properties.get("__index").map(crate::engine::operators::zval_get_long).unwrap_or(0);
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert("__index".to_string(), Val::new(PhpValue::Long(index + 1), PhpType::Long));
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value { execute_data.set_temp(slot_idx as usize, updated); }
                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                    execute_data.set_var(name, updated);
                }
                return Ok(ExecResult::Continue);
            }
        }

        // Built-in SplFileInfo/SplFileObject methods
        if class_name == "SplFileInfo" || class_name == "SplFileObject" {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let mn = method_name.as_str();
            let path = obj.properties.get("__path")
                .map(crate::engine::operators::zval_get_string)
                .unwrap_or_else(|| crate::engine::string::string_init("", false));

            if mn == "__construct" {
                let p = args.first().map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| crate::engine::string::string_init("", false));
                let mode = args.get(1).map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| crate::engine::string::string_init("r", false));
                let content = if class_name == "SplFileObject" {
                    crate::php::filesystem::php_file_get_contents(p.as_str())
                        .ok()
                        .map(|c| Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&c, false))), PhpType::String))
                        .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null))
                } else {
                    Val::new(PhpValue::Long(0), PhpType::Null)
                };
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert("__path".to_string(),
                        Val::new(PhpValue::String(Box::new(p)), PhpType::String));
                    o.properties.insert("__content".to_string(), content);
                    o.properties.insert("__pos".to_string(),
                        Val::new(PhpValue::Long(0), PhpType::Long));
                    o.properties.insert("__mode".to_string(),
                        Val::new(PhpValue::String(Box::new(mode)), PhpType::String));
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value { execute_data.set_temp(slot_idx as usize, updated); }
                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                    execute_data.set_var(name, updated);
                }
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null)); }
                return Ok(ExecResult::Continue);
            }

            if mn == "getPath" {
                let dir = crate::php::filesystem::php_dirname(path.as_str());
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&dir, false))), PhpType::String));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "getFilename" {
                let base = crate::php::filesystem::php_basename(path.as_str());
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&base, false))), PhpType::String));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "getPathname" {
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::String(Box::new(path)), PhpType::String));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "getRealPath" {
                let rp = crate::php::filesystem::php_realpath(path.as_str()).unwrap_or_default();
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&rp, false))), PhpType::String));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "getSize" {
                let size = crate::php::filesystem::php_filesize(path.as_str()).unwrap_or(0);
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(size as i64), PhpType::Long));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "isFile" {
                let is_f = crate::php::filesystem::php_is_file(path.as_str());
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(if is_f {1} else {0}), if is_f {PhpType::True} else {PhpType::False}));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "isDir" {
                let is_d = crate::php::filesystem::php_is_dir(path.as_str());
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(if is_d {1} else {0}), if is_d {PhpType::True} else {PhpType::False}));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "isReadable" {
                let r = crate::php::filesystem::php_is_readable(path.as_str());
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(if r {1} else {0}), if r {PhpType::True} else {PhpType::False}));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "isWritable" {
                let w = crate::php::filesystem::php_is_writable(path.as_str());
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(if w {1} else {0}), if w {PhpType::True} else {PhpType::False}));
                }
                return Ok(ExecResult::Continue);
            }

            // SplFileObject-specific methods
            if mn == "fgets" {
                let content = obj.properties.get("__content")
                    .map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| crate::engine::string::string_init("", false));
                let pos = obj.properties.get("__pos")
                    .map(crate::engine::operators::zval_get_long)
                    .unwrap_or(0);
                let content_str = content.as_str();
                let result = if (pos as usize) >= content_str.len() {
                    Val::new(PhpValue::Long(0), PhpType::False)
                } else {
                    let remaining = &content_str[pos as usize..];
                    let line_end = remaining.find('\n').map(|i| i + 1).unwrap_or(remaining.len());
                    let line = &remaining[..line_end];
                    let new_pos = pos + line_end as i64;
                    let mut updated = clone_val(&obj_val);
                    if let PhpValue::Object(ref mut o) = updated.value {
                        o.properties.insert("__pos".to_string(), Val::new(PhpValue::Long(new_pos), PhpType::Long));
                    }
                    if is_temp_ref(&op.op2) {
                        if let PhpValue::Long(slot_idx) = op.op2.value { execute_data.set_temp(slot_idx as usize, updated); }
                    } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                        let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                        execute_data.set_var(name, updated);
                    }
                    Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(line, false))), PhpType::String)
                };
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
                return Ok(ExecResult::Continue);
            }

            if mn == "fread" {
                let content = obj.properties.get("__content")
                    .map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| crate::engine::string::string_init("", false));
                let pos = obj.properties.get("__pos")
                    .map(crate::engine::operators::zval_get_long)
                    .unwrap_or(0);
                let length = args.first().map(crate::engine::operators::zval_get_long).unwrap_or(0) as usize;
                let content_str = content.as_str();
                let result = if (pos as usize) >= content_str.len() {
                    Val::new(PhpValue::Long(0), PhpType::False)
                } else {
                    let remaining = &content_str[pos as usize..];
                    let read_len = length.min(remaining.len());
                    let data = &remaining[..read_len];
                    let new_pos = pos + read_len as i64;
                    let mut updated = clone_val(&obj_val);
                    if let PhpValue::Object(ref mut o) = updated.value {
                        o.properties.insert("__pos".to_string(), Val::new(PhpValue::Long(new_pos), PhpType::Long));
                    }
                    if is_temp_ref(&op.op2) {
                        if let PhpValue::Long(slot_idx) = op.op2.value { execute_data.set_temp(slot_idx as usize, updated); }
                    } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                        let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                        execute_data.set_var(name, updated);
                    }
                    Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(data, false))), PhpType::String)
                };
                if let Some(slot) = result_slot(op) { execute_data.set_temp(slot, result); }
                return Ok(ExecResult::Continue);
            }

            if mn == "feof" {
                let content = obj.properties.get("__content")
                    .map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| crate::engine::string::string_init("", false));
                let pos = obj.properties.get("__pos")
                    .map(crate::engine::operators::zval_get_long)
                    .unwrap_or(0);
                let eof = (pos as usize) >= content.as_str().len();
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(if eof {1} else {0}), if eof {PhpType::True} else {PhpType::False}));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "fwrite" || mn == "fputs" {
                let data = args.first().map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| crate::engine::string::string_init("", false));
                let written = crate::php::filesystem::php_file_put_contents(path.as_str(), data.as_str())
                    .unwrap_or(0);
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(written as i64), PhpType::Long));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "rewind" {
                let mut updated = clone_val(&obj_val);
                if let PhpValue::Object(ref mut o) = updated.value {
                    o.properties.insert("__pos".to_string(), Val::new(PhpValue::Long(0), PhpType::Long));
                }
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value { execute_data.set_temp(slot_idx as usize, updated); }
                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                    execute_data.set_var(name, updated);
                }
                return Ok(ExecResult::Continue);
            }
        }

        // Built-in SimpleXMLElement methods
        if class_name == "SimpleXMLElement" {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let mn = method_name.as_str();

            if mn == "__construct" {
                // Parse XML string and populate object properties
                let xml_str = args.first()
                    .map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| string_init("", false));
                if let Ok(node) = crate::php::xml::parse_xml(xml_str.as_str()) {
                    let mut updated = crate::engine::types::PhpObject::new("SimpleXMLElement");
                    updated.properties.insert("__name".to_string(), Val::new(
                        PhpValue::String(Box::new(string_init(&node.name, false))), PhpType::String));
                    if !node.text.is_empty() {
                        updated.properties.insert("__text".to_string(), Val::new(
                            PhpValue::String(Box::new(string_init(&node.text, false))), PhpType::String));
                    }
                    for (k, v) in &node.attributes {
                        let key = format!("@{k}");
                        updated.properties.insert(key, Val::new(
                            PhpValue::String(Box::new(string_init(v, false))), PhpType::String));
                    }
                    let mut child_map: std::collections::HashMap<String, Vec<&crate::php::xml::XmlNode>> = std::collections::HashMap::new();
                    for child in &node.children {
                        child_map.entry(child.name.clone()).or_default().push(child);
                    }
                    for (name, children) in child_map {
                        if children.len() == 1 {
                            updated.properties.insert(name, simplexml_node_to_val(children[0]));
                        } else {
                            let mut arr = PhpArray::new();
                            for (i, child) in children.iter().enumerate() {
                                let _ = crate::engine::hash::hash_add_or_update(
                                    &mut arr, None, i as u64, simplexml_node_to_val(child), 0);
                            }
                            updated.properties.insert(name, Val::new(
                                PhpValue::Array(Box::new(arr)), PhpType::Array));
                        }
                    }
                    let result = Val::new(PhpValue::Object(Box::new(updated)), PhpType::Object);
                    if is_temp_ref(&op.op2) {
                        if let PhpValue::Long(slot_idx) = op.op2.value {
                            execute_data.set_temp(slot_idx as usize, result);
                        }
                    } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                        let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                        execute_data.set_var(name, result);
                    }
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "asXML" || mn == "saveXML" {
                // Serialize the object back to XML
                let xml = simplexml_object_to_string(obj);
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(
                        PhpValue::String(Box::new(crate::engine::string::string_init(&xml, false))),
                        PhpType::String));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "getName" {
                // Return the element name (stored in __name property or class hint)
                let name = obj.properties.get("__name")
                    .map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| crate::engine::string::string_init("", false));
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::String(Box::new(name)), PhpType::String));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "__toString" {
                let text = obj.properties.get("__text")
                    .map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| crate::engine::string::string_init("", false));
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::String(Box::new(text)), PhpType::String));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "count" {
                // Count child elements
                let count = obj.properties.iter()
                    .filter(|(k, _)| !k.starts_with('@') && !k.starts_with("__"))
                    .count();
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(count as i64), PhpType::Long));
                }
                return Ok(ExecResult::Continue);
            }
        }

        // Built-in PDO methods
        if class_name == "PDO" {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let mn = method_name.as_str();

            if mn == "__construct" {
                let dsn = args.first().map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| string_init("", false));
                let dsn_str = dsn.as_str().to_string();
                let parts: Vec<&str> = dsn_str.splitn(2, ':').collect();
                let driver = parts.first().unwrap_or(&"").to_string();
                let path = parts.get(1).unwrap_or(&":memory:").to_string();

                let mut updated = crate::engine::types::PhpObject::new("PDO");
                updated.properties.insert("__dsn".to_string(), Val::new(
                    PhpValue::String(Box::new(string_init(&dsn_str, false))), PhpType::String));
                updated.properties.insert("__driver".to_string(), Val::new(
                    PhpValue::String(Box::new(string_init(&driver, false))), PhpType::String));

                if driver == "sqlite" {
                    match crate::php::sqlite::SqlitePdo::new(&path) {
                        Ok(pdo) => {
                            let conn_id = crate::php::sqlite::register_connection(pdo);
                            updated.properties.insert("__conn_id".to_string(),
                                Val::new(PhpValue::Long(conn_id as i64), PhpType::Long));
                            updated.properties.insert("__connected".to_string(),
                                Val::new(PhpValue::Long(1), PhpType::True));
                        }
                        Err(e) => {
                            updated.properties.insert("__error".to_string(), Val::new(
                                PhpValue::String(Box::new(string_init(&e, false))), PhpType::String));
                            updated.properties.insert("__connected".to_string(),
                                Val::new(PhpValue::Long(0), PhpType::False));
                        }
                    }
                } else {
                    updated.properties.insert("__error".to_string(), Val::new(
                        PhpValue::String(Box::new(string_init(
                            &format!("Unsupported PDO driver: {driver}"), false))), PhpType::String));
                    updated.properties.insert("__connected".to_string(),
                        Val::new(PhpValue::Long(0), PhpType::False));
                }

                let result = Val::new(PhpValue::Object(Box::new(updated)), PhpType::Object);
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value {
                        execute_data.set_temp(slot_idx as usize, result);
                    }
                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                    execute_data.set_var(name, result);
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "query" {
                let sql = args.first().map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| string_init("", false));
                let conn_id = obj.properties.get("__conn_id")
                    .map(crate::engine::operators::zval_get_long).unwrap_or(0) as usize;

                if let Some(mut pdo) = crate::php::sqlite::take_connection(conn_id) {
                    match pdo.query(sql.as_str()) {
                        Ok(rows) => {
                            let row_count = rows.len() as i64;
                            let mut stmt_obj = crate::engine::types::PhpObject::new("PDOStatement");
                            let mut arr = PhpArray::new();
                            for (i, row) in rows.iter().enumerate() {
                                let mut row_arr = PhpArray::new();
                                for (j, (k, v)) in row.iter().enumerate() {
                                    let key = string_init(k, false);
                                    let _ = crate::engine::hash::hash_add_or_update(
                                        &mut row_arr, Some(&key), j as u64,
                                        Val::new(PhpValue::String(Box::new(string_init(v, false))), PhpType::String), 0);
                                }
                                let _ = crate::engine::hash::hash_add_or_update(
                                    &mut arr, None, i as u64,
                                    Val::new(PhpValue::Array(Box::new(row_arr)), PhpType::Array), 0);
                            }
                            stmt_obj.properties.insert("__rows".to_string(),
                                Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array));
                            stmt_obj.properties.insert("__row_count".to_string(),
                                Val::new(PhpValue::Long(row_count), PhpType::Long));
                            stmt_obj.properties.insert("__pos".to_string(),
                                Val::new(PhpValue::Long(0), PhpType::Long));
                            crate::php::sqlite::put_connection(conn_id, pdo);
                            if let Some(slot) = result_slot(op) {
                                execute_data.set_temp(slot, Val::new(
                                    PhpValue::Object(Box::new(stmt_obj)), PhpType::Object));
                            }
                            return Ok(ExecResult::Continue);
                        }
                        Err(_e) => {
                            crate::php::sqlite::put_connection(conn_id, pdo);
                            if let Some(slot) = result_slot(op) {
                                execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::False));
                            }
                            return Ok(ExecResult::Continue);
                        }
                    }
                }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::False));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "exec" {
                let sql = args.first().map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| string_init("", false));
                let conn_id = obj.properties.get("__conn_id")
                    .map(crate::engine::operators::zval_get_long).unwrap_or(0) as usize;

                if let Some(mut pdo) = crate::php::sqlite::take_connection(conn_id) {
                    let result = pdo.exec(sql.as_str());
                    let last_id = pdo.last_insert_id();
                    crate::php::sqlite::put_connection(conn_id, pdo);
                    match result {
                        Ok(affected) => {
                            // Update last_insert_id on the PDO object
                            let mut updated = clone_val(&obj_val);
                            if let PhpValue::Object(ref mut o) = updated.value {
                                o.properties.insert("__last_insert_id".to_string(),
                                    Val::new(PhpValue::Long(last_id), PhpType::Long));
                            }
                            if is_temp_ref(&op.op2) {
                                if let PhpValue::Long(slot_idx) = op.op2.value {
                                    execute_data.set_temp(slot_idx as usize, updated);
                                }
                            } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                                let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                                execute_data.set_var(name, updated);
                            }
                            if let Some(slot) = result_slot(op) {
                                execute_data.set_temp(slot, Val::new(PhpValue::Long(affected), PhpType::Long));
                            }
                            return Ok(ExecResult::Continue);
                        }
                        Err(_) => {
                            if let Some(slot) = result_slot(op) {
                                execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::False));
                            }
                            return Ok(ExecResult::Continue);
                        }
                    }
                }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::False));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "lastInsertId" {
                let last_id = obj.properties.get("__last_insert_id")
                    .map(crate::engine::operators::zval_get_long).unwrap_or(0);
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(last_id), PhpType::Long));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "prepare" {
                let sql = args.first().map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| string_init("", false));
                let mut stmt_obj = crate::engine::types::PhpObject::new("PDOStatement");
                stmt_obj.properties.insert("__sql".to_string(), Val::new(
                    PhpValue::String(Box::new(sql)), PhpType::String));
                stmt_obj.properties.insert("__rows".to_string(), Val::new(
                    PhpValue::Array(Box::default()), PhpType::Array));
                stmt_obj.properties.insert("__pos".to_string(), Val::new(PhpValue::Long(0), PhpType::Long));
                stmt_obj.properties.insert("__params".to_string(), Val::new(
                    PhpValue::Array(Box::default()), PhpType::Array));
                stmt_obj.properties.insert("__row_count".to_string(), Val::new(PhpValue::Long(0), PhpType::Long));
                let conn_id = obj.properties.get("__conn_id")
                    .map(crate::engine::operators::zval_get_long).unwrap_or(0) as usize;
                stmt_obj.properties.insert("__conn_id".to_string(),
                    Val::new(PhpValue::Long(conn_id as i64), PhpType::Long));
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(
                        PhpValue::Object(Box::new(stmt_obj)), PhpType::Object));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "beginTransaction" {
                let conn_id = obj.properties.get("__conn_id")
                    .map(crate::engine::operators::zval_get_long).unwrap_or(0) as usize;
                if let Some(mut pdo) = crate::php::sqlite::take_connection(conn_id) {
                    let result = pdo.begin_transaction();
                    crate::php::sqlite::put_connection(conn_id, pdo);
                    let ok = result.unwrap_or(false);
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, Val::new(
                            PhpValue::Long(if ok {1} else {0}),
                            if ok {PhpType::True} else {PhpType::False}));
                    }
                    return Ok(ExecResult::Continue);
                }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::False));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "commit" {
                let conn_id = obj.properties.get("__conn_id")
                    .map(crate::engine::operators::zval_get_long).unwrap_or(0) as usize;
                if let Some(mut pdo) = crate::php::sqlite::take_connection(conn_id) {
                    let result = pdo.commit();
                    crate::php::sqlite::put_connection(conn_id, pdo);
                    let ok = result.unwrap_or(false);
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, Val::new(
                            PhpValue::Long(if ok {1} else {0}),
                            if ok {PhpType::True} else {PhpType::False}));
                    }
                    return Ok(ExecResult::Continue);
                }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::False));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "rollBack" || mn == "rollback" {
                let conn_id = obj.properties.get("__conn_id")
                    .map(crate::engine::operators::zval_get_long).unwrap_or(0) as usize;
                if let Some(mut pdo) = crate::php::sqlite::take_connection(conn_id) {
                    let result = pdo.rollback();
                    crate::php::sqlite::put_connection(conn_id, pdo);
                    let ok = result.unwrap_or(false);
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, Val::new(
                            PhpValue::Long(if ok {1} else {0}),
                            if ok {PhpType::True} else {PhpType::False}));
                    }
                    return Ok(ExecResult::Continue);
                }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::False));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "errorCode" {
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(
                        PhpValue::String(Box::new(string_init("00000", false))), PhpType::String));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "errorInfo" {
                let mut arr = PhpArray::new();
                let _ = crate::engine::hash::hash_add_or_update(
                    &mut arr, None, 0,
                    Val::new(PhpValue::String(Box::new(string_init("00000", false))), PhpType::String), 0);
                let _ = crate::engine::hash::hash_add_or_update(
                    &mut arr, None, 1, Val::new(PhpValue::Long(0), PhpType::Null), 0);
                let _ = crate::engine::hash::hash_add_or_update(
                    &mut arr, None, 2, Val::new(PhpValue::Long(0), PhpType::Null), 0);
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(
                        PhpValue::Array(Box::new(arr)), PhpType::Array));
                }
                return Ok(ExecResult::Continue);
            }
        }

        // Built-in PDOStatement methods
        if class_name == "PDOStatement" {
            let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let mn = method_name.as_str();

            if mn == "execute" {
                let sql = obj.properties.get("__sql")
                    .map(crate::engine::operators::zval_get_string)
                    .unwrap_or_else(|| string_init("", false));
                let conn_id = obj.properties.get("__conn_id")
                    .map(crate::engine::operators::zval_get_long).unwrap_or(0) as usize;

                // Collect params from args (array of values)
                let params: Vec<String> = if let Some(PhpValue::Array(arr)) = args.first().map(|v| &v.value) {
                    arr.ar_data.iter().map(|b| {
                        crate::engine::operators::zval_get_string(&b.val).as_str().to_string()
                    }).collect()
                } else {
                    Vec::new()
                };

                if let Some(mut pdo) = crate::php::sqlite::take_connection(conn_id) {
                    let sql_upper = sql.as_str().trim().to_uppercase();
                    if sql_upper.starts_with("SELECT") {
                        match pdo.prepare_execute(sql.as_str(), &params) {
                            Ok(rows) => {
                                let row_count = rows.len() as i64;
                                let mut updated = clone_val(&obj_val);
                                if let PhpValue::Object(ref mut o) = updated.value {
                                    let mut arr = PhpArray::new();
                                    for (i, row) in rows.iter().enumerate() {
                                        let mut row_arr = PhpArray::new();
                                        for (j, (k, v)) in row.iter().enumerate() {
                                            let key = string_init(k, false);
                                            let _ = crate::engine::hash::hash_add_or_update(
                                                &mut row_arr, Some(&key), j as u64,
                                                Val::new(PhpValue::String(Box::new(string_init(v, false))), PhpType::String), 0);
                                        }
                                        let _ = crate::engine::hash::hash_add_or_update(
                                            &mut arr, None, i as u64,
                                            Val::new(PhpValue::Array(Box::new(row_arr)), PhpType::Array), 0);
                                    }
                                    o.properties.insert("__rows".to_string(),
                                        Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array));
                                    o.properties.insert("__row_count".to_string(),
                                        Val::new(PhpValue::Long(row_count), PhpType::Long));
                                    o.properties.insert("__pos".to_string(),
                                        Val::new(PhpValue::Long(0), PhpType::Long));
                                }
                                crate::php::sqlite::put_connection(conn_id, pdo);
                                if is_temp_ref(&op.op2) {
                                    if let PhpValue::Long(slot_idx) = op.op2.value {
                                        execute_data.set_temp(slot_idx as usize, updated);
                                    }
                                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                                    execute_data.set_var(name, updated);
                                }
                                if let Some(slot) = result_slot(op) {
                                    execute_data.set_temp(slot, Val::new(PhpValue::Long(1), PhpType::True));
                                }
                                return Ok(ExecResult::Continue);
                            }
                            Err(_) => {
                                crate::php::sqlite::put_connection(conn_id, pdo);
                                if let Some(slot) = result_slot(op) {
                                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::False));
                                }
                                return Ok(ExecResult::Continue);
                            }
                        }
                    } else {
                        match pdo.prepare_exec(sql.as_str(), &params) {
                            Ok(affected) => {
                                let mut updated = clone_val(&obj_val);
                                if let PhpValue::Object(ref mut o) = updated.value {
                                    o.properties.insert("__row_count".to_string(),
                                        Val::new(PhpValue::Long(affected), PhpType::Long));
                                }
                                crate::php::sqlite::put_connection(conn_id, pdo);
                                if is_temp_ref(&op.op2) {
                                    if let PhpValue::Long(slot_idx) = op.op2.value {
                                        execute_data.set_temp(slot_idx as usize, updated);
                                    }
                                } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                                    let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                                    execute_data.set_var(name, updated);
                                }
                                if let Some(slot) = result_slot(op) {
                                    execute_data.set_temp(slot, Val::new(PhpValue::Long(1), PhpType::True));
                                }
                                return Ok(ExecResult::Continue);
                            }
                            Err(_) => {
                                crate::php::sqlite::put_connection(conn_id, pdo);
                                if let Some(slot) = result_slot(op) {
                                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::False));
                                }
                                return Ok(ExecResult::Continue);
                            }
                        }
                    }
                }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::False));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "fetch" || mn == "fetchAssoc" {
                let pos = obj.properties.get("__pos")
                    .map(crate::engine::operators::zval_get_long).unwrap_or(0) as usize;
                if let Some(PhpValue::Array(arr)) = obj.properties.get("__rows").map(|v| &v.value)
                    && pos < arr.ar_data.len() {
                        let row = clone_val(&arr.ar_data[pos].val);
                        let mut updated = clone_val(&obj_val);
                        if let PhpValue::Object(ref mut o) = updated.value {
                            o.properties.insert("__pos".to_string(),
                                Val::new(PhpValue::Long(pos as i64 + 1), PhpType::Long));
                        }
                        if is_temp_ref(&op.op2) {
                            if let PhpValue::Long(slot_idx) = op.op2.value {
                                execute_data.set_temp(slot_idx as usize, updated);
                            }
                        } else if is_var_ref(&op.op2) && let PhpValue::String(ref s) = op.op2.value {
                            let name = s.as_str().strip_prefix('$').unwrap_or(s.as_str());
                            execute_data.set_var(name, updated);
                        }
                        if let Some(slot) = result_slot(op) {
                            execute_data.set_temp(slot, row);
                        }
                        return Ok(ExecResult::Continue);
                    }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::False));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "fetchAll" {
                if let Some(PhpValue::Array(arr)) = obj.properties.get("__rows").map(|v| &v.value) {
                    let mut result_arr = PhpArray::new();
                    for (i, bucket) in arr.ar_data.iter().enumerate() {
                        let _ = crate::engine::hash::hash_add_or_update(
                            &mut result_arr, None, i as u64, clone_val(&bucket.val), 0);
                    }
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, Val::new(
                            PhpValue::Array(Box::new(result_arr)), PhpType::Array));
                    }
                    return Ok(ExecResult::Continue);
                }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(
                        PhpValue::Array(Box::default()), PhpType::Array));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "rowCount" {
                let count = obj.properties.get("__row_count")
                    .map(crate::engine::operators::zval_get_long).unwrap_or(0);
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(count), PhpType::Long));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "columnCount" {
                if let Some(PhpValue::Array(arr)) = obj.properties.get("__rows").map(|v| &v.value) {
                    let count = arr.ar_data.first().map(|b| {
                        if let PhpValue::Array(row) = &b.val.value {
                            row.ar_data.len() as i64
                        } else { 0 }
                    }).unwrap_or(0);
                    if let Some(slot) = result_slot(op) {
                        execute_data.set_temp(slot, Val::new(PhpValue::Long(count), PhpType::Long));
                    }
                    return Ok(ExecResult::Continue);
                }
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Long));
                }
                return Ok(ExecResult::Continue);
            }

            if mn == "bindParam" || mn == "bindValue" {
                if let Some(slot) = result_slot(op) {
                    execute_data.set_temp(slot, Val::new(PhpValue::Long(1), PhpType::True));
                }
                return Ok(ExecResult::Continue);
            }
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
            let arg_names: Vec<Option<String>> =
                execute_data.call_arg_names.drain(names_base..).collect();
            let arg_by_ref: Vec<bool> = execute_data.call_arg_by_ref.drain(base..).collect();
            let variadic = execute_data
                .class_table
                .get(&class_name)
                .and_then(|ce| ce.methods.get(method_name.as_str()))
                .and_then(|m| m.op_array.variadic_param.clone());
            let ref_params = execute_data
                .class_table
                .get(&class_name)
                .and_then(|ce| ce.methods.get(method_name.as_str()))
                .map(|m| m.op_array.ref_params.clone())
                .unwrap_or_default();
            bind_call_args(
                execute_data,
                &params,
                &args,
                &arg_names,
                &variadic,
                &ref_params,
                &arg_by_ref,
            );

            // Execute method
            let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
            method_op_array.ops = ops;
            let saved_try_depth = execute_data.try_stack.len();
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
            if let Some(er) = crate::engine::vm::exception_dispatch::propagate_after_call(
                execute_data,
                saved_try_depth,
            ) {
                return Ok(er);
            }

            // Copy modified $this back to the original object location (objects are reference-like in PHP)
            let this_val = execute_data.get_var("this");
            if let PhpValue::Object(_) = this_val.value {
                if is_temp_ref(&op.op2) {
                    if let PhpValue::Long(slot_idx) = op.op2.value {
                        execute_data.set_temp(slot_idx as usize, this_val);
                    }
                } else if is_var_ref(&op.op2)
                    && let PhpValue::String(ref s) = op.op2.value
                {
                    let vname = s.as_str();
                    let name = vname.strip_prefix('$').unwrap_or(vname);
                    execute_data.set_var(name, this_val);
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

            // Drain args now that we know __call is going to consume them.
            let (base, names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
            let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
            let _arg_names: Vec<Option<String>> =
                execute_data.call_arg_names.drain(names_base..).collect();

            let name_val = Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(
                    method_name.as_str(),
                    false,
                ))),
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
            bind_call_args(
                execute_data,
                &params,
                &[name_val, args_val],
                &[None, None],
                &None,
                &[],
                &[false, false],
            );

            let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
            method_op_array.ops = ops;
            let saved_try_depth = execute_data.try_stack.len();
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
            if let Some(er) = crate::engine::vm::exception_dispatch::propagate_after_call(
                execute_data,
                saved_try_depth,
            ) {
                return Ok(er);
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

    let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
    let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
    if let PhpValue::Object(ref obj) = obj_val.value
        && method_name.as_str() == "__construct"
        && crate::engine::vm::exception_dispatch::is_standard_throwable(&obj.class_name)
    {
        let msg = args
            .first()
            .map(|v| {
                crate::engine::operators::zval_get_string(v)
                    .as_str()
                    .to_string()
            })
            .unwrap_or_default();
        let code = args
            .get(1)
            .map(crate::engine::operators::zval_get_long)
            .unwrap_or(0);
        let mut updated = clone_val(&obj_val);
        if let PhpValue::Object(ref mut o) = updated.value {
            o.properties.insert(
                "message".to_string(),
                Val::new(
                    PhpValue::String(Box::new(crate::engine::string::string_init(&msg, false))),
                    PhpType::String,
                ),
            );
            o.properties.insert(
                "code".to_string(),
                Val::new(PhpValue::Long(code), PhpType::Long),
            );
            if let Some(file) = execute_data.constants.get("__FILE__") {
                o.properties.insert("file".to_string(), clone_val(file));
            }
        }
        // Write back to the slot the NewObj result lives in.
        if let PhpValue::Long(idx) = op.op2.value {
            execute_data.set_temp(idx as usize, updated);
        }
    } else if let Some(slot) = result_slot(op) {
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
    let mut class_name = crate::engine::operators::zval_get_string(&class_name_val)
        .as_str()
        .to_string();

    if class_name == "static" || class_name == "self" {
        class_name = execute_data.called_class.clone().unwrap_or_default();
    }

    let prop_name_val = resolve_operand(&op.op2, execute_data);
    let prop_name_raw = crate::engine::operators::zval_get_string(&prop_name_val);
    let prop_name_str = prop_name_raw.as_str();
    let prop_name = prop_name_str.strip_prefix('$').unwrap_or(prop_name_str);

    let result_val = if let Some(ce) = execute_data.class_table.get(&class_name) {
        if prop_name == "class" {
            Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(
                    &class_name,
                    false,
                ))),
                PhpType::String,
            )
        } else {
            // Walk the class and its parent chain for static properties
            // and constants (PHP inherits both from parent classes).
            let mut found: Option<Val> = None;
            let mut current_ce = Some(ce);
            while let Some(cur) = current_ce {
                if let Some(v) = cur.static_properties.get(prop_name) {
                    found = Some(clone_val(v));
                    break;
                }
                if let Some(v) = cur.constants.get(prop_name) {
                    found = Some(clone_val(v));
                    break;
                }
                current_ce = cur
                    .parent_name
                    .as_ref()
                    .and_then(|p| execute_data.class_table.get(p));
            }
            found.unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null))
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
    let mut class_name = crate::engine::operators::zval_get_string(&class_name_val)
        .as_str()
        .to_string();

    if class_name == "static" {
        class_name = execute_data.called_class.clone().unwrap_or_default();
    }

    let resolved_class = class_name.clone();

    // Built-in DateTime::createFromFormat() static method
    if (resolved_class == "DateTime" || resolved_class == "DateTimeImmutable")
        && method_name.as_str() == "createFromFormat"
    {
        let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
        let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
        let fmt = args
            .first()
            .map(crate::engine::operators::zval_get_string)
            .unwrap_or_else(|| crate::engine::string::string_init("", false));
        let dt_str = args
            .get(1)
            .map(crate::engine::operators::zval_get_string)
            .unwrap_or_else(|| crate::engine::string::string_init("", false));
        let ts = crate::php::datetime::parse_from_format(fmt.as_str(), dt_str.as_str());
        let mut obj = crate::engine::types::PhpObject::new(&resolved_class);
        obj.properties.insert(
            "timestamp".to_string(),
            Val::new(PhpValue::Long(ts.unwrap_or(0)), PhpType::Long),
        );
        let result_val = Val::new(PhpValue::Object(Box::new(obj)), PhpType::Object);
        if let Some(slot) = result_slot(op) {
            execute_data.set_temp(slot, result_val);
        }
        return Ok(ExecResult::Continue);
    }

    // Built-in Fiber::suspend() static method — sets the suspend flag so the
    // execution loop breaks. Returns the suspend value as a placeholder; the
    // actual return value (from resume()) is placed in the DoFCall result slot
    // by the resume handler.
    if resolved_class == "Fiber" && method_name.as_str() == "suspend" {
        let (base, _names_base) = execute_data.call_arg_stack.pop().unwrap_or((0, 0));
        let args: Vec<Val> = execute_data.call_args.drain(base..).collect();
        let suspend_val = args.first().map(clone_val)
            .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null));
        execute_data.fiber_suspend_requested = Some(suspend_val);
        // Return null as placeholder — resume() overwrites the result slot
        if let Some(slot) = result_slot(op) {
            execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
        }
        return Ok(ExecResult::Continue);
    }

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
        let arg_names: Vec<Option<String>> =
            execute_data.call_arg_names.drain(names_base..).collect();
        let arg_by_ref: Vec<bool> = execute_data.call_arg_by_ref.drain(base..).collect();
        let variadic = execute_data
            .class_table
            .get(&resolved_class)
            .and_then(|ce| ce.methods.get(method_name.as_str()))
            .and_then(|m| m.op_array.variadic_param.clone());
        let ref_params = execute_data
            .class_table
            .get(&resolved_class)
            .and_then(|ce| ce.methods.get(method_name.as_str()))
            .map(|m| m.op_array.ref_params.clone())
            .unwrap_or_default();
        bind_call_args(
            execute_data,
            &params,
            &args,
            &arg_names,
            &variadic,
            &ref_params,
            &arg_by_ref,
        );

        let mut method_op_array = OpArray::with_capacity(ops.len(), oparray_filename);
        method_op_array.ops = ops;
        let saved_try_depth = execute_data.try_stack.len();
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
        if let Some(er) = crate::engine::vm::exception_dispatch::propagate_after_call(
            execute_data,
            saved_try_depth,
        ) {
            return Ok(er);
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
    if let Some(ce) = execute_data.class_table.get(&resolved_class)
        && let Some(magic) = ce.methods.get("__callStatic")
    {
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
        if let Some(p0) = params.first() {
            execute_data.set_var(
                p0,
                Val::new(
                    PhpValue::String(Box::new(crate::engine::string::string_init(
                        method_name.as_str(),
                        false,
                    ))),
                    PhpType::String,
                ),
            );
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

    let _ = execute_data.call_arg_stack.pop();
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, Val::new(PhpValue::Long(0), PhpType::Null));
    }
    Ok(ExecResult::Continue)
}

#[inline]
pub fn execute_clone_obj(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
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

/// Unset a variable: `unset($var)`.  Removes the variable from the symbol
/// table (or the appropriate scope — global, ref-param, local).
#[inline]
pub fn execute_unset(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    if let PhpValue::String(ref s) = op.op1.value {
        let vname = s.as_str();
        let name = vname.strip_prefix('$').unwrap_or(vname);
        execute_data.unset_var(name);
    }
    Ok(ExecResult::Continue)
}

/// Unset an array element: `unset($arr[$key])`.
/// Removes the element at the given key from the array stored in op1.
#[inline]
pub fn execute_unset_dim(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let key_val = resolve_operand(&op.op2, execute_data);

    // Get the container, remove the key, write it back (clone-modify-set).
    if is_var_ref(&op.op1) {
        if let PhpValue::String(ref s) = op.op1.value {
            let vname = s.as_str();
            let name = vname.strip_prefix('$').unwrap_or(vname);
            let mut container = execute_data.get_var(name);
            if let PhpValue::Array(ref mut arr) = container.value {
                let key_str = crate::engine::operators::zval_get_string(&key_val);
                let key = crate::engine::string::string_init(key_str.as_str(), false);
                let _ = crate::engine::hash::hash_del(arr, &key);
            }
            execute_data.set_var(name, container);
        }
    } else if is_temp_ref(&op.op1)
        && let PhpValue::Long(slot_idx) = op.op1.value
    {
        let slot = slot_idx as usize;
        let mut container = execute_data.get_temp(slot);
        if let PhpValue::Array(ref mut arr) = container.value {
            let key_str = crate::engine::operators::zval_get_string(&key_val);
            let key = crate::engine::string::string_init(key_str.as_str(), false);
            let _ = crate::engine::hash::hash_del(arr, &key);
        }
        execute_data.set_temp(slot, container);
    }
    Ok(ExecResult::Continue)
}

/// Unset an object property: `unset($obj->prop)`.
/// If the property exists, remove it. If it doesn't exist and the class
/// defines `__unset`, invoke the magic method with the property name.
#[inline]
pub fn execute_unset_obj_prop(
    op: &Op,
    execute_data: &mut ExecuteData,
) -> Result<ExecResult, String> {
    let obj_val = resolve_operand(&op.op1, execute_data);
    let prop_name_val = resolve_operand(&op.op2, execute_data);
    let prop_name = crate::engine::operators::zval_get_string(&prop_name_val);

    if let PhpValue::Object(ref obj) = obj_val.value {
        let class_name = obj.class_name.clone();
        let has_prop = obj.properties.contains_key(prop_name.as_str());

        if has_prop {
            // Property exists: remove it (clone-modify-set pattern, same as AssignObjProp).
            if is_var_ref(&op.op1) {
                if let PhpValue::String(ref s) = op.op1.value {
                    let vname = s.as_str();
                    let name = vname.strip_prefix('$').unwrap_or(vname);
                    let mut obj_val = execute_data.get_var(name);
                    if let PhpValue::Object(ref mut obj) = obj_val.value {
                        obj.properties.remove(prop_name.as_str());
                    }
                    execute_data.set_var(name, obj_val);
                }
            } else if is_temp_ref(&op.op1)
                && let PhpValue::Long(slot_idx) = op.op1.value
            {
                let slot = slot_idx as usize;
                let mut obj_val = execute_data.get_temp(slot);
                if let PhpValue::Object(ref mut obj) = obj_val.value {
                    obj.properties.remove(prop_name.as_str());
                }
                execute_data.set_temp(slot, obj_val);
            }
        } else if let Some(ce) = execute_data.class_table.get(&class_name) {
            // Property doesn't exist: check for __unset magic method.
            if let Some(m) = ce.methods.get("__unset") {
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
                    .unwrap_or_else(|| format!("{}::__unset", class_name));

                let saved_current_op = execute_data.current_op;
                let saved_op_array = execute_data.op_array.take();
                let saved_script_dir = execute_data.current_script_dir.clone();
                let saved_magic_dir = execute_data.constants.get("__DIR__").map(clone_val);
                let saved_magic_file = execute_data.constants.get("__FILE__").map(clone_val);
                let saved_called_class = execute_data.called_class.clone();
                execute_data.called_class = Some(class_name.clone());
                execute_data.set_var("this", clone_val(&obj_val));

                let name_val = Val::new(
                    PhpValue::String(Box::new(crate::engine::string::string_init(
                        prop_name.as_str(),
                        false,
                    ))),
                    PhpType::String,
                );
                bind_call_args(
                    execute_data,
                    &params,
                    &[name_val],
                    &[None],
                    &None,
                    &[],
                    &[false],
                );

                let mut method_op_array = OpArray::with_capacity(ops.len(), file_label);
                method_op_array.ops = ops;
                let saved_try_depth = execute_data.try_stack.len();
                let (_status, _result) =
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
                if let Some(er) = crate::engine::vm::exception_dispatch::propagate_after_call(
                    execute_data,
                    saved_try_depth,
                ) {
                    return Ok(er);
                }
            }
        }
    }

    Ok(ExecResult::Continue)
}

/// Bind call arguments to parameters, supporting named args, variadic, and by-ref params
#[inline]
pub(crate) fn bind_call_args(
    execute_data: &mut ExecuteData,
    param_names: &[String],
    args: &[Val],
    arg_names: &[Option<String>],
    variadic_param: &Option<String>,
    ref_params: &[bool],
    arg_by_ref: &[bool],
) {
    let regular_count = if variadic_param.is_some() {
        param_names.len().saturating_sub(1)
    } else {
        param_names.len()
    };

    let mut bound = vec![false; regular_count];

    let bind_one =
        |execute_data: &mut ExecuteData, pos: usize, clean: &str, arg: &Val, arg_idx: usize| {
            if ref_params.get(pos).copied().unwrap_or(false)
                && arg_by_ref.get(arg_idx).copied().unwrap_or(false)
                && is_var_ref(arg)
                && let PhpValue::String(ref s) = arg.value
            {
                let caller = s.as_str();
                let caller_clean = caller.strip_prefix('$').unwrap_or(caller);
                execute_data
                    .ref_param_bindings
                    .insert(clean.to_string(), caller_clean.to_string());
                return;
            }
            execute_data.set_var(clean, clone_val(arg));
        };

    // First pass: bind named arguments
    for (i, name_opt) in arg_names.iter().enumerate() {
        if let Some(name) = name_opt
            && let Some(pos) = param_names[..regular_count]
                .iter()
                .position(|p| p.strip_prefix('$').unwrap_or(p) == name.as_str())
            && let Some(arg) = args.get(i)
        {
            let p = &param_names[pos];
            let clean = p.strip_prefix('$').unwrap_or(p);
            bind_one(execute_data, pos, clean, arg, i);
            bound[pos] = true;
        }
    }

    // Second pass: bind positional arguments to remaining params
    let mut param_idx = 0;
    for (i, name_opt) in arg_names.iter().enumerate() {
        if name_opt.is_none() {
            while param_idx < regular_count && bound[param_idx] {
                param_idx += 1;
            }
            if param_idx < regular_count
                && let Some(arg) = args.get(i)
            {
                let p = &param_names[param_idx];
                let clean = p.strip_prefix('$').unwrap_or(p);
                bind_one(execute_data, param_idx, clean, arg, i);
                bound[param_idx] = true;
                param_idx += 1;
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
                param_names[..regular_count]
                    .iter()
                    .position(|p| p.strip_prefix('$').unwrap_or(p) == name.as_str())
                    .is_none()
            } else {
                // Positional arg is extra if beyond regular_count
                let pos = arg_names[..i].iter().filter(|n| n.is_none()).count();
                pos >= regular_count
            };
            if is_extra {
                let _ =
                    crate::engine::hash::hash_add_or_update(&mut arr, None, idx, clone_val(arg), 0);
                idx += 1;
            }
        }
        let arr_val = Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array);
        let clean = var_name.strip_prefix('$').unwrap_or(var_name);
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
        Opcode::BwAnd => execute_bw_and(op, execute_data),
        Opcode::BwOr => execute_bw_or(op, execute_data),
        Opcode::BwXor => execute_bw_xor(op, execute_data),
        Opcode::BwNot => execute_bw_not(op, execute_data),
        Opcode::Sl => execute_sl(op, execute_data),
        Opcode::Sr => execute_sr(op, execute_data),
        Opcode::Spaceship => execute_spaceship(op, execute_data),
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
        Opcode::SendVarRef => execute_send_var_ref(op, execute_data),
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

// --- Previously no-op opcodes, now wired to real handlers ---

/// `AssignObj` — assign a value to an object property (op1=obj, op2=prop name,
/// result=value). This is the opcode-level form; the compiler currently emits
/// `FetchObjProp` + `Assign` for `$obj->prop = v`, but this handler ensures the
/// opcode is not a silent no-op if emitted directly.
#[inline]
pub fn execute_assign_obj(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.result, execute_data);
    let obj = resolve_operand(&op.op1, execute_data);
    if let PhpValue::Object(obj_inner) = &obj.value {
        let mut updated = crate::engine::types::PhpObject::new(&obj_inner.class_name);
        updated.properties = obj_inner.properties.clone();
        if let PhpValue::String(prop_name) = &op.op2.value {
            updated.properties.insert(prop_name.as_str().to_string(), clone_val(&val));
        }
        let new_obj = Val::new(PhpValue::Object(Box::new(updated)), PhpType::Object);
        // Write back to the variable/temp holding the object
        if is_var_ref(&op.op1)
            && let PhpValue::String(ref name) = op.op1.value
        {
            let n = name.as_str();
            let clean = n.strip_prefix('$').unwrap_or(n);
            execute_data.set_var(clean, new_obj);
        } else if is_temp_ref(&op.op1)
            && let PhpValue::Long(idx) = op.op1.value
        {
            execute_data.set_temp(idx as usize, new_obj);
        }
    }
    Ok(ExecResult::Continue)
}

/// `TypeCheck` — check the type of op1 against a type spec in op2 (string type name).
/// Result temp gets a boolean.
#[inline]
pub fn execute_type_check(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let type_name = if let PhpValue::String(ref s) = op.op2.value {
        s.as_str().to_lowercase()
    } else {
        String::new()
    };
    let result = match type_name.as_str() {
        "int" | "integer" | "long" => val.get_type() == PhpType::Long,
        "float" | "double" => val.get_type() == PhpType::Double,
        "string" => val.get_type() == PhpType::String,
        "bool" | "boolean" => val.get_type() == PhpType::True || val.get_type() == PhpType::False,
        "array" => val.get_type() == PhpType::Array,
        "object" => val.get_type() == PhpType::Object,
        "null" => val.get_type() == PhpType::Null,
        "numeric" => {
            let s = crate::engine::operators::zval_get_string(&val);
            s.as_str().parse::<f64>().is_ok() || val.get_type() == PhpType::Long || val.get_type() == PhpType::Double
        }
        _ => false,
    };
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, make_bool(result));
    }
    Ok(ExecResult::Continue)
}

/// `IsSet` — opcode-level isset(): check if op1 is set and not null.
#[inline]
pub fn execute_is_set(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let result = val.get_type() != PhpType::Null && val.get_type() != PhpType::Undef;
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, make_bool(result));
    }
    Ok(ExecResult::Continue)
}

/// `Empty` — opcode-level empty(): check if op1 is "empty" per PHP rules.
#[inline]
pub fn execute_empty(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let is_empty = match val.get_type() {
        PhpType::Null | PhpType::False | PhpType::Undef => true,
        PhpType::Long => crate::engine::operators::zval_get_long(&val) == 0,
        PhpType::Double => crate::engine::operators::zval_get_double(&val) == 0.0,
        PhpType::String => {
            let s = crate::engine::operators::zval_get_string(&val);
            s.as_str().is_empty() || s.as_str() == "0"
        }
        PhpType::Array => {
            if let PhpValue::Array(arr) = &val.value {
                arr.ar_data.is_empty()
            } else {
                false
            }
        }
        _ => false,
    };
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, make_bool(is_empty));
    }
    Ok(ExecResult::Continue)
}

/// `Count` — opcode-level count(): count elements in op1 (array or string length).
#[inline]
pub fn execute_count(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let count = match &val.value {
        PhpValue::Array(arr) => arr.ar_data.len() as i64,
        PhpValue::String(s) => s.as_str().chars().count() as i64,
        _ => 0,
    };
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, Val::new(PhpValue::Long(count), PhpType::Long));
    }
    Ok(ExecResult::Continue)
}

/// `Keys` — opcode-level array_keys(): return keys of op1 array.
#[inline]
pub fn execute_keys(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let mut result = crate::engine::types::PhpArray::new();
    if let PhpValue::Array(arr) = &val.value {
        for (idx, bucket) in (0_u64..).zip(&arr.ar_data) {
            let key_val = if let Some(ref k) = bucket.key {
                Val::new(
                    PhpValue::String(Box::new(crate::engine::string::string_init(k.as_str(), false))),
                    PhpType::String,
                )
            } else {
                Val::new(PhpValue::Long(idx as i64), PhpType::Long)
            };
            let _ = crate::engine::hash::hash_add_or_update(&mut result, None, idx, key_val, 0);
        }
    }
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, Val::new(PhpValue::Array(Box::new(result)), PhpType::Array));
    }
    Ok(ExecResult::Continue)
}

/// `Values` — opcode-level array_values(): return values of op1 array.
#[inline]
pub fn execute_values(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let val = resolve_operand(&op.op1, execute_data);
    let mut result = crate::engine::types::PhpArray::new();
    if let PhpValue::Array(arr) = &val.value {
        for (idx, bucket) in (0_u64..).zip(&arr.ar_data) {
            let _ = crate::engine::hash::hash_add_or_update(
                &mut result,
                None,
                idx,
                clone_val(&bucket.val),
                0,
            );
        }
    }
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, Val::new(PhpValue::Array(Box::new(result)), PhpType::Array));
    }
    Ok(ExecResult::Continue)
}

/// `ArrayDiff` — opcode-level array_diff(): return elements in op1 not in op2.
#[inline]
pub fn execute_array_diff(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let arr1 = resolve_operand(&op.op1, execute_data);
    let arr2 = resolve_operand(&op.op2, execute_data);
    let mut result = crate::engine::types::PhpArray::new();
    if let (PhpValue::Array(a1), PhpValue::Array(a2)) = (&arr1.value, &arr2.value) {
        let values2: Vec<String> = a2.ar_data.iter().map(|b| {
            crate::engine::operators::zval_get_string(&b.val).as_str().to_string()
        }).collect();
        for (idx, bucket) in (0_u64..).zip(&a1.ar_data) {
            let v = crate::engine::operators::zval_get_string(&bucket.val);
            if !values2.contains(&v.as_str().to_string()) {
                let _ = crate::engine::hash::hash_add_or_update(
                    &mut result,
                    None,
                    idx,
                    clone_val(&bucket.val),
                    0,
                );
            }
        }
    }
    if let Some(slot) = result_slot(op) {
        execute_data.set_temp(slot, Val::new(PhpValue::Array(Box::new(result)), PhpType::Array));
    }
    Ok(ExecResult::Continue)
}

/// `Yield` — suspend generator execution and produce a value.
/// op1 = value to yield, op2 = key (or null for auto-increment), result = temp for sent value.
/// Sets `generator_yield_requested` to break out of `execute_ex_returning`.
#[inline]
pub fn execute_yield(op: &Op, execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    let yield_value = resolve_operand(&op.op1, execute_data);
    let yield_key = resolve_operand(&op.op2, execute_data);
    // Signal the execution loop to break, carrying the yielded value and key.
    // We store the key in a separate field on ExecuteData.
    execute_data.generator_yield_requested = Some(yield_value);
    execute_data.generator_yield_key = Some(yield_key);
    Ok(ExecResult::Continue)
}
