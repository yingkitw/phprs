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
    // Bare identifier constants are emitted as ConstantAst (not real string literals)
    if operand.get_type() == PhpType::ConstantAst {
        if let PhpValue::String(name) = &operand.value {
            let name_str = name.as_str();
            if let Some(v) = execute_data.constants.get(name_str) {
                return clone_val(v);
            }
            // PHP legacy: undefined constant name becomes a string of that name
            return Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(name_str, false))),
                PhpType::String,
            );
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
    pub call_arg_names: Vec<Option<String>>,
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
    /// Late static binding: the class that was actually called (for static:: resolution)
    pub called_class: Option<String>,
    /// Stack for nested function call argument tracking (InitFCall pushes, DoFCall pops)
    pub call_arg_stack: Vec<(usize, usize)>,
    /// Parallel to call_args: true when argument was passed by variable reference
    pub call_arg_by_ref: Vec<bool>,
    /// Caller scope for pass-by-reference parameters during UDF execution
    pub ref_caller_scope: Option<crate::engine::types::PhpArray>,
    /// Maps callee ref-parameter name → caller variable name
    pub ref_param_bindings: std::collections::HashMap<String, String>,
    /// SPL autoload function names (registered by spl_autoload_register())
    pub autoload_functions: Vec<String>,
    /// Temp slot for foreach key when iterating `as $key => $value` (set by FeReset).
    pub fe_key_slot: Option<u32>,
    /// Script-level globals snapshot (shared across function calls).
    pub global_script_table: Option<crate::engine::types::PhpArray>,
    /// Names imported via `global $name` in the current function scope.
    pub global_imports: std::collections::HashSet<String>,
}

impl ExecuteData {
    pub fn new() -> Self {
        let mut ed = Self {
            op_array: None,
            current_op: 0,
            symbol_table: Some(crate::engine::types::PhpArray::new()),
            function_table: None,
            temp_vars: Vec::new(),
            call_args: Vec::new(),
            call_arg_names: Vec::new(),
            included_files: std::collections::HashSet::new(),
            class_table: std::collections::HashMap::new(),
            constants: std::collections::HashMap::new(),
            current_script_dir: None,
            exit_requested: None,
            error_handler: None,
            exception_handler: None,
            shutdown_functions: Vec::new(),
            called_class: None,
            call_arg_stack: Vec::new(),
            call_arg_by_ref: Vec::new(),
            ref_caller_scope: None,
            ref_param_bindings: std::collections::HashMap::new(),
            autoload_functions: Vec::new(),
            fe_key_slot: None,
            global_script_table: None,
            global_imports: std::collections::HashSet::new(),
        };
        ed.register_reflection_classes();
        ed
    }

    /// Register built-in reflection classes
    pub fn register_reflection_classes(&mut self) {
        crate::engine::vm::reflection::register_reflection_classes(self);
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
        if let Some(caller_var) = self.ref_param_bindings.get(name) {
            if let Some(ref scope) = self.ref_caller_scope {
                let key = crate::engine::string::string_init(caller_var, false);
                if let Some(val) = crate::engine::hash::hash_find(scope, &key) {
                    return clone_val(val);
                }
            }
            return Val::new(PhpValue::Long(0), PhpType::Null);
        }
        if self.global_imports.contains(name) {
            if let Some(ref global) = self.global_script_table {
                let key = crate::engine::string::string_init(name, false);
                if let Some(val) = crate::engine::hash::hash_find(global, &key) {
                    return clone_val(val);
                }
            }
            return Val::new(PhpValue::Long(0), PhpType::Null);
        }
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
        if let Some(caller_var) = self.ref_param_bindings.get(name).cloned() {
            if let Some(ref mut scope) = self.ref_caller_scope {
                let key = crate::engine::string::string_init(&caller_var, false);
                let key_box = Box::new(key);
                let _ = crate::engine::hash::hash_add_or_update(
                    scope,
                    Some(&*key_box),
                    0,
                    val,
                    0,
                );
            }
            return;
        }
        if self.global_imports.contains(name) {
            self.ensure_global_script_table();
            if let Some(ref mut global) = self.global_script_table {
                let key = crate::engine::string::string_init(name, false);
                let key_box = Box::new(key);
                let _ = crate::engine::hash::hash_add_or_update(
                    global,
                    Some(&*key_box),
                    0,
                    val,
                    0,
                );
            }
            return;
        }
        if let Some(ref mut st) = self.symbol_table {
            let key = crate::engine::string::string_init(name, false);
            let key_box = Box::new(key);
            let _ = crate::engine::hash::hash_add_or_update(st, Some(&*key_box), 0, val, 0);
        }
    }

    /// Ensure the script-global table exists (lazy init from current symbol table).
    fn ensure_global_script_table(&mut self) {
        if self.global_script_table.is_none() {
            if let Some(ref st) = self.symbol_table {
                self.global_script_table = Some(Self::clone_php_array(st));
            } else {
                self.global_script_table = Some(crate::engine::types::PhpArray::new());
            }
        }
    }

    /// Import a script-global variable into the current function scope.
    pub fn bind_global(&mut self, name: &str) {
        self.ensure_global_script_table();
        self.global_imports.insert(name.to_string());
    }

    /// Merge imported globals back into the caller symbol table on function return.
    pub fn merge_globals_into(&self, outer: &mut crate::engine::types::PhpArray) {
        if let Some(ref global) = self.global_script_table {
            for name in &self.global_imports {
                let key = crate::engine::string::string_init(name, false);
                if let Some(val) = crate::engine::hash::hash_find(global, &key) {
                    let key_box = Box::new(key);
                    let _ = crate::engine::hash::hash_add_or_update(
                        outer,
                        Some(&*key_box),
                        0,
                        clone_val(val),
                        0,
                    );
                }
            }
        }
    }

    /// Clone a PHP array (shallow copy of elements).
    pub fn clone_php_array(arr: &crate::engine::types::PhpArray) -> crate::engine::types::PhpArray {
        let mut copy = crate::engine::types::PhpArray::new();
        for bucket in &arr.ar_data {
            let val = clone_val(&bucket.val);
            if let Some(ref key) = bucket.key {
                let _ = crate::engine::hash::hash_add_or_update(
                    &mut copy,
                    Some(key.as_ref()),
                    bucket.h,
                    val,
                    0,
                );
            } else {
                let _ = crate::engine::hash::hash_add_or_update(&mut copy, None, bucket.h, val, 0);
            }
        }
        copy
    }

    /// Remove a variable from the symbol table
    pub fn remove_var(&mut self, name: &str) {
        if let Some(ref mut st) = self.symbol_table {
            let key = crate::engine::string::string_init(name, false);
            let _ = crate::engine::hash::hash_del(st, &key);
        }
    }
}
