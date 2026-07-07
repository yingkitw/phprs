//! Function Table
//!
//! Manages storage and lookup of compiled functions

use crate::engine::vm::OpArray;
use std::collections::HashMap;

/// Function table for storing compiled functions
#[derive(Debug)]
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

    /// Merge functions from another table (existing names are kept).
    pub fn merge(&mut self, mut other: FunctionTable) {
        for (name, op_array) in other.functions.drain() {
            self.functions.entry(name).or_insert(op_array);
        }
    }
}

/// Merge compiled functions from an include/require into the running script's table.
pub fn merge_into_execute_data(
    execute_data: &mut crate::engine::vm::ExecuteData,
    incoming: FunctionTable,
) {
    use std::sync::Arc;

    let merged = match execute_data.function_table.take() {
        None => incoming,
        Some(arc) => {
            let owned = Arc::downcast::<FunctionTable>(arc)
                .expect("function_table must hold FunctionTable");
            let mut base = Arc::try_unwrap(owned)
                .expect("function_table Arc must be uniquely owned during merge");
            base.merge(incoming);
            base
        }
    };
    execute_data.function_table = Some(Arc::new(merged));
}

impl Default for FunctionTable {
    fn default() -> Self {
        Self::new()
    }
}
