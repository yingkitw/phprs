//! Runtime exception dispatch.
//!
//! Wires PHP exception semantics onto the opcode stream. `Throw` stashes the
//! thrown object and searches the active `try` regions (tracked in
//! `ExecuteData::try_stack`) for a matching `catch`, jumping into its body and
//! binding the exception object to the catch variable. If no enclosing catch
//! matches, the exception is uncaught.
//!
//! Scope: exceptions are dispatched within a single op-array (function/script).
//! Cross-function propagation (throw in a callee caught by a caller) is not yet
//! supported — the call machinery saves/restores `try_stack` so a throw in a
//! callee with no local catch surfaces as an uncaught error rather than
//! mis-dispatching against the caller's catches.

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
    /// No enclosing catch matched.
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

/// Collect the catches belonging to the `try` whose first catch is at
/// `first_catch_idx`. Returns `(catch_class, var_name, body_start_index)` for
/// each, skipping over any nested try/catch embedded in a catch body.
fn collect_catches(
    ops: &[crate::engine::vm::Op],
    first_catch_idx: usize,
) -> Vec<(String, String, u32)> {
    let mut out = Vec::new();
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
                out.push((class, var, (i + 1) as u32));
            }
            Opcode::CatchEnd if depth == 0 => {
                // If the next opcode is not another CatchBegin, this was the
                // last catch of this try.
                let next_is_catch = ops
                    .get(i + 1)
                    .map(|o| o.opcode == Opcode::CatchBegin)
                    .unwrap_or(false);
                if !next_is_catch {
                    break;
                }
            }
            Opcode::FinallyBegin | Opcode::FinallyEnd if depth == 0 => break,
            _ => {}
        }
        i += 1;
    }
    out
}

/// Search the active try stack (innermost first) for a matching catch.
/// Pops frames as it goes. Returns the outcome; on `Caught`, the owning frame
/// has already been popped from `try_stack`.
pub fn dispatch_exception(execute_data: &mut ExecuteData, thrown_class: &str) -> ExceptionOutcome {
    let first_catch_for = |try_idx: usize| -> u32 {
        execute_data
            .op_array
            .as_ref()
            .and_then(|arr| arr.ops.get(try_idx))
            .map(|op| op.extended_value)
            .unwrap_or(0)
    };

    while let Some(try_idx) = execute_data.try_stack.pop() {
        let first_catch = first_catch_for(try_idx) as usize;
        let catches = execute_data
            .op_array
            .as_ref()
            .map(|arr| collect_catches(&arr.ops, first_catch))
            .unwrap_or_default();
        for (catch_class, var, body_start) in catches {
            if exception_is_a(thrown_class, &catch_class, &execute_data.class_table) {
                return ExceptionOutcome::Caught { body_start, var };
            }
        }
        // This try didn't catch it; keep popping to propagate outward.
    }
    ExceptionOutcome::Uncaught
}

/// Convenience: true if a value looks like an exception object.
pub fn is_throwable_object(val: &Val) -> bool {
    matches!(&val.value, PhpValue::Object(_)) && val.get_type() == PhpType::Object
}
