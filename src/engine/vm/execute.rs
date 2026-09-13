//! Main VM execution loop

use super::execute_data::{ExecResult, ExecuteData, clone_val};

use super::opcodes::{Op, OpArray, Opcode};
use crate::engine::string::string_init;
use crate::engine::types::{PhpResult, PhpType, PhpValue, Val};
use std::sync::OnceLock;

/// One slot in the opcode dispatch table.
type OpcodeHandler = fn(&Op, &mut ExecuteData) -> Result<ExecResult, String>;

/// Table size derives from the `Opcode::Last` sentinel — no manual bumping.
const OPCODE_COUNT: usize = Opcode::COUNT;

// Performance-optimized dispatch table
static DISPATCH_TABLE: OnceLock<[OpcodeHandler; OPCODE_COUNT]> = OnceLock::new();

/// Initialize the dispatch table for computed goto style dispatch
#[inline]
fn init_dispatch_table() {
    DISPATCH_TABLE.get_or_init(|| {
        let mut table: [OpcodeHandler; OPCODE_COUNT] = [default_handler; OPCODE_COUNT];

        // Import optimized handlers
        use super::dispatch_handlers::*;

        table[Opcode::Nop as usize] = execute_nop;
        table[Opcode::Add as usize] = execute_add;
        table[Opcode::Sub as usize] = execute_sub;
        table[Opcode::Mul as usize] = execute_mul;
        table[Opcode::Div as usize] = execute_div;
        table[Opcode::Mod as usize] = execute_mod;
        table[Opcode::Pow as usize] = execute_pow;
        table[Opcode::BwAnd as usize] = execute_bw_and;
        table[Opcode::BwOr as usize] = execute_bw_or;
        table[Opcode::BwXor as usize] = execute_bw_xor;
        table[Opcode::BwNot as usize] = execute_bw_not;
        table[Opcode::Sl as usize] = execute_sl;
        table[Opcode::Sr as usize] = execute_sr;
        table[Opcode::BoolNot as usize] = execute_bool_not;
        table[Opcode::BoolAnd as usize] = execute_bool_and;
        table[Opcode::BoolOr as usize] = execute_bool_or;
        table[Opcode::BoolXor as usize] = execute_bool_xor;
        table[Opcode::Concat as usize] = execute_concat;
        table[Opcode::Assign as usize] = execute_assign;
        table[Opcode::AssignDim as usize] = execute_assign_dim;
        table[Opcode::Echo as usize] = execute_echo;
        table[Opcode::Return as usize] = execute_return;
        table[Opcode::Jmp as usize] = execute_jmp;
        table[Opcode::JmpZ as usize] = execute_jmpz;
        table[Opcode::JmpNZ as usize] = execute_jmpnz;
        table[Opcode::InitFCall as usize] = execute_init_fcall;
        table[Opcode::DoFCall as usize] = execute_do_fcall;
        table[Opcode::FetchVar as usize] = execute_fetch_var;
        table[Opcode::SendVal as usize] = execute_send_val;
        table[Opcode::SendValNamed as usize] = execute_send_val_named;
        table[Opcode::SendVarRef as usize] = execute_send_var_ref;
        table[Opcode::Spaceship as usize] = execute_spaceship;
        table[Opcode::BindGlobal as usize] = execute_bind_global;
        table[Opcode::Include as usize] = execute_include;
        table[Opcode::InitArray as usize] = execute_init_array;
        table[Opcode::AddArrayElement as usize] = execute_add_array_element;
        table[Opcode::FetchDim as usize] = execute_fetch_dim;
        table[Opcode::NewObj as usize] = execute_new_obj;
        table[Opcode::FetchObjProp as usize] = execute_fetch_obj_prop;
        table[Opcode::AssignObjProp as usize] = execute_assign_obj_prop;
        table[Opcode::AssignStaticProp as usize] = execute_assign_static_prop;
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
        table[Opcode::FeReset as usize] = execute_fe_reset;
        table[Opcode::FeFetch as usize] = execute_fe_fetch;
        table[Opcode::FetchStaticProp as usize] = execute_fetch_static_prop;
        table[Opcode::DoStaticCall as usize] = execute_do_static_call;
        table[Opcode::CloneObj as usize] = execute_clone_obj;
        table[Opcode::UnsetObjProp as usize] = execute_unset_obj_prop;
        table[Opcode::Unset as usize] = execute_unset;
        table[Opcode::UnsetDim as usize] = execute_unset_dim;

        // Previously no-op opcodes — now wired to real handlers
        table[Opcode::AssignObj as usize] = execute_assign_obj;
        table[Opcode::TypeCheck as usize] = execute_type_check;
        table[Opcode::IsSet as usize] = execute_is_set;
        table[Opcode::Empty as usize] = execute_empty;
        table[Opcode::Count as usize] = execute_count;
        table[Opcode::Keys as usize] = execute_keys;
        table[Opcode::Values as usize] = execute_values;
        table[Opcode::ArrayDiff as usize] = execute_array_diff;
        table[Opcode::Yield as usize] = execute_yield;

        // Exception handling
        table[Opcode::TryCatchBegin as usize] = execute_try_catch_begin;
        table[Opcode::TryCatchEnd as usize] = execute_try_catch_end;
        table[Opcode::CatchBegin as usize] = execute_catch_marker;
        table[Opcode::CatchEnd as usize] = execute_catch_marker;
        table[Opcode::FinallyBegin as usize] = execute_catch_marker;
        table[Opcode::FinallyEnd as usize] = execute_finally_end;
        table[Opcode::Throw as usize] = execute_throw;

        table
    });
}

