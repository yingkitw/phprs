//! Virtual Machine
//!
//! Virtual machine and execution
//!
//! This module implements the PHP virtual machine opcodes and execution engine.
//!
//! ## Architecture
//!
//! Operands (`op1`, `op2`) can be:
//! - **Literal values** — the Val is used directly
//! - **Temp var references** — `PhpType::Undef` with `Long(index)` → look up in `temp_vars`
//! - **Variable references** — `PhpType::Undef` with `String(name)` → look up in `symbol_table`
//!
//! Results are stored in `temp_vars[result_index]` where `result_index` comes from
//! `op.result` (a `Long` value when type is `Undef`).

#[cfg(test)]
mod tests;

mod builtins;
pub mod dispatch_handlers;
pub mod execute;
pub mod execute_data;
mod format;
mod handlers;
pub mod opcodes;

// Re-export public API
pub use execute::execute_ex;
pub use execute_data::{temp_var_ref, var_ref, ExecuteData};
pub use opcodes::{get_opcode_name, Op, OpArray, Opcode};
