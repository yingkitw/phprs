//! PHP session extension (minimal)
//!
//! Provides `session_start`, `session_destroy`, `session_id`, and `session_name`
//! with optional file-backed persistence between requests.

#[cfg(test)]
mod tests;

use crate::engine::hash::{hash_add_or_update, hash_init};
use crate::engine::operators::zval_get_string;
use crate::engine::string::string_init;
use crate::engine::types::{PhpArray, PhpType, PhpValue, Val};
use crate::engine::vm::execute_data::ExecuteData;
use std::path::PathBuf;
use std::sync::OnceLock;

static SAVE_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Configure directory for session files (used by `phprs serve`).
pub fn configure_save_path(path: PathBuf) {
    let _ = SAVE_PATH.set(path);
}

fn save_path() -> PathBuf {
    SAVE_PATH
        .get()
        .cloned()
        .unwrap_or_else(|| std::env::temp_dir().join("phprs-sessions"))
}

fn generate_session_id() -> String {
    let mut bytes = [0u8; 16];
    let _ = getrandom::getrandom(&mut bytes);
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn session_file_path(id: &str) -> PathBuf {
    save_path().join(format!("sess_{id}"))
}

fn json_value_to_val(value: &serde_json::Value) -> Val {
    match value {
        serde_json::Value::Null => Val::new(PhpValue::Long(0), PhpType::Null),
        serde_json::Value::Bool(b) => Val::new(
            PhpValue::Long(if *b { 1 } else { 0 }),
            if *b { PhpType::True } else { PhpType::False },
        ),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Val::new(PhpValue::Long(i), PhpType::Long)
            } else if let Some(f) = n.as_f64() {
                Val::new(PhpValue::Double(f), PhpType::Double)
            } else {
                Val::new(PhpValue::Long(0), PhpType::Null)
            }
        }
        serde_json::Value::String(s) => Val::new(
            PhpValue::String(Box::new(string_init(s, false))),
            PhpType::String,
        ),
        serde_json::Value::Array(items) => {
            let mut arr = PhpArray::new();
            hash_init(&mut arr, 8);
            for (i, item) in items.iter().enumerate() {
                let _ = hash_add_or_update(&mut arr, None, i as u64, json_value_to_val(item), 0);
            }
            Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array)
        }
        serde_json::Value::Object(map) => {
            let mut arr = PhpArray::new();
            hash_init(&mut arr, 8);
            for (k, v) in map {
                let key = string_init(k, false);
                let _ = hash_add_or_update(&mut arr, Some(&key), 0, json_value_to_val(v), 0);
            }
            Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array)
        }
    }
}

fn load_session_data(id: &str) -> PhpArray {
    let path = session_file_path(id);
    let Ok(content) = std::fs::read_to_string(&path) else {
        return PhpArray::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        return PhpArray::new();
    };
    match json {
        serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
            if let PhpValue::Array(arr) = json_value_to_val(&json).value {
                *arr
            } else {
                PhpArray::new()
            }
        }
        _ => PhpArray::new(),
    }
}

fn val_to_json_value(val: &Val) -> serde_json::Value {
    match val.get_type() {
        PhpType::Null => serde_json::Value::Null,
        PhpType::True => serde_json::Value::Bool(true),
        PhpType::False => serde_json::Value::Bool(false),
        PhpType::Long => serde_json::Value::Number(
            serde_json::Number::from(crate::engine::operators::zval_get_long(val)),
        ),
        PhpType::Double => serde_json::Number::from_f64(crate::engine::operators::zval_get_double(val))
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        PhpType::String => {
            serde_json::Value::String(crate::engine::operators::zval_get_string(val).as_str().to_string())
        }
        PhpType::Array => {
            if let PhpValue::Array(ref arr) = val.value {
                let is_list = arr.ar_data.iter().all(|b| b.key.is_none());
                if is_list {
                    serde_json::Value::Array(
                        arr.ar_data
                            .iter()
                            .map(|b| val_to_json_value(&b.val))
                            .collect(),
                    )
                } else {
                    let mut map = serde_json::Map::new();
                    for bucket in &arr.ar_data {
                        let key = bucket
                            .key
                            .as_ref()
                            .map(|k| k.as_str().to_string())
                            .unwrap_or_else(|| bucket.h.to_string());
                        map.insert(key, val_to_json_value(&bucket.val));
                    }
                    serde_json::Value::Object(map)
                }
            } else {
                serde_json::Value::Array(vec![])
            }
        }
        _ => serde_json::Value::Null,
    }
}