#[inline]
fn default_handler(_op: &Op, _execute_data: &mut ExecuteData) -> Result<ExecResult, String> {
    Ok(ExecResult::Continue)
}

/// Parent directory for relative includes. `None` keeps the caller's `current_script_dir`
/// (e.g. empty filename or synthetic `Class::method` labels).
fn script_dir_from_oparray_filename(filename: Option<&String>) -> Option<String> {
    let f = filename?;
    if f.is_empty() {
        return None;
    }
    if f.contains("::") && !f.contains('/') && !f.contains('\\') {
        return None;
    }
    std::path::Path::new(f.as_str())
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
}

fn apply_script_path_constants(execute_data: &mut ExecuteData, op_array: &OpArray) {
    if let Some(dir) = script_dir_from_oparray_filename(op_array.filename.as_ref()) {
        execute_data.current_script_dir = Some(dir.clone());
        let dir_val = Val::new(
            PhpValue::String(Box::new(string_init(&dir, false))),
            PhpType::String,
        );
        execute_data
            .constants
            .insert("__DIR__".to_string(), dir_val);
        if let Some(ref path) = op_array.filename {
            let file_val = Val::new(
                PhpValue::String(Box::new(string_init(path.as_str(), false))),
                PhpType::String,
            );
            execute_data
                .constants
                .insert("__FILE__".to_string(), file_val);
        }
    }
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
    apply_script_path_constants(execute_data, op_array);

    // Save the caller's try_stack; the callee starts with a fresh stack so
    // its exception dispatch cannot accidentally match the caller's try
    // entries (whose opcode indices refer to the caller's op_array, not the
    // callee's).  Restored before returning so propagate_after_call sees the
    // caller's entries again.
    let saved_try_stack = std::mem::take(&mut execute_data.try_stack);

    // Optimized execution loop with direct dispatch
    let ops = &op_array.ops;
    let len = ops.len();

    let mut iteration_count = 0;
    let max_iterations = 1_000_000; // Safety limit to prevent infinite loops

    let result: (PhpResult, Option<crate::engine::types::Val>) = loop {
        if execute_data.current_op >= len {
            finalize_request(execute_data);
            if execute_data.pending_exception.is_some() {
                break (PhpResult::Failure, None);
            }
            break (PhpResult::Success, None);
        }
        iteration_count += 1;
        if iteration_count > max_iterations {
            eprintln!(
                "VM execution exceeded maximum iterations ({}), possible infinite loop",
                max_iterations
            );
            break (PhpResult::Failure, None);
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
                    finalize_request(execute_data);
                    break (PhpResult::Success, None);
                }
                // Fiber::suspend() requested — break out of the loop.
                // The fiber dispatch code saves the VM state after this returns.
                if execute_data.fiber_suspend_requested.is_some() {
                    break (PhpResult::Success, None);
                }
                // Generator yield requested — break out of the loop.
                // The generator dispatch code saves the VM state after this returns.
                if execute_data.generator_yield_requested.is_some() {
                    break (PhpResult::Success, None);
                }
            }
            Ok(ExecResult::Jump(target)) => {
                execute_data.current_op = target as usize;
            }
            Ok(ExecResult::Return(value)) => {
                finalize_request(execute_data);
                break (PhpResult::Success, Some(value));
            }
            Err(e) => {
                eprintln!("Error executing opcode: {}", e);
                break (PhpResult::Failure, None);
            }
        }
    };

    execute_data.try_stack = saved_try_stack;
    result
}

