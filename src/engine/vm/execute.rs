//! Main VM execution loop

use super::execute_data::{clone_val, ExecResult, ExecuteData};

use super::opcodes::{Op, OpArray, Opcode};
use crate::engine::string::string_init;
use crate::engine::types::{PhpResult, PhpType, PhpValue, Val};
use std::sync::OnceLock;

// Performance-optimized dispatch table
static DISPATCH_TABLE: OnceLock<[fn(&Op, &mut ExecuteData) -> Result<ExecResult, String>; 73]> =
    OnceLock::new();

/// Initialize the dispatch table for computed goto style dispatch
#[inline]
fn init_dispatch_table() {
    DISPATCH_TABLE.get_or_init(|| {
        let mut table: [fn(&Op, &mut ExecuteData) -> Result<ExecResult, String>; 73] =
            [default_handler; 73];

        // Import optimized handlers
        use super::dispatch_handlers::*;

        table[Opcode::Nop as usize] = execute_nop;
        table[Opcode::Add as usize] = execute_add;
        table[Opcode::Sub as usize] = execute_sub;
        table[Opcode::Mul as usize] = execute_mul;
        table[Opcode::Div as usize] = execute_div;
        table[Opcode::Mod as usize] = execute_mod;
        table[Opcode::Pow as usize] = execute_pow;
        table[Opcode::Concat as usize] = execute_concat;
        table[Opcode::Assign as usize] = execute_assign;
        table[Opcode::Echo as usize] = execute_echo;
        table[Opcode::Return as usize] = execute_return;
        table[Opcode::Jmp as usize] = execute_jmp;
        table[Opcode::JmpZ as usize] = execute_jmpz;
        table[Opcode::JmpNZ as usize] = execute_jmpnz;
        table[Opcode::InitFCall as usize] = execute_init_fcall;
        table[Opcode::DoFCall as usize] = execute_do_fcall;
        table[Opcode::FetchVar as usize] = execute_fetch_var;
        table[Opcode::SendVal as usize] = execute_send_val;
        table[Opcode::Include as usize] = execute_include;
        table[Opcode::InitArray as usize] = execute_init_array;
        table[Opcode::AddArrayElement as usize] = execute_add_array_element;
        table[Opcode::FetchDim as usize] = execute_fetch_dim;
        table[Opcode::NewObj as usize] = execute_new_obj;
        table[Opcode::FetchObjProp as usize] = execute_fetch_obj_prop;
        table[Opcode::AssignObjProp as usize] = execute_assign_obj_prop;
        table[Opcode::InitMethodCall as usize] = execute_init_method_call;
        table[Opcode::DoMethodCall as usize] = execute_do_method_call;
        table[Opcode::Coalesce as usize] = execute_coalesce;
        table[Opcode::QmAssign as usize] = execute_qm_assign;
        table[Opcode::JmpNullZ as usize] = execute_jmp_null_z;
        table[Opcode::IsIdentical as usize] = execute_is_identical;
        table[Opcode::IsNotIdentical as usize] = execute_is_not_identical;
        table[Opcode::IsEqual as usize] = execute_is_equal;
        table[Opcode::IsNotEqual as usize] = execute_is_not_equal;
        table[Opcode::IsSmaller as usize] = execute_is_smaller;
        table[Opcode::IsSmallerOrEqual as usize] = execute_is_smaller_or_equal;

        table
    });
}

#[inline]
fn default_handler(_op: &Op, _execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    Ok(ExecResult::Continue)
}

