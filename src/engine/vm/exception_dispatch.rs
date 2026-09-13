//! Runtime exception dispatch.
//!
//! Wires PHP exception semantics onto the opcode stream. `Throw` stashes the
//! thrown object and searches the active `try` regions (tracked in
//! `ExecuteData::try_stack`) for a matching `catch`, jumping into its body and
//! binding the exception object to the catch variable. If no enclosing catch
//! matches, the current frame is unwound (`ExecResult::Return`) with the
//! exception left in `ExecuteData::pending_exception`.
//!
//! Cross-function propagation: every call site that runs a callee op-array
//! (`execute_ex_returning` / nested `execute_ex`) calls
//! [`propagate_after_call`] after restoring the caller's state, which
//! re-dispatches the pending exception against the caller's try regions.
//! Frames keep unwinding until some frame catches, or the top-level runner
//! (CLI / dev server) reports the fatal error.
//!
//! `finally` blocks run on all paths: normal try completion, after a matched
//! catch, and during exception unwinding. The compiler routes try-body and
//! catch-body exit jumps through `FinallyBegin` when a finally block exists.
//! `FinallyEnd` re-dispatches a pending exception (if any) after the finally
//! body runs; on the normal path it falls through.

use super::execute_data::ExecuteData;
use super::opcodes::Opcode;
use crate::engine::types::{PhpType, PhpValue, Val};

/// Standard PHP exception/error class hierarchy (parent relationships).
/// Used when the thrown or caught class is a built-in Throwable not present in
/// the user class table.
fn standard_parent(class: &str) -> Option<&'static str> {
    match class {
        "Throwable" => None,
        "Exception" | "Error" => Some("Throwable"),
        "RuntimeException" | "LogicException" | "RuntimeExceptionBase" => Some("Exception"),
        "InvalidArgumentException"
        | "BadMethodCallException"
        | "BadFunctionCallException"
        | "OutOfBoundsException" => Some("LogicException"),
        "OverflowException"
        | "UnderflowException"
        | "OutOfRangeException"
        | "UnexpectedValueException"
        | "RangeException"
        | "DomainException"
        | "LengthException" => Some("RuntimeException"),
        "TypeError"
        | "ValueError"
        | "ArgumentCountError"
        | "ArithmeticError"
        | "DivisionByZeroError"
        | "ParseError"
        | "AssertionError"
        | "UnhandledMatchError" => Some("Error"),
        "PDOException" => Some("RuntimeException"),
        _ => None,
    }
}

/// True for built-in PHP Throwable classes handled by the standard hierarchy.
pub fn is_standard_throwable(class: &str) -> bool {
    class == "Throwable" || standard_parent(class).is_some()
}

/// True if `thrown` is the same class or a descendant of `catch_class`.
/// Walks the user class table's parent chain first, then falls back to the
/// standard PHP exception hierarchy for built-in Throwables.
pub fn exception_is_a(
    thrown: &str,
    catch_class: &str,
    class_table: &std::collections::HashMap<String, crate::engine::types::ClassEntry>,
) -> bool {
    if thrown == catch_class || catch_class == "Throwable" {
        return true;
    }
    // Walk user-defined parent chain.
    let mut current = thrown;
    let mut hops = 0;
    while hops < 64 {
        if current == catch_class {
            return true;
        }
        match class_table
            .get(current)
            .and_then(|ce| ce.parent_name.as_deref())
        {
            Some(parent) => {
                current = parent;
                hops += 1;
            }
            None => break,
        }
    }
    // Fall back to the standard hierarchy (handles built-in Throwables that are
    // not registered as user classes).
    current = thrown;
    let mut hops = 0;
    while hops < 64 {
        if current == catch_class {
            return true;
        }
        match standard_parent(current) {
            Some(parent) => {
                current = parent;
                hops += 1;
            }
            None => return false,
        }
    }
    false
}

/// Outcome of searching for a handler for a pending exception.
pub enum ExceptionOutcome {
    /// A catch matched: jump to `body_start` and bind `exception` to `var`.
    Caught { body_start: u32, var: String },
    /// No catch matched but a finally block exists: jump to `body_start`,
    /// keeping the exception pending for re-dispatch at `FinallyEnd`.
    FinallyRun { body_start: u32 },
    /// No enclosing catch or finally matched.
    Uncaught,
}

