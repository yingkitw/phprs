//! PHP INI configuration
//!
//! Migrated from main/php_ini.h and main/php_ini.c

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::fs;

/// INI entry with name, value, and optional section
#[derive(Debug, Clone)]
pub struct IniEntry {
    pub name: String,
    pub value: String,
    pub section: Option<String>,
}

/// INI configuration parser
pub struct IniParser {
    entries: HashMap<String, IniEntry>,
    current_section: Option<String>,
}

impl IniParser {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            current_section: None,
        }
    }

    /// Parse INI file
    pub fn parse_file(&mut self, filename: &str) -> Result<(), String> {
        let content =
            fs::read_to_string(filename).map_err(|e| format!("Failed to read INI file: {e}"))?;
        self.parse_string(&content)
    }

    /// Parse INI string
    pub fn parse_string(&mut self, content: &str) -> Result<(), String> {
        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }

            // Parse section [section_name]
            if line.starts_with('[') && line.ends_with(']') {
                let section = &line[1..line.len() - 1];
                self.current_section = Some(section.to_string());
                continue;
            }

            // Parse key=value
            if let Some(equal_pos) = line.find('=') {
                let key = line[..equal_pos].trim().to_string();
                let value = line[equal_pos + 1..].trim().to_string();

                // Remove quotes if present
                let value = value.trim_matches(|c| c == '"' || c == '\'');

                let entry = IniEntry {
                    name: key.clone(),
                    value: value.to_string(),
                    section: self.current_section.clone(),
                };

                // Use full key (section.key) as map key
                let map_key = if let Some(ref section) = self.current_section {
                    format!("{section}.{key}")
                } else {
                    key
                };

                self.entries.insert(map_key, entry);
            } else {
                return Err(format!("Invalid INI syntax at line {line_num}: {line}"));
            }
        }

        Ok(())
    }

    /// Get INI entry by name
    pub fn get(&self, name: &str) -> Option<&IniEntry> {
        self.entries.get(name)
    }

    /// Get all entries
    pub fn get_all(&self) -> Vec<&IniEntry> {
        self.entries.values().collect()
    }

    /// Get entries for a specific section
    pub fn get_section(&self, section: &str) -> Vec<&IniEntry> {
        self.entries
            .values()
            .filter(|e| e.section.as_ref().map(|s| s == section).unwrap_or(false))
            .collect()
    }
}

impl Default for IniParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse INI file and return entries
pub fn php_ini_parse_file(filename: &str) -> Result<Vec<IniEntry>, String> {
    let mut parser = IniParser::new();
    parser.parse_file(filename)?;
    Ok(parser.get_all().into_iter().cloned().collect())
}

/// Get INI value by name
pub fn php_ini_get(_name: &str) -> Option<String> {
    // TODO: Implement global INI storage
    None
}

/// Set INI value
pub fn php_ini_set(_name: &str, _value: &str) -> Result<(), String> {
    // TODO: Implement global INI storage
    Ok(())
}
