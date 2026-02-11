//! PHP Extensions
//!
//! Extension framework and standard extensions
//! Migrated from PHP modules

/// Module Entry
///
/// Represents a PHP extension module
pub struct ModuleEntry {
    pub name: String,
    pub version: String,
    pub functions: Vec<String>,
}

impl ModuleEntry {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            functions: Vec::new(),
        }
    }

    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    pub fn add_function(&mut self, function_name: &str) {
        self.functions.push(function_name.to_string());
    }
}

impl Default for ModuleEntry {
    fn default() -> Self {
        Self::new("unknown")
    }
}