/// Extract (class_name, message) from a thrown value (object or otherwise).
pub fn thrown_class_and_message(val: &Val) -> (String, String) {
    match &val.value {
        PhpValue::Object(obj) => {
            let msg = obj
                .properties
                .get("message")
                .map(|v| {
                    crate::engine::operators::zval_get_string(v)
                        .as_str()
                        .to_string()
                })
                .unwrap_or_default();
            (obj.class_name.clone(), msg)
        }
        _ => (
            "Throwable".to_string(),
            crate::engine::operators::zval_get_string(val)
                .as_str()
                .to_string(),
        ),
    }
}

/// Read a string operand from an op (op1/op2), returning "" if not a string.
fn op_string(op: &crate::engine::vm::Op, which: Which) -> String {
    let v = match which {
        Which::Op1 => &op.op1,
        Which::Op2 => &op.op2,
    };
    if let PhpValue::String(s) = &v.value {
        s.as_str().to_string()
    } else {
        String::new()
    }
}

enum Which {
    Op1,
    Op2,
}

/// Collect the catches and optional finally belonging to the `try` whose first
/// catch is at `first_catch_idx`. Returns `(catches, finally_body_start)`.
fn collect_handlers(
    ops: &[crate::engine::vm::Op],
    first_catch_idx: usize,
) -> (Vec<(String, String, u32)>, Option<u32>) {
    let mut catches = Vec::new();
    let mut finally_body = None;
    let mut depth: i32 = 0;
    let mut i = first_catch_idx;
    while i < ops.len() {
        match ops[i].opcode {
            Opcode::TryCatchBegin => depth += 1,
            Opcode::TryCatchEnd => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            Opcode::CatchBegin if depth == 0 => {
                let class = op_string(&ops[i], Which::Op1);
                let var = op_string(&ops[i], Which::Op2);
                catches.push((class, var, (i + 1) as u32));
            }
            Opcode::CatchEnd if depth == 0 => {
                // If the next opcode is another CatchBegin or FinallyBegin,
                // keep scanning. Otherwise this was the last catch with no
                // following finally — stop.
                let next = ops.get(i + 1).map(|o| o.opcode);
                if next != Some(Opcode::CatchBegin) && next != Some(Opcode::FinallyBegin) {
                    break;
                }
            }
            Opcode::FinallyBegin if depth == 0 => {
                finally_body = Some((i + 1) as u32);
                break;
            }
            Opcode::FinallyEnd if depth == 0 => break,
            _ => {}
        }
        i += 1;
    }
    (catches, finally_body)
}

/// Search the active try stack (innermost first) for a matching catch.
///
/// The stack is shared across call frames, so entries found here may belong
/// to an OUTER frame. This function therefore searches **without** consuming
/// frames on failure; on a match it truncates the stack to drop the matched
/// try entry plus any stale inner entries above it. Entries whose op_array
/// filename does not match the current op_array are skipped — they belong to
/// a different frame and would otherwise be misinterpreted as local catches.
pub fn dispatch_exception(execute_data: &mut ExecuteData, thrown_class: &str) -> ExceptionOutcome {
    let first_catch_for = |try_idx: usize| -> u32 {
        execute_data
            .op_array
            .as_ref()
            .and_then(|arr| arr.ops.get(try_idx))
            .map(|op| op.extended_value)
            .unwrap_or(0)
    };

    let current_filename = execute_data
        .op_array
        .as_ref()
        .and_then(|arr| arr.filename.clone());

    let mut idx = execute_data.try_stack.len();
    while idx > 0 {
        idx -= 1;
        let (try_idx, ref try_filename) = execute_data.try_stack[idx];
        // Skip entries from a different op_array — they cannot have catches
        // matching an exception thrown in the current frame.
        if try_filename != &current_filename {
            continue;
        }
        let first_catch = first_catch_for(try_idx) as usize;
        let (catches, finally_body) = execute_data
            .op_array
            .as_ref()
            .map(|arr| collect_handlers(&arr.ops, first_catch))
            .unwrap_or_default();
        for (catch_class, var, body_start) in catches {
            if exception_is_a(thrown_class, &catch_class, &execute_data.class_table) {
                execute_data.try_stack.truncate(idx);
                return ExceptionOutcome::Caught { body_start, var };
            }
        }
        // No catch matched — run finally if present, keeping the exception
        // pending for re-dispatch at `FinallyEnd`.
        if let Some(body_start) = finally_body {
            execute_data.try_stack.truncate(idx);
            return ExceptionOutcome::FinallyRun { body_start };
        }
        // This try didn't catch it; keep searching outward.
    }
    ExceptionOutcome::Uncaught
}