/// Execute op array and capture return value (optimized)
pub fn execute_ex_returning(
    execute_data: &mut ExecuteData,
    op_array: &OpArray,
) -> (PhpResult, Option<crate::engine::types::Val>) {
    init_dispatch_table();

    // Pre-allocate with exact capacity to avoid reallocations
    let mut new_op_array = OpArray::with_capacity(
        op_array.ops.len(),
        op_array.filename.clone().unwrap_or_default(),
    );
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
    execute_data.current_script_dir = op_array.filename.as_ref().and_then(|f| {
        std::path::Path::new(f)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
    });
    if let Some(ref dir) = execute_data.current_script_dir {
        let dir_val = Val::new(
            PhpValue::String(Box::new(string_init(dir, false))),
            PhpType::String,
        );
        execute_data
            .constants
            .insert("__DIR__".to_string(), dir_val);
    }
    if let Some(ref path) = op_array.filename {
        let file_val = Val::new(
            PhpValue::String(Box::new(string_init(path, false))),
            PhpType::String,
        );
        execute_data
            .constants
            .insert("__FILE__".to_string(), file_val);
    }

    // Optimized execution loop with direct dispatch
    let ops = &op_array.ops;
    let len = ops.len();

    // Initialize dispatch table
    init_dispatch_table();

    let mut iteration_count = 0;
    let max_iterations = 1_000_000; // Safety limit to prevent infinite loops

    while execute_data.current_op < len {
        iteration_count += 1;
        if iteration_count > max_iterations {
            eprintln!(
                "VM execution exceeded maximum iterations ({}), possible infinite loop",
                max_iterations
            );
            return (PhpResult::Failure, None);
        }

        let op = unsafe { ops.get_unchecked(execute_data.current_op) };
        let result = unsafe {
            let dispatch_table = DISPATCH_TABLE.get().unwrap_unchecked();
            let handler = dispatch_table.get(op.opcode as usize).unwrap_unchecked();
            handler(op, execute_data)
        };

        match result {
            Ok(ExecResult::Continue) => {
                execute_data.current_op += 1;
                if execute_data.exit_requested.is_some() {
                    return (PhpResult::Success, None);
                }
            }
            Ok(ExecResult::Jump(target)) => {
                execute_data.current_op = target as usize;
            }
            Ok(ExecResult::Return(value)) => {
                return (PhpResult::Success, Some(value));
            }
            Err(e) => {
                eprintln!("Error executing opcode: {}", e);
                return (PhpResult::Failure, None);
            }
        }
    }
    (PhpResult::Success, None)
}

/// Execute op array (compiled script) - optimized
pub fn execute_ex(execute_data: &mut ExecuteData, op_array: &OpArray) -> PhpResult {
    init_dispatch_table();

    // Pre-allocate with exact capacity to avoid reallocations
    let mut new_op_array = OpArray::with_capacity(
        op_array.ops.len(),
        op_array.filename.clone().unwrap_or_default(),
    );
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
    execute_data.current_script_dir = op_array.filename.as_ref().and_then(|f| {
        std::path::Path::new(f)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
    });
    // Set __DIR__ magic constant for WordPress/script compatibility
    if let Some(ref dir) = execute_data.current_script_dir {
        let dir_val = Val::new(
            PhpValue::String(Box::new(string_init(dir, false))),
            PhpType::String,
        );
        execute_data
            .constants
            .insert("__DIR__".to_string(), dir_val);
    }
    if let Some(ref path) = op_array.filename {
        let file_val = Val::new(
            PhpValue::String(Box::new(string_init(path, false))),
            PhpType::String,
        );
        execute_data
            .constants
            .insert("__FILE__".to_string(), file_val);
    }

    // Optimized class table transfer with capacity hints
    execute_data.class_table.reserve(op_array.class_table.len());
    for (name, ce) in &op_array.class_table {
        if !execute_data.class_table.contains_key(name) {
            let mut new_ce = crate::engine::types::ClassEntry::new(name);
            new_ce.parent_name = ce.parent_name.clone();
            new_ce
                .default_properties
                .reserve(ce.default_properties.len());
            new_ce.methods.reserve(ce.methods.len());

            for (prop_name, prop_val) in &ce.default_properties {
                new_ce
                    .default_properties
                    .insert(prop_name.clone(), clone_val(prop_val));
            }
            for (method_name, method) in &ce.methods {
                let mut new_op_arr = OpArray::with_capacity(
                    method.op_array.ops.len(),
                    format!("{}::{}", name, method_name),
                );
                new_op_arr.ops = method
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
                new_ce.methods.insert(
                    method_name.clone(),
                    crate::engine::types::ClassMethod {
                        name: method.name.clone(),
                        visibility: method.visibility,
                        is_static: method.is_static,
                        params: method.params.clone(),
                        op_array: new_op_arr,
                    },
                );
            }
            execute_data.class_table.insert(name.clone(), new_ce);
        }
    }

    // Optimized execution loop with direct dispatch
    let ops = &op_array.ops;
    let len = ops.len();

    let mut iteration_count = 0;
    let max_iterations = 1_000_000; // Safety limit to prevent infinite loops

    while execute_data.current_op < len {
        iteration_count += 1;
        if iteration_count > max_iterations {
            eprintln!(
                "VM execution exceeded maximum iterations ({}), possible infinite loop",
                max_iterations
            );
            return PhpResult::Failure;
        }

        let op = unsafe { ops.get_unchecked(execute_data.current_op) };
        let result = unsafe {
            let dispatch_table = DISPATCH_TABLE.get().unwrap_unchecked();
            let handler = dispatch_table.get(op.opcode as usize).unwrap_unchecked();
            handler(op, execute_data)
        };

        match result {
            Ok(ExecResult::Continue) => {
                execute_data.current_op += 1;
                if execute_data.exit_requested.is_some() {
                    return PhpResult::Success;
                }
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
