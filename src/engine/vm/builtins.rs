//! Built-in PHP function implementations

use super::execute_data::{clone_val, ExecuteData};
use super::format::{print_r_value, var_dump_value, zval_to_json};
use crate::engine::types::{PhpType, PhpValue, Val};

/// Helper to create a string Val
fn string_val(s: &str) -> Val {
    Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(s, false))),
        PhpType::String,
    )
}

/// Helper to create a bool Val
fn bool_val(b: bool) -> Val {
    Val::new(
        PhpValue::Long(if b { 1 } else { 0 }),
        if b { PhpType::True } else { PhpType::False },
    )
}

/// Helper to get string from first arg (with arity check)
fn require_string_arg(args: &[Val], func: &str) -> Result<String, String> {
    if args.is_empty() {
        return Err(format!("{func}() expects 1 argument"));
    }
    Ok(crate::engine::operators::zval_get_string(&args[0])
        .as_str()
        .to_string())
}

/// Helper to check a type predicate on first arg
fn type_check(args: &[Val], predicate: impl FnOnce(PhpType) -> bool) -> Val {
    bool_val(!args.is_empty() && predicate(args[0].get_type()))
}

/// Resolve path for file operations: relative paths are resolved against current script directory
fn resolve_path_for_runtime(path: &str, execute_data: &ExecuteData) -> String {
    if path.starts_with('/') || (path.len() >= 2 && path.get(1..2) == Some(":")) {
        path.to_string()
    } else if let Some(ref dir) = execute_data.current_script_dir {
        let mut p = std::path::PathBuf::from(dir);
        p.push(path);
        p.to_string_lossy().into_owned()
    } else {
        path.to_string()
    }
}

