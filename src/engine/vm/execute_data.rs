//! Execution context and operand resolution helpers

use crate::engine::facade::{StdValFactory, ValFactory};
use crate::engine::types::{PhpType, PhpValue, Val};

/// Sentinel type for temp var / variable references in operands.
/// When an operand's type is `Undef` and value is `Long(n)`, it refers to temp_vars[n].
/// When an operand's type is `Undef` and value is `String(name)`, it refers to symbol_table[name].
pub(crate) const TEMP_VAR_TYPE: PhpType = PhpType::Undef;

/// Create a Val that references a temp var slot
pub fn temp_var_ref(index: u32) -> Val {
    Val::new(PhpValue::Long(index as i64), TEMP_VAR_TYPE)
}

/// Create a Val that references a named variable ($name)
pub fn var_ref(name: &str) -> Val {
    let clean = if name.starts_with('$') {
        &name[1..]
    } else {
        name
    };
    Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(clean, false))),
        TEMP_VAR_TYPE,
    )
}

/// Check if a Val is a temp var reference
pub(crate) fn is_temp_ref(z: &Val) -> bool {
    z.get_type() == TEMP_VAR_TYPE && matches!(z.value, PhpValue::Long(_))
}

/// Check if a Val is a named variable reference
pub(crate) fn is_var_ref(z: &Val) -> bool {
    z.get_type() == TEMP_VAR_TYPE && matches!(z.value, PhpValue::String(_))
}

/// Resolve an operand: if it's a temp var ref, look up in temp_vars;
/// if it's a variable ref, look up in symbol_table; if it's a constant identifier,
/// look up in constants; otherwise return literal.
pub(crate) fn resolve_operand(operand: &Val, execute_data: &ExecuteData) -> Val {
    if is_temp_ref(operand) {
        if let PhpValue::Long(idx) = &operand.value {
            return execute_data.get_temp(*idx as usize);
        }
    }
    if is_var_ref(operand) {
        if let PhpValue::String(name) = &operand.value {
            let n = name.as_str();
            let clean = if n.starts_with('$') { &n[1..] } else { n };
            return execute_data.get_var(clean);
        }
    }
    // Check if it's a constant identifier (bare string that might be a constant name)
    if operand.get_type() == PhpType::String {
        if let PhpValue::String(name) = &operand.value {
            let name_str = name.as_str();
            // Check if this is a constant (uppercase identifier without quotes in source)
            if execute_data.constants.contains_key(name_str) {
                return clone_val(execute_data.constants.get(name_str).unwrap());
            }
        }
    }
    clone_val(operand)
}

/// Get the result temp var index from an op's result field
pub(crate) fn result_slot(op: &super::opcodes::Op) -> Option<usize> {
    if is_temp_ref(&op.result) {
        if let PhpValue::Long(idx) = &op.result.value {
            return Some(*idx as usize);
        }
    }
    None
}

/// Helper function to clone a Val
pub(crate) fn clone_val(source: &Val) -> Val {
    StdValFactory::clone_val(source)
}

/// Result of opcode execution
pub enum ExecResult {
    Continue,
    Jump(u32),
    Return(Val),
}

/// Execution context for PHP scripts
#[derive(Debug)]
pub struct ExecuteData {
    pub op_array: Option<super::opcodes::OpArray>,
    pub current_op: usize,
    pub symbol_table: Option<crate::engine::types::PhpArray>,
    pub function_table: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
    pub temp_vars: Vec<Val>,
    pub call_args: Vec<Val>,
    pub included_files: std::collections::HashSet<String>,
    pub class_table: std::collections::HashMap<String, crate::engine::types::ClassEntry>,
    /// Constants defined by define() (shared across includes)
    pub constants: std::collections::HashMap<String, Val>,
    /// Directory of the script currently being executed (for resolving relative include paths)
    pub current_script_dir: Option<String>,
    /// If set by exit()/die(), script should terminate with this code
    pub exit_requested: Option<i64>,
    /// Error handler function name (set by set_error_handler())
    pub error_handler: Option<String>,
    /// Exception handler function name (set by set_exception_handler())
    pub exception_handler: Option<String>,
    /// Shutdown function names (registered by register_shutdown_function())
    pub shutdown_functions: Vec<String>,
}

impl ExecuteData {
    pub fn new() -> Self {
        Self {
            op_array: None,
            current_op: 0,
            symbol_table: Some(crate::engine::types::PhpArray::new()),
            function_table: None,
            temp_vars: Vec::new(),
            call_args: Vec::new(),
            included_files: std::collections::HashSet::new(),
            class_table: std::collections::HashMap::new(),
            constants: std::collections::HashMap::new(),
            current_script_dir: None,
            exit_requested: None,
            error_handler: None,
            exception_handler: None,
            shutdown_functions: Vec::new(),
        }
    }

    /// Ensure temp_vars has at least `n` slots
    pub fn ensure_temp_slots(&mut self, n: usize) {
        if self.temp_vars.len() < n {
            self.temp_vars
                .resize_with(n, || Val::new(PhpValue::Long(0), PhpType::Null));
        }
    }

    /// Get a temp var value (clone)
    pub fn get_temp(&self, index: usize) -> Val {
        self.temp_vars
            .get(index)
            .map(|z| clone_val(z))
            .unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null))
    }

    /// Set a temp var value
    pub fn set_temp(&mut self, index: usize, val: Val) {
        if index >= self.temp_vars.len() {
            self.ensure_temp_slots(index + 1);
        }
        self.temp_vars[index] = val;
    }

    /// Look up a variable by name in the symbol table
    pub fn get_var(&self, name: &str) -> Val {
        if let Some(ref st) = self.symbol_table {
            let key = crate::engine::string::string_init(name, false);
            if let Some(val) = crate::engine::hash::hash_find(st, &key) {
                return clone_val(val);
            }
        }
        Val::new(PhpValue::Long(0), PhpType::Null)
    }

    /// Set a variable in the symbol table
    pub fn set_var(&mut self, name: &str, val: Val) {
        if let Some(ref mut st) = self.symbol_table {
            let key = crate::engine::string::string_init(name, false);
            let key_box = Box::new(key);
            let _ = crate::engine::hash::hash_add_or_update(st, Some(&*key_box), 0, val, 0);
        }
    }

    /// Remove a variable from the symbol table
    pub fn remove_var(&mut self, name: &str) {
        if let Some(ref mut st) = self.symbol_table {
            let key = crate::engine::string::string_init(name, false);
            let _ = crate::engine::hash::hash_del(st, &key);
        }
    }
}