/// Resume execution from the current VM state (for Fiber resume).
/// Unlike `execute_ex_returning`, this does NOT reset `current_op` or clone
/// the op_array — it continues from wherever `execute_data.current_op` points.
pub fn execute_ex_resume(execute_data: &mut ExecuteData) -> (PhpResult, Option<crate::engine::types::Val>) {
    init_dispatch_table();

    // Use the op_array already installed in execute_data (set by the caller).
    // Don't reset current_op — continue from where we left off.
    let saved_try_stack = std::mem::take(&mut execute_data.try_stack);

    // Get a reference to the ops slice from the installed op_array.
    let ops: Vec<Op> = execute_data.op_array.as_ref()
        .map(|oa| oa.ops.iter().map(|op| Op::new(
            op.opcode,
            clone_val(&op.op1),
            clone_val(&op.op2),
            clone_val(&op.result),
            op.extended_value,
        )).collect())
        .unwrap_or_default();
    let len = ops.len();

    let mut iteration_count = 0;
    let max_iterations = 1_000_000;

    let result: (PhpResult, Option<crate::engine::types::Val>) = loop {
        if execute_data.current_op >= len {
            finalize_request(execute_data);
            if execute_data.pending_exception.is_some() {
                break (PhpResult::Failure, None);
            }
            break (PhpResult::Success, None);
        }
        iteration_count += 1;
        if iteration_count > max_iterations {
            break (PhpResult::Failure, None);
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
                    finalize_request(execute_data);
                    break (PhpResult::Success, None);
                }
                if execute_data.fiber_suspend_requested.is_some() {
                    break (PhpResult::Success, None);
                }
                if execute_data.generator_yield_requested.is_some() {
                    break (PhpResult::Success, None);
                }
            }
            Ok(ExecResult::Jump(target)) => {
                execute_data.current_op = target as usize;
            }
            Ok(ExecResult::Return(value)) => {
                finalize_request(execute_data);
                break (PhpResult::Success, Some(value));
            }
            Err(e) => {
                eprintln!("Error executing opcode: {}", e);
                break (PhpResult::Failure, None);
            }
        }
    };

    execute_data.try_stack = saved_try_stack;
    result
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
    apply_script_path_constants(execute_data, op_array);

    // Optimized class table transfer with capacity hints
    execute_data.class_table.reserve(op_array.class_table.len());
    for (name, ce) in &op_array.class_table {
        if !execute_data.class_table.contains_key(name) {
            let mut new_ce = crate::engine::types::ClassEntry::new(name);
            new_ce.parent_name = ce.parent_name.clone();
            new_ce.is_final = ce.is_final;
            new_ce.is_abstract = ce.is_abstract;
            new_ce.is_enum = ce.is_enum;
            new_ce.enum_base_type = ce.enum_base_type;
            new_ce
                .default_properties
                .reserve(ce.default_properties.len());
            new_ce.static_properties.reserve(ce.static_properties.len());
            new_ce.property_flags.reserve(ce.property_flags.len());
            new_ce.constants.reserve(ce.constants.len());
            new_ce.methods.reserve(ce.methods.len());

            for (prop_name, prop_val) in &ce.default_properties {
                new_ce
                    .default_properties
                    .insert(prop_name.clone(), clone_val(prop_val));
            }
            for (prop_name, prop_val) in &ce.static_properties {
                new_ce
                    .static_properties
                    .insert(prop_name.clone(), clone_val(prop_val));
            }
            for (prop_name, flags) in &ce.property_flags {
                new_ce.property_flags.insert(prop_name.clone(), *flags);
            }
            for (const_name, const_val) in &ce.constants {
                new_ce
                    .constants
                    .insert(const_name.clone(), clone_val(const_val));
            }
            for const_name in &ce.final_constants {
                new_ce.final_constants.insert(const_name.clone());
            }
            for (method_name, method) in &ce.methods {
                let method_file = method
                    .op_array
                    .filename
                    .clone()
                    .filter(|f| !f.is_empty())
                    .unwrap_or_else(|| format!("{}::{}", name, method_name));
                let mut new_op_arr = OpArray::with_capacity(method.op_array.ops.len(), method_file);
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

    // Save the caller's try_stack; the callee starts with a fresh stack so
    // its exception dispatch cannot accidentally match the caller's try
    // entries (whose opcode indices refer to the caller's op_array, not the
    // callee's).  Restored before returning so propagate_after_call sees the
    // caller's entries again.
    let saved_try_stack = std::mem::take(&mut execute_data.try_stack);

    // Optimized execution loop with direct dispatch
    let ops = &op_array.ops;
    let len = ops.len();

    let mut iteration_count = 0;
    let max_iterations = 1_000_000; // Safety limit to prevent infinite loops

    let result: PhpResult = loop {
        if execute_data.current_op >= len {
            finalize_request(execute_data);
            if execute_data.pending_exception.is_some() {
                break PhpResult::Failure;
            }
            break PhpResult::Success;
        }
        iteration_count += 1;
        if iteration_count > max_iterations {
            eprintln!(
                "VM execution exceeded maximum iterations ({}), possible infinite loop",
                max_iterations
            );
            break PhpResult::Failure;
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
                    finalize_request(execute_data);
                    break PhpResult::Success;
                }
            }
            Ok(ExecResult::Jump(target)) => {
                execute_data.current_op = target as usize;
            }
            Ok(ExecResult::Return(_value)) => {
                finalize_request(execute_data);
                if execute_data.pending_exception.is_some() {
                    break PhpResult::Failure;
                }
                break PhpResult::Success;
            }
            Err(e) => {
                eprintln!("Error executing opcode: {}", e);
                break PhpResult::Failure;
            }
        }
    };

    execute_data.try_stack = saved_try_stack;
    result
}

