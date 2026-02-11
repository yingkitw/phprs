//! Main VM execution loop

use crate::engine::types::PhpResult;
use super::opcodes::{Op, OpArray};
use super::execute_data::{clone_val, ExecResult, ExecuteData};
use super::handlers::execute_opcode;

/// Execute op array and capture return value
pub fn execute_ex_returning(execute_data: &mut ExecuteData, op_array: &OpArray) -> (PhpResult, Option<crate::engine::types::Val>) {
    let mut new_op_array = OpArray::new(op_array.filename.clone().unwrap_or_default());
    for op in &op_array.ops {
        new_op_array.add_op(Op::new(
            op.opcode, clone_val(&op.op1), clone_val(&op.op2), clone_val(&op.result), op.extended_value,
        ));
    }
    execute_data.op_array = Some(new_op_array);
    execute_data.current_op = 0;

    while let Some(op) = op_array.ops.get(execute_data.current_op) {
        match execute_opcode(op, execute_data) {
            Ok(ExecResult::Continue) => { execute_data.current_op += 1; }
            Ok(ExecResult::Jump(target)) => { execute_data.current_op = target as usize; }
            Ok(ExecResult::Return(value)) => { return (PhpResult::Success, Some(value)); }
            Err(e) => {
                eprintln!("Error executing opcode: {}", e);
                return (PhpResult::Failure, None);
            }
        }
    }
    (PhpResult::Success, None)
}

/// Execute op array (compiled script)
pub fn execute_ex(execute_data: &mut ExecuteData, op_array: &OpArray) -> PhpResult {
    // Copy op array into execute_data
    let mut new_op_array = OpArray::new(op_array.filename.clone().unwrap_or_default());
    for op in &op_array.ops {
        new_op_array.add_op(Op::new(
            op.opcode,
            clone_val(&op.op1),
            clone_val(&op.op2),
            clone_val(&op.result),
            op.extended_value,
        ));
    }

    execute_data.op_array = Some(new_op_array);
    execute_data.current_op = 0;

    // Transfer class table from op_array to execute_data
    for (name, ce) in &op_array.class_table {
        if !execute_data.class_table.contains_key(name) {
            let mut new_ce = crate::engine::types::ClassEntry::new(name);
            new_ce.parent_name = ce.parent_name.clone();
            for (prop_name, prop_val) in &ce.default_properties {
                new_ce.default_properties.insert(prop_name.clone(), clone_val(prop_val));
            }
            for (method_name, method) in &ce.methods {
                let mut new_ops = Vec::new();
                for op in &method.op_array.ops {
                    new_ops.push(Op::new(op.opcode, clone_val(&op.op1), clone_val(&op.op2), clone_val(&op.result), op.extended_value));
                }
                let mut new_op_arr = OpArray::new(format!("{}::{}", name, method_name));
                new_op_arr.ops = new_ops;
                new_ce.methods.insert(method_name.clone(), crate::engine::types::ClassMethod {
                    name: method.name.clone(),
                    visibility: method.visibility,
                    is_static: method.is_static,
                    params: method.params.clone(),
                    op_array: new_op_arr,
                });
            }
            execute_data.class_table.insert(name.clone(), new_ce);
        }
    }

    while let Some(op) = op_array.ops.get(execute_data.current_op) {
        match execute_opcode(op, execute_data) {
            Ok(ExecResult::Continue) => {
                execute_data.current_op += 1;
            }
            Ok(ExecResult::Jump(target)) => {
                execute_data.current_op = target as usize;
            }
            Ok(ExecResult::Return(_value)) => {
                return PhpResult::Success;
            }
            Err(e) => {
                eprintln!("Error executing opcode: {}", e);
                return PhpResult::Failure;
            }
        }
    }

    PhpResult::Success
}