/// Execute a built-in function call
pub(crate) fn execute_builtin_function(
    name: &str,
    args: &[Val],
    _execute_data: &mut ExecuteData,
) -> Result<Option<Val>, String> {
    match name {
        // --- String functions ---
        "strlen" => {
            let s = require_string_arg(args, "strlen")?;
            Ok(Some(Val::new(
                PhpValue::Long(s.len() as i64),
                PhpType::Long,
            )))
        }
        "strpos" => {
            if args.len() < 2 {
                return Err("strpos() expects at least 2 arguments".into());
            }
            let haystack = crate::engine::operators::zval_get_string(&args[0]);
            let needle = crate::engine::operators::zval_get_string(&args[1]);
            match haystack.as_str().find(needle.as_str()) {
                Some(pos) => Ok(Some(Val::new(PhpValue::Long(pos as i64), PhpType::Long))),
                None => Ok(Some(Val::new(PhpValue::Long(0), PhpType::False))),
            }
        }
        "substr" => {
            if args.len() < 2 {
                return Err("substr() expects at least 2 arguments".into());
            }
            let s = crate::engine::operators::zval_get_string(&args[0]);
            let start = crate::engine::operators::zval_get_long(&args[1]) as usize;
            let src = s.as_str();
            let len = if args.len() > 2 {
                crate::engine::operators::zval_get_long(&args[2]) as usize
            } else {
                src.len().saturating_sub(start)
            };
            let end = (start + len).min(src.len());
            let result = if start < src.len() {
                &src[start..end]
            } else {
                ""
            };
            Ok(Some(string_val(result)))
        }
        "str_replace" => {
            if args.len() < 3 {
                return Err("str_replace() expects at least 3 arguments".into());
            }
            let search = crate::engine::operators::zval_get_string(&args[0]);
            let replace = crate::engine::operators::zval_get_string(&args[1]);
            let subject = crate::engine::operators::zval_get_string(&args[2]);
            let result = subject.as_str().replace(search.as_str(), replace.as_str());
            Ok(Some(string_val(&result)))
        }
        "strtolower" => {
            let s = require_string_arg(args, "strtolower")?;
            Ok(Some(string_val(&s.to_lowercase())))
        }
        "strtoupper" => {
            let s = require_string_arg(args, "strtoupper")?;
            Ok(Some(string_val(&s.to_uppercase())))
        }
        "trim" => {
            let s = require_string_arg(args, "trim")?;
            Ok(Some(string_val(s.trim())))
        }
        "explode" => {
            if args.len() < 2 {
                return Err("explode() expects at least 2 arguments".into());
            }
            let delim = crate::engine::operators::zval_get_string(&args[0]);
            let s = crate::engine::operators::zval_get_string(&args[1]);
            let parts: Vec<&str> = s.as_str().split(delim.as_str()).collect();
            let mut arr = crate::engine::types::PhpArray::new();
            for (i, part) in parts.iter().enumerate() {
                let val = string_val(part);
                let _ = crate::engine::hash::hash_add_or_update(&mut arr, None, i as u64, val, 0);
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(arr)),
                PhpType::Array,
            )))
        }
        "implode" | "join" => {
            if args.len() < 2 {
                return Err("implode() expects 2 arguments".into());
            }
            let glue = crate::engine::operators::zval_get_string(&args[0]);
            if let PhpValue::Array(ref arr) = args[1].value {
                let parts: Vec<String> = arr
                    .ar_data
                    .iter()
                    .map(|b| {
                        crate::engine::operators::zval_get_string(&b.val)
                            .as_str()
                            .to_string()
                    })
                    .collect();
                let result = parts.join(glue.as_str());
                Ok(Some(string_val(&result)))
            } else {
                Ok(Some(string_val("")))
            }
        }
        "sprintf" => {
            if args.is_empty() {
                return Err("sprintf() expects at least 1 argument".into());
            }
            let fmt = crate::engine::operators::zval_get_string(&args[0]);
            let mut result = fmt.as_str().to_string();
            for arg in &args[1..] {
                let s = crate::engine::operators::zval_get_string(arg);
                if let Some(pos) = result.find("%s") {
                    result.replace_range(pos..pos + 2, s.as_str());
                } else if let Some(pos) = result.find("%d") {
                    let v = crate::engine::operators::zval_get_long(arg);
                    result.replace_range(pos..pos + 2, &v.to_string());
                }
            }
            Ok(Some(string_val(&result)))
        }

        // --- Type conversion ---
        "intval" => {
            if args.is_empty() {
                return Err("intval() expects 1 argument".into());
            }
            Ok(Some(Val::new(
                PhpValue::Long(crate::engine::operators::zval_get_long(&args[0])),
                PhpType::Long,
            )))
        }
        "floatval" | "doubleval" => {
            if args.is_empty() {
                return Err("floatval() expects 1 argument".into());
            }
            Ok(Some(Val::new(
                PhpValue::Double(crate::engine::operators::zval_get_double(&args[0])),
                PhpType::Double,
            )))
        }
        "strval" => {
            let s = require_string_arg(args, "strval")?;
            Ok(Some(string_val(&s)))
        }

        // --- Type checking ---
        "isset" => {
            if args.is_empty() {
                return Ok(Some(bool_val(false)));
            }
            Ok(Some(bool_val(
                args[0].get_type() != PhpType::Null && args[0].get_type() != PhpType::Undef,
            )))
        }
        "empty" => {
            if args.is_empty() {
                return Ok(Some(bool_val(true)));
            }
            let is_empty = match &args[0].value {
                PhpValue::Long(0) => true,
                PhpValue::Double(v) if *v == 0.0 => true,
                PhpValue::String(s) if s.as_str().is_empty() || s.as_str() == "0" => true,
                _ => args[0].get_type() == PhpType::Null || args[0].get_type() == PhpType::False,
            };
            Ok(Some(bool_val(is_empty)))
        }
        "is_int" | "is_integer" | "is_long" => Ok(Some(type_check(args, |t| t == PhpType::Long))),
        "is_string" => Ok(Some(type_check(args, |t| t == PhpType::String))),
        "is_float" | "is_double" => Ok(Some(type_check(args, |t| t == PhpType::Double))),
        "is_bool" => Ok(Some(type_check(args, |t| {
            matches!(t, PhpType::True | PhpType::False)
        }))),
        "is_null" => Ok(Some(type_check(args, |t| t == PhpType::Null))),
        "is_array" => Ok(Some(type_check(args, |t| t == PhpType::Array))),

        // --- Array functions ---
        "array_key_exists" => {
            if args.len() < 2 {
                return Err("array_key_exists() expects 2 arguments".into());
            }
            let key = crate::engine::operators::zval_get_string(&args[0]);
            if let PhpValue::Array(ref arr) = args[1].value {
                let found = arr.ar_data.iter().any(|b| {
                    b.key
                        .as_ref()
                        .map(|k| k.as_str() == key.as_str())
                        .unwrap_or(false)
                });
                Ok(Some(bool_val(found)))
            } else {
                Ok(Some(bool_val(false)))
            }
        }
        "in_array" => {
            if args.len() < 2 {
                return Err("in_array() expects at least 2 arguments".into());
            }
            let needle = crate::engine::operators::zval_get_string(&args[0]);
            if let PhpValue::Array(ref arr) = args[1].value {
                let found = arr.ar_data.iter().any(|b| {
                    crate::engine::operators::zval_get_string(&b.val).as_str() == needle.as_str()
                });
                Ok(Some(bool_val(found)))
            } else {
                Ok(Some(bool_val(false)))
            }
        }
        "count" | "sizeof" => {
            if args.is_empty() {
                return Err("count() expects 1 argument".into());
            }
            if let PhpValue::Array(ref arr) = args[0].value {
                Ok(Some(Val::new(
                    PhpValue::Long(arr.ar_data.len() as i64),
                    PhpType::Long,
                )))
            } else {
                Ok(Some(Val::new(PhpValue::Long(1), PhpType::Long)))
            }
        }
        "array_push" => {
            if args.len() < 2 {
                return Err("array_push() expects at least 2 arguments".into());
            }
            Ok(Some(Val::new(PhpValue::Long(0), PhpType::Long)))
        }
        "array_merge" => {
            let mut merged = crate::engine::types::PhpArray::new();
            let mut idx: u64 = 0;
            for arg in args {
                if let PhpValue::Array(ref arr) = arg.value {
                    for bucket in &arr.ar_data {
                        let val = clone_val(&bucket.val);
                        if let Some(ref k) = bucket.key {
                            let key = crate::engine::string::string_init(k.as_str(), false);
                            let _ = crate::engine::hash::hash_add_or_update(
                                &mut merged,
                                Some(&key),
                                0,
                                val,
                                0,
                            );
                        } else {
                            let _ = crate::engine::hash::hash_add_or_update(
                                &mut merged,
                                None,
                                idx,
                                val,
                                0,
                            );
                            idx += 1;
                        }
                    }
                }
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(merged)),
                PhpType::Array,
            )))
        }

        // --- Output / debug ---
        "var_dump" => {
            for arg in args {
                let dump = var_dump_value(arg);
                let _ = crate::php::output::php_output_write(dump.as_bytes());
            }
            Ok(None)
        }
        "print_r" => {
            if !args.is_empty() {
                let output = print_r_value(&args[0]);
                let return_str =
                    args.len() > 1 && crate::engine::operators::zval_get_bool(&args[1]);
                if return_str {
                    Ok(Some(string_val(&output)))
                } else {
                    let _ = crate::php::output::php_output_write(output.as_bytes());
                    Ok(Some(Val::new(PhpValue::Long(1), PhpType::True)))
                }
            } else {
                Ok(None)
            }
        }
        "echo" | "print" => {
            for arg in args {
                let s = crate::engine::operators::zval_get_string(arg);
                let _ = crate::php::output::php_output_write(s.as_bytes());
            }
            Ok(Some(Val::new(PhpValue::Long(1), PhpType::Long)))
        }

        // --- JSON ---
        "json_encode" => {
            if args.is_empty() {
                return Err("json_encode() expects 1 argument".into());
            }
            Ok(Some(string_val(&zval_to_json(&args[0]))))
        }
        "json_decode" => {
            if args.is_empty() {
                return Err("json_decode() expects 1 argument".into());
            }
            let s = crate::engine::operators::zval_get_string(&args[0]);
            Ok(Some(string_val(s.as_str())))
        }

        // --- Filesystem ---
        "file_get_contents" => {
            if args.is_empty() {
                return Err("file_get_contents() expects 1 argument".into());
            }
            let path = crate::engine::operators::zval_get_string(&args[0]);
            let resolved = resolve_path_for_runtime(path.as_str(), _execute_data);
            match std::fs::read_to_string(&resolved) {
                Ok(content) => Ok(Some(string_val(&content))),
                Err(_) => Ok(Some(Val::new(PhpValue::Long(0), PhpType::False))),
            }
        }
        "file_exists" => {
            if args.is_empty() {
                return Err("file_exists() expects 1 argument".into());
            }
            let path = crate::engine::operators::zval_get_string(&args[0]);
            let resolved = resolve_path_for_runtime(path.as_str(), _execute_data);
            Ok(Some(bool_val(std::path::Path::new(&resolved).exists())))
        }

        // --- Constants (WordPress / PHP compatibility) ---
        "define" => {
            if args.len() < 2 {
                return Err("define() expects at least 2 arguments".into());
            }
            let name = crate::engine::operators::zval_get_string(&args[0]);
            let name_s = name.as_str().to_string();
            let val = clone_val(&args[1]);
            _execute_data.constants.insert(name_s, val);
            Ok(Some(bool_val(true)))
        }
        "defined" => {
            if args.is_empty() {
                return Err("defined() expects 1 argument".into());
            }
            let name = crate::engine::operators::zval_get_string(&args[0]);
            let found = _execute_data.constants.contains_key(name.as_str());
            Ok(Some(bool_val(found)))
        }
        "constant" => {
            if args.is_empty() {
                return Err("constant() expects 1 argument".into());
            }
            let name = crate::engine::operators::zval_get_string(&args[0]);
            match _execute_data.constants.get(name.as_str()) {
                Some(v) => Ok(Some(clone_val(v))),
                None => Err(format!("constant {} undefined", name.as_str())),
            }
        }
        "dirname" => {
            if args.is_empty() {
                return Err("dirname() expects at least 1 argument".into());
            }
            let path = crate::engine::operators::zval_get_string(&args[0]);
            let dir = std::path::Path::new(path.as_str())
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok(Some(string_val(&dir)))
        }
        "exit" | "die" => {
            let code: i64 = if args.is_empty() {
                0
            } else if args[0].get_type() == crate::engine::types::PhpType::Long {
                crate::engine::operators::zval_get_long(&args[0])
            } else {
                let msg = crate::engine::operators::zval_get_string(&args[0]);
                let _ = crate::php::output::php_output_write(msg.as_str().as_bytes());
                let _ = crate::php::output::php_output_write(b"\n");
                0
            };
            _execute_data.exit_requested = Some(code);
            Ok(None)
        }

        // --- WordPress hooks (stubs) ---
        "do_action" => Ok(None),
        "apply_filters" => {
            if args.len() >= 2 {
                Ok(Some(clone_val(&args[1])))
            } else {
                Ok(None)
            }
        }

        // --- Info ---
        "phpversion" => Ok(Some(string_val("8.3.0-phprs"))),
        "phpinfo" => {
            let _ = crate::php::output::php_output_write(b"PHP-RS 0.1.0 (Rust implementation)\n");
            Ok(None)
        }

        _ => Ok(None), // Unknown function — return None to signal not found
    }
}
