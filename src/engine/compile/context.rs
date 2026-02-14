//! Compilation Context
//!
//! Manages the compilation state and opcode emission

use super::function_table::FunctionTable;
use crate::engine::types::Val;
use crate::engine::vm::{Op, OpArray, Opcode};

/// Compilation context
pub struct CompileContext {
    pub op_array: OpArray,
    pub current_line: u32,
    pub filename: Option<String>,
    // Track jump targets for control flow
    pub jump_targets: Vec<usize>, // Opcode indices that need jump target updates
    // Function table for storing compiled functions
    pub function_table: FunctionTable,
    // Temp var slot allocator
    next_temp_var: u32,
    // Class table for storing compiled class definitions
    pub class_table: std::collections::HashMap<String, crate::engine::types::ClassEntry>,
    // Current namespace (e.g. "App\\Models")
    pub current_namespace: Option<String>,
    // Use imports: short name → fully qualified name
    pub use_imports: std::collections::HashMap<String, String>,
    // Generator yield array slot (if any)
    pub yield_slot: Option<u32>,
}

impl CompileContext {
    pub fn new() -> Self {
        Self {
            op_array: OpArray::new(String::new()),
            current_line: 0,
            filename: None,
            jump_targets: Vec::new(),
            function_table: FunctionTable::new(),
            next_temp_var: 0,
            class_table: std::collections::HashMap::new(),
            current_namespace: None,
            use_imports: std::collections::HashMap::new(),
            yield_slot: None,
        }
    }

    /// Allocate a new temp var slot and return its index
    pub fn alloc_temp(&mut self) -> u32 {
        let idx = self.next_temp_var;
        self.next_temp_var += 1;
        idx
    }

    /// Ensure generator yield array exists and return its temp ref
    pub fn ensure_yield_array(&mut self) -> Val {
        if let Some(slot) = self.yield_slot {
            return crate::engine::vm::temp_var_ref(slot);
        }
        let slot = self.alloc_temp();
        self.emit_opcode(
            Opcode::InitArray,
            crate::engine::facade::null_val(),
            crate::engine::facade::null_val(),
            crate::engine::vm::temp_var_ref(slot),
        );
        self.yield_slot = Some(slot);
        crate::engine::vm::temp_var_ref(slot)
    }

    /// Resolve a class/trait name using current namespace and imports
    pub fn resolve_class_name(&self, name: &str) -> String {
        if let Some(imported) = self.use_imports.get(name) {
            return imported.clone();
        }
        if let Some(ns) = &self.current_namespace {
            if ns.is_empty() {
                return name.to_string();
            }
            return format!("{}\\{}", ns, name);
        }
        name.to_string()
    }

    /// Create context with filename
    pub fn with_filename(filename: String) -> Self {
        let mut ctx = Self::new();
        ctx.set_filename(&filename);
        ctx
    }

    /// Emit an opcode
    pub fn emit_opcode(&mut self, opcode: Opcode, op1: Val, op2: Val, result: Val) {
        let op = Op {
            opcode,
            op1,
            op2,
            result,
            extended_value: 0,
        };
        self.op_array.ops.push(op);
    }

    /// Emit an opcode and return its index
    pub fn emit_opcode_with_index(
        &mut self,
        opcode: Opcode,
        op1: Val,
        op2: Val,
        result: Val,
    ) -> usize {
        let index = self.op_array.ops.len();
        self.emit_opcode(opcode, op1, op2, result);
        index
    }

    /// Update jump target for an opcode at the given index
    pub fn update_jump_target(&mut self, op_index: usize, target: u32) {
        if op_index < self.op_array.ops.len() {
            self.op_array.ops[op_index].extended_value = target;
        }
    }

    /// Get current opcode index (for jump targets)
    pub fn current_op_index(&self) -> usize {
        self.op_array.ops.len()
    }

    /// Patch op1 of an opcode at the given index
    pub fn patch_op_op1(&mut self, op_index: usize, val: Val) {
        if op_index < self.op_array.ops.len() {
            self.op_array.ops[op_index].op1 = val;
        }
    }

    /// Patch op2 of an opcode at the given index
    pub fn patch_op_op2(&mut self, op_index: usize, val: Val) {
        if op_index < self.op_array.ops.len() {
            self.op_array.ops[op_index].op2 = val;
        }
    }

    /// Set current line number
    pub fn set_line(&mut self, line: u32) {
        self.current_line = line;
    }

    /// Set filename
    pub fn set_filename(&mut self, filename: &str) {
        self.op_array.filename = Some(filename.to_string());
        self.filename = Some(filename.to_string());
    }

    /// Take the op array out of the context (for method compilation)
    pub fn take_op_array(&mut self) -> OpArray {
        std::mem::replace(&mut self.op_array, OpArray::new(String::new()))
    }

    /// Register a class definition
    pub fn register_class(&mut self, ce: crate::engine::types::ClassEntry) {
        self.class_table.insert(ce.name.clone(), ce);
    }

    /// Finalize compilation
    pub fn finalize(mut self) -> OpArray {
        self.op_array.line_start = 0;
        self.op_array.line_end = self.current_line;
        // Transfer class table to op array
        self.op_array.class_table = self.class_table;
        self.op_array
    }
}

impl Default for CompileContext {
    fn default() -> Self {
        Self::new()
    }
}
