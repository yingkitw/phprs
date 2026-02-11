//! Function Table
//!
//! Manages storage and lookup of compiled functions

use crate::engine::vm::OpArray;
use std::collections::HashMap;

/// Function table for storing compiled functions
pub struct FunctionTable {
    functions: HashMap<String, OpArray>,
}

impl FunctionTable {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    /// Store a function in the table
    /// Function names are stored in lowercase for case-insensitive lookup
    pub fn store_function(&mut self, name: &str, op_array: OpArray) {
        let lower_name = name.to_lowercase();
        self.functions.insert(lower_name, op_array);
    }

    /// Look up a function by name (case-insensitive)
    pub fn lookup_function(&self, name: &str) -> Option<&OpArray> {
        let lower_name = name.to_lowercase();
        self.functions.get(&lower_name)
    }

    /// Check if a function exists
    pub fn has_function(&self, name: &str) -> bool {
        let lower_name = name.to_lowercase();
        self.functions.contains_key(&lower_name)
    }

    /// Get all function names
    pub fn get_function_names(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }
}

impl Default for FunctionTable {
    fn default() -> Self {
        Self::new()
    }
}