fn finalize_request(execute_data: &mut ExecuteData) {
    if let Err(e) = crate::php::session::session_write_close(execute_data) {
        eprintln!("Session write error: {e}");
    }
}

#[cfg(test)]
mod dispatch_table_tests {
    use super::*;

    /// Opcode indices that intentionally have no handler (documented no-ops;
    /// their builtin equivalents work — see MEMORY.md §2). Any new entry here
    /// must be justified; anything NOT listed here and not registered in
    /// `init_dispatch_table` fails this test, so silent no-op drift is caught.
    const DOCUMENTED_NO_OPS: &[usize] = &[];

    #[test]
    fn test_dispatch_table_covers_every_real_opcode() {
        init_dispatch_table();
        let table = DISPATCH_TABLE.get().expect("dispatch table initialized");
        let mut unregistered: Vec<usize> = Vec::new();
        for (idx, handler) in table.iter().enumerate().take(Opcode::COUNT) {
            if DOCUMENTED_NO_OPS.contains(&idx) {
                continue;
            }
            // Pointer equality: registered handlers differ from the default.
            if std::ptr::eq(*handler as *const (), default_handler as *const ()) {
                unregistered.push(idx);
            }
        }
        assert!(
            unregistered.is_empty(),
            "opcode indices without a dispatch handler and not on the documented no-op list: {unregistered:?}"
        );
    }

    #[test]
    fn test_count_sentinel_covers_last_real_opcode() {
        // The sentinel must be strictly greater than every real opcode so the
        // table size always covers all dispatchable indices. Update when a
        // new opcode is added (the compiler will point here).
        assert_eq!(Opcode::COUNT, (Opcode::Yield as usize) + 1);
    }
}
