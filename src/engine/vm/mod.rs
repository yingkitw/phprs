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

pub mod opcodes;
pub mod execute_data;
mod builtins;
mod format;
mod handlers;
pub mod execute;

// Re-export public API
pub use opcodes::{Op, OpArray, Opcode, get_opcode_name};
pub use execute_data::{ExecuteData, temp_var_ref, var_ref};
pub use execute::execute_ex;