/// Convenience: true if a value looks like an exception object.
pub fn is_throwable_object(val: &Val) -> bool {
    matches!(&val.value, PhpValue::Object(_)) && val.get_type() == PhpType::Object
}

/// Dispatch a method call on a built-in Throwable (`Exception`/`Error` tree).
/// These classes are not user-defined, so their getters are resolved here.
/// Returns `None` when `method` is not a known Throwable getter, letting the
/// caller fall through to normal undefined-method handling.
pub fn execute_throwable_getter(class: &str, method: &str, obj_val: &Val) -> Option<Val> {
    use crate::engine::types::{PhpArray, PhpType};

    if !is_standard_throwable(class) {
        return None;
    }
    let PhpValue::Object(obj) = &obj_val.value else {
        return None;
    };
    let prop = |name: &str| obj.properties.get(name).map(super::execute_data::clone_val);

    match method.to_ascii_lowercase().as_str() {
        "getmessage" => Some(prop("message").unwrap_or_else(|| {
            Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init("", false))),
                PhpType::String,
            )
        })),
        "getcode" => {
            Some(prop("code").unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Long)))
        }
        "getfile" => Some(prop("file").unwrap_or_else(|| {
            Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init("", false))),
                PhpType::String,
            )
        })),
        "getline" => {
            Some(prop("line").unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Long)))
        }
        "getprevious" => {
            Some(prop("previous").unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null)))
        }
        "gettrace" => Some(Val::new(
            PhpValue::Array(Box::new(PhpArray::new())),
            PhpType::Array,
        )),
        "gettraceasstring" => Some(Val::new(
            PhpValue::String(Box::new(crate::engine::string::string_init(
                "#0 {main}",
                false,
            ))),
            PhpType::String,
        )),
        _ => None,
    }
}

/// Re-dispatch a pending exception against the CALLER's frame after a callee
/// exited with the exception uncaught.
///
/// Call sites that run a callee op-array capture `execute_data.try_stack.len()`
/// before the call and invoke this after restoring the caller's state
/// (`op_array`, `current_op`, symbol tables, …). Returns `Some(ExecResult)`
/// for the calling opcode handler to return — `Jump` into a matching catch
/// body (exception consumed), or `Return` to keep unwinding with the exception
/// still pending. Returns `None` when no exception is pending.
pub fn propagate_after_call(
    execute_data: &mut ExecuteData,
    saved_try_depth: usize,
) -> Option<crate::engine::vm::execute_data::ExecResult> {
    use crate::engine::vm::execute_data::ExecResult;

    // Drop any try frames the aborted callee left behind.
    let pending = execute_data.pending_exception.take()?;
    execute_data.try_stack.truncate(saved_try_depth);
    let thrown = pending;
    let (class, _message) = thrown_class_and_message(&thrown);
    match dispatch_exception(execute_data, &class) {
        ExceptionOutcome::Caught { body_start, var } => {
            execute_data.set_var(&var, thrown);
            Some(ExecResult::Jump(body_start))
        }
        ExceptionOutcome::FinallyRun { body_start } => {
            // Keep the exception pending; FinallyEnd will re-dispatch it.
            execute_data.pending_exception = Some(thrown);
            Some(ExecResult::Jump(body_start))
        }
        ExceptionOutcome::Uncaught => {
            // Still uncaught: leave pending for the next outer frame (or the
            // top-level runner) and unwind this frame too.
            execute_data.pending_exception = Some(thrown);
            Some(ExecResult::Return(Val::new(
                PhpValue::Long(0),
                PhpType::Null,
            )))
        }
    }
}