fn save_session_data(id: &str, arr: &PhpArray) -> Result<(), String> {
    let dir = save_path();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let val = Val::new(PhpValue::Array(Box::new(ExecuteData::clone_php_array(arr))), PhpType::Array);
    let json = serde_json::to_string(&val_to_json_value(&val)).map_err(|e| e.to_string())?;
    std::fs::write(session_file_path(id), json).map_err(|e| e.to_string())
}

fn bind_session_array(execute_data: &mut ExecuteData, data: PhpArray) {
    execute_data.set_var(
        "_SESSION",
        Val::new(PhpValue::Array(Box::new(data)), PhpType::Array),
    );
}

/// Apply incoming session id from an HTTP cookie before `session_start()`.
pub fn apply_incoming_session_id(execute_data: &mut ExecuteData, id: &str) {
    if !id.is_empty() {
        execute_data.session_id = id.to_string();
    }
}

/// Build a `Set-Cookie` header value when a session is active.
pub fn cookie_header_value(execute_data: &ExecuteData) -> Option<String> {
    if execute_data.session_active && !execute_data.session_id.is_empty() {
        Some(format!(
            "{}={}; Path=/; HttpOnly; SameSite=Lax",
            execute_data.session_name, execute_data.session_id
        ))
    } else {
        None
    }
}

pub fn session_start(execute_data: &mut ExecuteData) -> Result<bool, String> {
    if execute_data.session_active {
        return Ok(true);
    }
    if execute_data.session_id.is_empty() {
        execute_data.session_id = generate_session_id();
    }
    if execute_data.session_name.is_empty() {
        execute_data.session_name = "PHPSESSID".to_string();
    }
    let data = load_session_data(&execute_data.session_id);
    bind_session_array(execute_data, data);
    execute_data.session_active = true;
    Ok(true)
}

pub fn session_destroy(execute_data: &mut ExecuteData) -> Result<bool, String> {
    if !execute_data.session_id.is_empty() {
        let _ = std::fs::remove_file(session_file_path(&execute_data.session_id));
    }
    execute_data.session_active = false;
    execute_data.session_id.clear();
    bind_session_array(execute_data, PhpArray::new());
    Ok(true)
}

pub fn session_id(args: &[Val], execute_data: &mut ExecuteData) -> Result<String, String> {
    if !args.is_empty() {
        let new_id = zval_get_string(&args[0]).as_str().to_string();
        if execute_data.session_active && !execute_data.session_id.is_empty() {
            let _ = std::fs::remove_file(session_file_path(&execute_data.session_id));
        }
        execute_data.session_id = new_id;
    }
    Ok(execute_data.session_id.clone())
}

pub fn session_name(args: &[Val], execute_data: &mut ExecuteData) -> Result<String, String> {
    if !args.is_empty() {
        execute_data.session_name = zval_get_string(&args[0]).as_str().to_string();
    }
    if execute_data.session_name.is_empty() {
        execute_data.session_name = "PHPSESSID".to_string();
    }
    Ok(execute_data.session_name.clone())
}

/// Persist session data at end of request (called from VM).
pub fn session_write_close(execute_data: &ExecuteData) -> Result<(), String> {
    if !execute_data.session_active || execute_data.session_id.is_empty() {
        return Ok(());
    }
    let session_val = execute_data.get_var("_SESSION");
    if let PhpValue::Array(ref arr) = session_val.value {
        save_session_data(&execute_data.session_id, arr)?;
    }
    Ok(())
}
