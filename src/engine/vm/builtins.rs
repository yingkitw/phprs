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
            let val = &args[0];
            let is_empty = match val.get_type() {
                PhpType::Null | PhpType::False | PhpType::Undef => true,
                PhpType::Long => crate::engine::operators::zval_get_long(val) == 0,
                PhpType::Double => crate::engine::operators::zval_get_double(val) == 0.0,
                PhpType::String => {
                    let s = crate::engine::operators::zval_get_string(val);
                    s.as_str().is_empty() || s.as_str() == "0"
                }
                PhpType::Array => {
                    if let PhpValue::Array(ref arr) = val.value {
                        arr.ar_data.is_empty()
                    } else {
                        false
                    }
                }
                _ => false,
            };
            Ok(Some(bool_val(is_empty)))
        }
        "unset" => {
            // unset is a language construct, but we provide a stub
            Ok(None)
        }
        "is_array" => Ok(Some(type_check(args, |t| t == PhpType::Array))),
        "is_string" => Ok(Some(type_check(args, |t| t == PhpType::String))),
        "is_int" | "is_integer" | "is_long" => Ok(Some(type_check(args, |t| t == PhpType::Long))),
        "is_float" | "is_double" => Ok(Some(type_check(args, |t| t == PhpType::Double))),
        "is_bool" => Ok(Some(type_check(args, |t| {
            t == PhpType::True || t == PhpType::False
        }))),
        "is_null" => Ok(Some(type_check(args, |t| t == PhpType::Null))),
        "is_numeric" => Ok(Some(type_check(args, |t| {
            t == PhpType::Long || t == PhpType::Double
        }))),
        "is_object" => Ok(Some(type_check(args, |t| t == PhpType::Object))),

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
            let path_str = path.as_str();

            // Check if it's an HTTP/HTTPS URL
            if path_str.starts_with("http://") || path_str.starts_with("https://") {
                match crate::php::http_stream::file_get_contents_http(path_str) {
                    Ok(content) => Ok(Some(string_val(&content))),
                    Err(e) => {
                        eprintln!("HTTP error: {}", e);
                        Ok(Some(Val::new(PhpValue::Long(0), PhpType::False)))
                    }
                }
            } else {
                // Local file
                let resolved = resolve_path_for_runtime(path_str, _execute_data);
                match std::fs::read_to_string(&resolved) {
                Ok(content) => Ok(Some(string_val(&content))),
                Err(_) => Ok(Some(Val::new(PhpValue::Long(0), PhpType::False))),
                }
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

        // --- HTML/String escaping ---
        "htmlspecialchars" => {
            if args.is_empty() {
                return Err("htmlspecialchars() expects at least 1 argument".into());
            }
            let s = crate::engine::operators::zval_get_string(&args[0]);
            let escaped = s
                .as_str()
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#039;");
            Ok(Some(string_val(&escaped)))
        }
        "htmlentities" => {
            if args.is_empty() {
                return Err("htmlentities() expects at least 1 argument".into());
            }
            let s = crate::engine::operators::zval_get_string(&args[0]);
            let escaped = s
                .as_str()
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#039;");
            Ok(Some(string_val(&escaped)))
        }

        // --- Regular Expressions ---
        "preg_match" => {
            if args.len() < 2 {
                return Err("preg_match() expects at least 2 arguments".into());
            }
            let pattern = crate::engine::operators::zval_get_string(&args[0]);
            let subject = crate::engine::operators::zval_get_string(&args[1]);

            match crate::php::regex::preg_match(pattern.as_str(), subject.as_str(), None) {
                Ok(result) => Ok(Some(Val::new(PhpValue::Long(result), PhpType::Long))),
                Err(e) => Err(format!("preg_match error: {}", e)),
            }
        }
        "preg_match_all" => {
            if args.len() < 2 {
                return Err("preg_match_all() expects at least 2 arguments".into());
            }
            let pattern = crate::engine::operators::zval_get_string(&args[0]);
            let subject = crate::engine::operators::zval_get_string(&args[1]);

            match crate::php::regex::preg_match_all(pattern.as_str(), subject.as_str()) {
                Ok(matches) => {
                    // Return count of matches
                    Ok(Some(Val::new(
                        PhpValue::Long(matches.len() as i64),
                        PhpType::Long,
                    )))
                }
                Err(e) => Err(format!("preg_match_all error: {}", e)),
            }
        }
        "preg_replace" => {
            if args.len() < 3 {
                return Err("preg_replace() expects at least 3 arguments".into());
            }
            let pattern = crate::engine::operators::zval_get_string(&args[0]);
            let replacement = crate::engine::operators::zval_get_string(&args[1]);
            let subject = crate::engine::operators::zval_get_string(&args[2]);

            match crate::php::regex::preg_replace(
                pattern.as_str(),
                replacement.as_str(),
                subject.as_str(),
            ) {
                Ok(result) => Ok(Some(string_val(&result))),
                Err(e) => Err(format!("preg_replace error: {}", e)),
            }
        }
        "preg_split" => {
            if args.len() < 2 {
                return Err("preg_split() expects at least 2 arguments".into());
            }
            let pattern = crate::engine::operators::zval_get_string(&args[0]);
            let subject = crate::engine::operators::zval_get_string(&args[1]);
            let limit = if args.len() > 2 {
                Some(crate::engine::operators::zval_get_long(&args[2]) as usize)
            } else {
                None
            };

            match crate::php::regex::preg_split(pattern.as_str(), subject.as_str(), limit) {
                Ok(parts) => {
                    let mut arr = crate::engine::types::PhpArray::new();
                    for (i, part) in parts.iter().enumerate() {
                        let val = string_val(part);
                        let _ = crate::engine::hash::hash_add_or_update(
                            &mut arr, None, i as u64, val, 0,
                        );
                    }
                    Ok(Some(Val::new(
                        PhpValue::Array(Box::new(arr)),
                        PhpType::Array,
                    )))
                }
                Err(e) => Err(format!("preg_split error: {}", e)),
            }
        }

        // --- WordPress/Array functions ---
        "shortcode_atts" => {
            // shortcode_atts($defaults, $atts) - merge attributes with defaults
            if args.len() < 2 {
                return Err("shortcode_atts() expects 2 arguments".into());
            }
            // For now, just return the second argument (atts)
            // In real implementation, this would merge with defaults
            Ok(Some(clone_val(&args[1])))
        }
        "esc_attr" => {
            if args.is_empty() {
                return Err("esc_attr() expects 1 argument".into());
            }
            let s = crate::engine::operators::zval_get_string(&args[0]);
            let escaped = s
                .as_str()
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#039;");
            Ok(Some(string_val(&escaped)))
        }
        "esc_url" => {
            if args.is_empty() {
                return Err("esc_url() expects 1 argument".into());
            }
            // For now, just return the URL as-is
            // Real implementation would sanitize URL
            Ok(Some(clone_val(&args[0])))
        }
        "ucfirst" => {
            if args.is_empty() {
                return Err("ucfirst() expects 1 argument".into());
            }
            let s = crate::engine::operators::zval_get_string(&args[0]);
            let s_str = s.as_str();
            if s_str.is_empty() {
                return Ok(Some(string_val("")));
            }
            let mut chars = s_str.chars();
            let first = chars.next().unwrap().to_uppercase().to_string();
            let rest: String = chars.collect();
            Ok(Some(string_val(&(first + &rest))))
        }

        // --- Info ---
        "phpversion" => Ok(Some(string_val("8.3.0-phprs"))),
        "phpinfo" => {
            let _ = crate::php::output::php_output_write(b"PHP-RS 0.1.0 (Rust implementation)\n");
            Ok(None)
        }

        // --- Output buffering ---
        "ob_start" => {
            crate::php::output::php_output_start()?;
            Ok(Some(bool_val(true)))
        }
        "ob_end_clean" => match crate::php::output::php_output_end_clean() {
            Ok(()) => Ok(Some(bool_val(true))),
            Err(_) => Ok(Some(bool_val(false))),
        },
        "ob_end_flush" => match crate::php::output::php_output_end_flush() {
            Ok(_) => Ok(Some(bool_val(true))),
            Err(_) => Ok(Some(bool_val(false))),
        },
        "ob_get_clean" => {
            let contents = crate::php::output::php_output_get_clean().unwrap_or_default();
            Ok(Some(string_val(&contents)))
        }
        "ob_get_flush" => {
            let contents = crate::php::output::php_output_get_flush().unwrap_or_default();
            Ok(Some(string_val(&contents)))
        }
        "ob_get_contents" => {
            let contents = crate::php::output::php_output_get_contents().unwrap_or_default();
            Ok(Some(string_val(&contents)))
        }
        "ob_get_level" => {
            let level = crate::php::output::php_output_get_level();
            Ok(Some(Val::new(PhpValue::Long(level as i64), PhpType::Long)))
        }
        "ob_clean" => {
            let _ = crate::php::output::php_output_clean();
            Ok(None)
        }
        "ob_flush" => {
            let _ = crate::php::output::php_output_flush();
            Ok(None)
        }
        "ob_implicit_flush" => Ok(None),

        // --- Error handling ---
        "set_error_handler" => {
            if args.is_empty() {
                return Err("set_error_handler() expects at least 1 argument".into());
            }
            let prev = _execute_data.error_handler.take();
            let handler_name = crate::engine::operators::zval_get_string(&args[0]);
            _execute_data.error_handler = Some(handler_name.as_str().to_string());
            Ok(Some(string_val(prev.as_deref().unwrap_or(""))))
        }
        "restore_error_handler" => {
            _execute_data.error_handler = None;
            Ok(Some(bool_val(true)))
        }
        "set_exception_handler" => {
            if args.is_empty() {
                return Err("set_exception_handler() expects at least 1 argument".into());
            }
            let prev = _execute_data.exception_handler.take();
            let handler_name = crate::engine::operators::zval_get_string(&args[0]);
            _execute_data.exception_handler = Some(handler_name.as_str().to_string());
            Ok(Some(string_val(prev.as_deref().unwrap_or(""))))
        }
        "restore_exception_handler" => {
            _execute_data.exception_handler = None;
            Ok(Some(bool_val(true)))
        }
        "register_shutdown_function" => {
            if args.is_empty() {
                return Err("register_shutdown_function() expects at least 1 argument".into());
            }
            let func = crate::engine::operators::zval_get_string(&args[0]);
            _execute_data
                .shutdown_functions
                .push(func.as_str().to_string());
            Ok(None)
        }
        "error_reporting" => Ok(Some(Val::new(PhpValue::Long(0), PhpType::Long))),
        "trigger_error" | "user_error" => {
            if args.is_empty() {
                return Err("trigger_error() expects at least 1 argument".into());
            }
            let msg = crate::engine::operators::zval_get_string(&args[0]);
            eprintln!("PHP User Error: {}", msg.as_str());
            Ok(Some(bool_val(true)))
        }
        "set_include_path" => Ok(Some(string_val(""))),
        "get_include_path" => Ok(Some(string_val("."))),
        "ini_set" => Ok(Some(Val::new(PhpValue::Long(0), PhpType::Null))),
        "ini_get" => Ok(Some(string_val(""))),

        // --- Math functions ---
        "abs" => crate::php::math::math_abs(args).map(Some),
        "ceil" => crate::php::math::math_ceil(args).map(Some),
        "floor" => crate::php::math::math_floor(args).map(Some),
        "round" => crate::php::math::math_round(args).map(Some),
        "sqrt" => crate::php::math::math_sqrt(args).map(Some),
        "pow" => crate::php::math::math_pow(args).map(Some),
        "exp" => crate::php::math::math_exp(args).map(Some),
        "log" => crate::php::math::math_log(args).map(Some),
        "log10" => crate::php::math::math_log10(args).map(Some),
        "sin" => crate::php::math::math_sin(args).map(Some),
        "cos" => crate::php::math::math_cos(args).map(Some),
        "tan" => crate::php::math::math_tan(args).map(Some),
        "asin" => crate::php::math::math_asin(args).map(Some),
        "acos" => crate::php::math::math_acos(args).map(Some),
        "atan" => crate::php::math::math_atan(args).map(Some),
        "atan2" => crate::php::math::math_atan2(args).map(Some),
        "pi" => crate::php::math::math_pi(args).map(Some),
        "max" => crate::php::math::math_max(args).map(Some),
        "min" => crate::php::math::math_min(args).map(Some),
        "rand" => crate::php::math::math_rand(args).map(Some),

        // --- Hash functions ---
        "md5" => crate::php::hash::hash_md5(args).map(Some),
        "sha1" => crate::php::hash::hash_sha1(args).map(Some),
        "hash" => crate::php::hash::hash_generic(args).map(Some),
        "hash_hmac" => crate::php::hash::hash_hmac(args).map(Some),
        "base64_encode" => crate::php::hash::base64_encode(args).map(Some),
        "base64_decode" => crate::php::hash::base64_decode(args).map(Some),
        "crc32" => crate::php::hash::crc32(args).map(Some),
        "bin2hex" => crate::php::hash::bin2hex(args).map(Some),
        "hex2bin" => crate::php::hash::hex2bin(args).map(Some),

        // --- Crypt functions ---
        "random_bytes" => crate::php::hash::random_bytes(args).map(Some),
        "random_int" => crate::php::hash::random_int(args).map(Some),
        "password_hash" => crate::php::hash::password_hash(args).map(Some),
        "password_verify" => crate::php::hash::password_verify(args).map(Some),

        // --- DateTime functions ---
        "time" => crate::php::datetime::time_now(args).map(Some),
        "microtime" => crate::php::datetime::microtime(args).map(Some),
        "date" => crate::php::datetime::date_format(args).map(Some),
        "mktime" => crate::php::datetime::mktime(args).map(Some),
        "strtotime" => crate::php::datetime::strtotime(args).map(Some),

        // --- URL functions ---
        "parse_url" => crate::php::url::parse_url(args).map(Some),
        "http_build_query" => crate::php::url::http_build_query(args).map(Some),
        "urlencode" => crate::php::url::urlencode(args).map(Some),
        "urldecode" => crate::php::url::urldecode(args).map(Some),
        "rawurlencode" => crate::php::url::rawurlencode(args).map(Some),
        "rawurldecode" => crate::php::url::rawurldecode(args).map(Some),
        "parse_str" => crate::php::url::parse_str(args).map(Some),
        "get_headers" => crate::php::url::get_headers(args).map(Some),

        // --- Multibyte string functions ---
        "mb_strlen" => crate::php::mbstring::mb_strlen(args).map(Some),
        "mb_substr" => crate::php::mbstring::mb_substr(args).map(Some),
        "mb_strtolower" => crate::php::mbstring::mb_strtolower(args).map(Some),
        "mb_strtoupper" => crate::php::mbstring::mb_strtoupper(args).map(Some),
        "mb_strpos" => crate::php::mbstring::mb_strpos(args).map(Some),
        "mb_strrpos" => crate::php::mbstring::mb_strrpos(args).map(Some),
        "mb_convert_encoding" => crate::php::mbstring::mb_convert_encoding(args).map(Some),
        "mb_substr_count" => crate::php::mbstring::mb_substr_count(args).map(Some),
        "mb_strwidth" => crate::php::mbstring::mb_strwidth(args).map(Some),
        "mb_strimwidth" => crate::php::mbstring::mb_strimwidth(args).map(Some),

        _ => Ok(None), // Unknown function — return None to signal not found
    }
}
