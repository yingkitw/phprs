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

/// Helper to create a null Val
fn null_val() -> Val {
    Val::new(PhpValue::Long(0), PhpType::Null)
}

/// Coerce an f64 into a PHP Long when integral, otherwise a Double — matches
/// PHP's numeric result shaping (e.g. `array_sum` of integers stays int).
fn numeric_val(n: f64) -> Val {
    if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
        Val::new(PhpValue::Long(n as i64), PhpType::Long)
    } else {
        Val::new(PhpValue::Double(n), PhpType::Double)
    }
}

/// PHP-style numeric string check: optional leading sign, digits, optional
/// fractional part, optional exponent. Whitespace-only padding is allowed.
fn is_numeric_string(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed.parse::<f64>().is_ok()
        && trimmed
            .chars()
            .next()
            .map(|c| c.is_ascii_digit() || c == '+' || c == '-' || c == '.')
            .unwrap_or(false)
}

/// Extract the precision (`.N`) from a printf spec like `%.2f` or `%5.2f`.
fn parse_precision(spec: &str) -> Option<usize> {
    let dot_idx = spec.find('.')?;
    let after = &spec[dot_idx + 1..];
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Minimal printf/sprintf supporting `%s`, `%d`, `%f`, `%%`, consuming args in order.
fn php_sprintf(fmt: &crate::engine::types::PhpString, args: &[Val]) -> String {
    let mut result = String::new();
    let mut arg_iter = args.iter();
    let bytes = fmt.as_str();
    let mut chars = bytes.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        if c != '%' {
            result.push(c);
            continue;
        }
        // Collect a simple conversion spec: optional flags/width then a verb.
        let mut spec = String::from('%');
        while let Some(&(_, pc)) = chars.peek() {
            if "0123456789.+- #".contains(pc) {
                spec.push(pc);
                chars.next();
            } else {
                break;
            }
        }
        match chars.next().map(|(_, pc)| pc) {
            Some('%') => result.push('%'),
            Some('s') => {
                let s = arg_iter
                    .next()
                    .map(|a| crate::engine::operators::zval_get_string(a).as_str().to_string())
                    .unwrap_or_default();
                result.push_str(&s);
            }
            Some('d' | 'i') => {
                let v = arg_iter
                    .next()
                    .map(|a| crate::engine::operators::zval_get_long(a))
                    .unwrap_or(0);
                result.push_str(&format!("{v}"));
            }
            Some('f' | 'F') => {
                let prec = parse_precision(&spec).unwrap_or(6);
                let v = arg_iter
                    .next()
                    .map(|a| crate::engine::operators::zval_get_double(a))
                    .unwrap_or(0.0);
                result.push_str(&format!("{v:.prec$}"));
            }
            Some('e' | 'E') => {
                let prec = parse_precision(&spec).unwrap_or(6);
                let v = arg_iter
                    .next()
                    .map(|a| crate::engine::operators::zval_get_double(a))
                    .unwrap_or(0.0);
                result.push_str(&format!("{v:.prec$e}"));
            }
            Some('g' | 'G') => {
                let v = arg_iter
                    .next()
                    .map(|a| crate::engine::operators::zval_get_double(a))
                    .unwrap_or(0.0);
                result.push_str(&format!("{v}"));
            }
            Some('x') => {
                let v = arg_iter
                    .next()
                    .map(|a| crate::engine::operators::zval_get_long(a))
                    .unwrap_or(0);
                result.push_str(&format!("{v:x}"));
            }
            Some('X') => {
                let v = arg_iter
                    .next()
                    .map(|a| crate::engine::operators::zval_get_long(a))
                    .unwrap_or(0);
                result.push_str(&format!("{v:X}"));
            }
            Some(other) => {
                spec.push(other);
                result.push_str(&spec);
            }
            None => result.push_str(&spec),
        }
    }
    result
}

/// Format an integer into a string in an arbitrary base (2..=36).
fn to_base(mut n: u64, base: u32) -> String {
    if n == 0 {
        return "0".to_string();
    }
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % base as u64) as usize] as char);
        n /= base as u64;
    }
    out.iter().rev().collect()
}

/// PHP `similar_text` common-character count (recursive longest-common-substring).
fn similar_text(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    sim_rec(&a, &b)
}
fn sim_rec(a: &[char], b: &[char]) -> usize {
    let (mut max, mut pos1, mut pos2) = (0usize, 0usize, 0usize);
    for i in 0..a.len() {
        for j in 0..b.len() {
            let mut k = 0;
            while i + k < a.len() && j + k < b.len() && a[i + k] == b[j + k] {
                k += 1;
            }
            if k > max {
                max = k;
                pos1 = i;
                pos2 = j;
            }
        }
    }
    if max == 0 {
        return 0;
    }
    let left = sim_rec(&a[..pos1], &b[..pos2]);
    let right = sim_rec(&a[pos1 + max..], &b[pos2 + max..]);
    max + left + right
}

/// Standard edit distance.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut cur = vec![0usize; m + 1];
    for i in 1..=n {
        cur[0] = i;
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m]
}

/// Standard 4-letter soundex code.
fn soundex(s: &str) -> String {
    let chars: Vec<char> = s.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if chars.is_empty() {
        return String::new();
    }
    let code = |c: char| -> Option<char> {
        match c.to_ascii_uppercase() {
            'B' | 'F' | 'P' | 'V' => Some('1'),
            'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => Some('2'),
            'D' | 'T' => Some('3'),
            'L' => Some('4'),
            'M' | 'N' => Some('5'),
            'R' => Some('6'),
            _ => None, // A E I O U H W Y
        }
    };
    let mut out = String::new();
    out.push(chars[0].to_ascii_uppercase());
    let mut prev = code(chars[0]);
    for &c in chars.iter().skip(1) {
        if out.len() == 4 {
            break;
        }
        if let Some(d) = code(c) {
            if Some(d) != prev {
                out.push(d);
            }
            prev = Some(d);
        } else if matches!(c.to_ascii_uppercase(), 'H' | 'W') {
            // keep previous code (don't reset)
        } else {
            prev = None;
        }
    }
    while out.len() < 4 {
        out.push('0');
    }
    out
}

/// Simplified metaphone approximation (not full Lawrence-Philips). Uppercases,
/// drops duplicate adjacent consonants, removes most vowels except the first.
fn simplified_metaphone(s: &str) -> String {
    let up: Vec<char> = s
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let vowels = |c: char| matches!(c, 'A' | 'E' | 'I' | 'O' | 'U');
    let mut out = String::new();
    let mut last = '\0';
    for (i, &c) in up.iter().enumerate() {
        if c == last {
            continue;
        }
        if vowels(c) && i != 0 {
            continue;
        }
        if c == 'H' || c == 'W' {
            continue;
        }
        out.push(c);
        last = c;
    }
    if out.is_empty() {
        out = up.iter().collect();
    }
    out
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

/// Check if a function name corresponds to a built-in
pub(crate) fn is_builtin_function(name: &str) -> bool {
    matches!(
        name,
        "strlen" | "strpos" | "substr" | "str_replace" | "strtolower" | "strtoupper"
        | "trim" | "explode" | "implode" | "join" | "sprintf" | "intval" | "floatval"
        | "doubleval" | "strval" | "isset" | "empty" | "unset" | "is_string"
        | "is_int" | "is_float" | "is_bool" | "is_array" | "is_null" | "is_object"
        | "array_key_exists" | "in_array" | "count" | "sizeof" | "array_push"
        | "array_merge" | "array_keys" | "array_values" | "array_pop" | "array_shift"
        | "array_slice" | "array_reverse" | "var_dump" | "print_r" | "echo" | "print" | "json_encode"
        | "json_decode" | "file_get_contents" | "file_exists" | "define" | "defined"
        | "constant" | "class_exists" | "interface_exists" | "trait_exists"
        | "method_exists" | "property_exists" | "function_exists" | "get_class"
        | "get_parent_class" | "gettype" | "spl_autoload_register"
        | "spl_autoload_unregister" | "spl_autoload_functions" | "phpversion"
        | "phpinfo" | "ob_start" | "ob_end_clean" | "ob_end_flush" | "ob_get_clean"
        | "ob_get_flush" | "ob_get_contents" | "ob_get_level" | "ob_clean" | "ob_flush"
        | "ob_implicit_flush" | "set_error_handler" | "set_exception_handler"
        | "register_shutdown_function" | "abs" | "ceil" | "floor" | "round"
        | "sqrt" | "pow" | "exp" | "log" | "log10" | "sin" | "cos" | "tan" | "asin"
        | "acos" | "atan" | "atan2" | "pi" | "max" | "min" | "rand" | "md5" | "sha1"
        | "hash" | "hash_hmac" | "base64_encode" | "base64_decode" | "crc32" | "bin2hex"
        | "hex2bin" | "random_bytes" | "random_int" | "password_hash" | "password_verify"
        | "time" | "microtime" | "date" | "mktime" | "strtotime" | "parse_url"
        | "http_build_query" | "urlencode" | "urldecode" | "rawurlencode" | "rawurldecode"
        | "parse_str" | "get_headers" | "str_getcsv" | "fgetcsv" | "fputcsv"
        | "session_start" | "session_destroy" | "session_id" | "session_name"
        | "gzcompress" | "gzuncompress" | "gzencode" | "gzdecode" | "gzdeflate"
        | "gzinflate" | "mb_strlen" | "mb_substr" | "mb_strtolower" | "mb_strtoupper"
        | "mb_strpos" | "mb_strrpos" | "mb_convert_encoding" | "mb_substr_count"
        | "mb_strwidth" | "mb_strimwidth" | "dirname" | "die" | "exit" | "do_action"
        | "apply_filters" | "shortcode_atts" | "htmlspecialchars"
        | "htmlspecialchars_decode" | "htmlentities" | "wordwrap" | "nl2br" | "str_repeat"
        | "str_pad" | "str_split" | "chunk_split" | "strip_tags" | "addslashes"
        | "stripslashes" | "quotemeta" | "ucwords" | "lcfirst" | "strrev" | "str_shuffle"
        | "str_contains" | "str_starts_with" | "str_ends_with" | "str_ireplace"
        | "strtr" | "similar_text" | "levenshtein" | "metaphone" | "soundex"
        | "number_format" | "money_format" | "decbin" | "bindec" | "decoct" | "octdec"
        | "dechex" | "hexdec" | "base_convert" | "deg2rad" | "rad2deg"
        // phprs additions: callbacks, array/string/math/type helpers
        | "call_user_func" | "call_user_func_array" | "array_map" | "array_filter"
        | "array_reduce" | "array_walk" | "array_combine" | "array_flip" | "array_search"
        | "array_unique" | "array_column" | "array_sum" | "array_product" | "array_chunk"
        | "array_diff" | "array_intersect" | "array_count_values" | "array_fill"
        | "array_pad" | "range" | "ucfirst" | "substr_count" | "substr_replace" | "strpbrk"
        | "substr_compare" | "intdiv" | "fmod" | "hypot" | "is_nan" | "is_infinite"
        | "is_finite" | "is_callable" | "boolval" | "serialize" | "unserialize"
    )
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
            Ok(Some(string_val(&php_sprintf(&fmt, &args[1..]))))
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
            // unset is normally a language construct; builtin hook returns no value.
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
        "is_numeric" => {
            if args.is_empty() {
                return Ok(Some(bool_val(false)));
            }
            // PHP: numeric strings (including hex is NOT numeric since PHP 7,
            // leading-sign + decimals + exponent) count as numeric.
            let is_num = match &args[0].value {
                PhpValue::Long(_) | PhpValue::Double(_) => true,
                PhpValue::String(s) => is_numeric_string(s.as_str()),
                _ => false,
            };
            Ok(Some(bool_val(is_num)))
        }
        "is_object" => Ok(Some(type_check(args, |t| t == PhpType::Object))),
        "is_callable" => {
            // Only string callables are resolvable here; treat known function
            // names (builtins or user functions) as callable.
            let callable = if args.is_empty() {
                return Ok(Some(bool_val(false)));
            } else {
                &args[0]
            };
            let is_cb = match &callable.value {
                PhpValue::String(s) => {
                    let n = s.as_str();
                    is_builtin_function(n)
                        || _execute_data
                            .function_table
                            .as_ref()
                            .and_then(|ft| {
                                ft.downcast_ref::<crate::engine::compile::function_table::FunctionTable>()
                            })
                            .map(|ft| ft.has_function(n))
                            .unwrap_or(false)
                }
                _ => false,
            };
            Ok(Some(bool_val(is_cb)))
        }
        "boolval" => {
            if args.is_empty() {
                return Ok(Some(bool_val(false)));
            }
            Ok(Some(bool_val(crate::engine::operators::zval_get_bool(&args[0]))))
        }

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
        "array_keys" => {
            if args.is_empty() {
                return Ok(Some(Val::new(PhpValue::Array(Box::new(crate::engine::types::PhpArray::new())), PhpType::Array)));
            }
            let mut result = crate::engine::types::PhpArray::new();
            if let PhpValue::Array(ref arr) = args[0].value {
                let mut idx: u64 = 0;
                for bucket in &arr.ar_data {
                    let key_val = if let Some(ref k) = bucket.key {
                        Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(k.as_str(), false))), PhpType::String)
                    } else {
                        Val::new(PhpValue::Long(idx as i64), PhpType::Long)
                    };
                    let _ = crate::engine::hash::hash_add_or_update(&mut result, None, idx, key_val, 0);
                    idx += 1;
                }
            }
            Ok(Some(Val::new(PhpValue::Array(Box::new(result)), PhpType::Array)))
        }
        "array_values" => {
            if args.is_empty() {
                return Ok(Some(Val::new(PhpValue::Array(Box::new(crate::engine::types::PhpArray::new())), PhpType::Array)));
            }
            let mut result = crate::engine::types::PhpArray::new();
            if let PhpValue::Array(ref arr) = args[0].value {
                let mut idx: u64 = 0;
                for bucket in &arr.ar_data {
                    let val = clone_val(&bucket.val);
                    let _ = crate::engine::hash::hash_add_or_update(&mut result, None, idx, val, 0);
                    idx += 1;
                }
            }
            Ok(Some(Val::new(PhpValue::Array(Box::new(result)), PhpType::Array)))
        }
        "array_pop" => {
            if args.is_empty() {
                return Ok(Some(null_val()));
            }
            if let PhpValue::Array(ref arr) = args[0].value {
                if let Some(bucket) = arr.ar_data.last() {
                    Ok(Some(clone_val(&bucket.val)))
                } else {
                    Ok(Some(null_val()))
                }
            } else {
                Ok(Some(null_val()))
            }
        }
        "array_shift" => {
            if args.is_empty() {
                return Ok(Some(null_val()));
            }
            if let PhpValue::Array(ref arr) = args[0].value {
                if let Some(bucket) = arr.ar_data.first() {
                    Ok(Some(clone_val(&bucket.val)))
                } else {
                    Ok(Some(null_val()))
                }
            } else {
                Ok(Some(null_val()))
            }
        }
        "array_slice" => {
            if args.len() < 2 {
                return Err("array_slice() expects at least 2 arguments".into());
            }
            let offset = crate::engine::operators::zval_get_long(&args[1]) as usize;
            let length = if args.len() >= 3 {
                crate::engine::operators::zval_get_long(&args[2]) as usize
            } else {
                usize::MAX
            };
            let mut result = crate::engine::types::PhpArray::new();
            if let PhpValue::Array(ref arr) = args[0].value {
                let mut idx: u64 = 0;
                for bucket in arr.ar_data.iter().skip(offset).take(length) {
                    let val = clone_val(&bucket.val);
                    if let Some(ref k) = bucket.key {
                        let key = crate::engine::string::string_init(k.as_str(), false);
                        let _ = crate::engine::hash::hash_add_or_update(&mut result, Some(&key), 0, val, 0);
                    } else {
                        let _ = crate::engine::hash::hash_add_or_update(&mut result, None, idx, val, 0);
                        idx += 1;
                    }
                }
            }
            Ok(Some(Val::new(PhpValue::Array(Box::new(result)), PhpType::Array)))
        }
        "array_reverse" => {
            if args.is_empty() {
                return Ok(Some(Val::new(PhpValue::Array(Box::new(crate::engine::types::PhpArray::new())), PhpType::Array)));
            }
            let mut result = crate::engine::types::PhpArray::new();
            if let PhpValue::Array(ref arr) = args[0].value {
                let preserve_keys = args.get(1).map(|a| crate::engine::operators::zval_get_bool(a)).unwrap_or(false);
                let mut idx: u64 = 0;
                for bucket in arr.ar_data.iter().rev() {
                    let val = clone_val(&bucket.val);
                    if preserve_keys {
                        if let Some(ref k) = bucket.key {
                            let key = crate::engine::string::string_init(k.as_str(), false);
                            let _ = crate::engine::hash::hash_add_or_update(&mut result, Some(&key), 0, val, 0);
                        } else {
                            let _ = crate::engine::hash::hash_add_or_update(&mut result, None, bucket.h, val, 0);
                        }
                    } else {
                        let _ = crate::engine::hash::hash_add_or_update(&mut result, None, idx, val, 0);
                        idx += 1;
                    }
                }
            }
            Ok(Some(Val::new(PhpValue::Array(Box::new(result)), PhpType::Array)))
        }
        "array_push" => {
            if args.len() < 2 {
                return Err("array_push() expects at least 2 arguments".into());
            }
            let current_count = if let PhpValue::Array(ref arr) = args[0].value {
                arr.ar_data.len()
            } else {
                0
            };
            Ok(Some(Val::new(PhpValue::Long((current_count + args.len() - 1) as i64), PhpType::Long)))
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

        // --- Callback-driven functions ---
        "call_user_func" => {
            if args.is_empty() {
                return Err("call_user_func() expects at least 1 argument".into());
            }
            let (cb, rest) = args.split_first().unwrap();
            super::callable::invoke_callable(_execute_data, cb, rest)
        }
        "call_user_func_array" => {
            if args.len() < 2 {
                return Err("call_user_func_array() expects 2 arguments".into());
            }
            let mut call_args: Vec<Val> = Vec::new();
            if let PhpValue::Array(ref arr) = args[1].value {
                for bucket in &arr.ar_data {
                    call_args.push(clone_val(&bucket.val));
                }
            }
            super::callable::invoke_callable(_execute_data, &args[0], &call_args)
        }
        "array_map" => {
            if args.len() < 2 {
                return Err("array_map() expects at least 2 arguments".into());
            }
            let mut result = crate::engine::types::PhpArray::new();
            if let PhpValue::Array(ref arr) = args[1].value {
                for (i, bucket) in arr.ar_data.iter().enumerate() {
                    let mapped = super::callable::invoke_callable(
                        _execute_data,
                        &args[0],
                        &[clone_val(&bucket.val)],
                    )?;
                    let val = mapped.unwrap_or_else(null_val);
                    if let Some(ref k) = bucket.key {
                        let key = crate::engine::string::string_init(k.as_str(), false);
                        let _ = crate::engine::hash::hash_add_or_update(
                            &mut result,
                            Some(&key),
                            0,
                            val,
                            0,
                        );
                    } else {
                        let _ =
                            crate::engine::hash::hash_add_or_update(&mut result, None, i as u64, val, 0);
                    }
                }
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
                PhpType::Array,
            )))
        }
        "array_filter" => {
            if args.is_empty() {
                return Err("array_filter() expects at least 1 argument".into());
            }
            let use_callback = args.len() > 1;
            let mut result = crate::engine::types::PhpArray::new();
            if let PhpValue::Array(ref arr) = args[0].value {
                for bucket in &arr.ar_data {
                    let keep = if use_callback {
                        let r = super::callable::invoke_callable(
                            _execute_data,
                            &args[1],
                            &[clone_val(&bucket.val)],
                        )?;
                        crate::engine::operators::zval_get_bool(&r.unwrap_or_else(null_val))
                    } else {
                        // No callback: keep truthy values (PHP default behavior).
                        crate::engine::operators::zval_get_bool(&bucket.val)
                    };
                    if keep {
                        let val = clone_val(&bucket.val);
                        if let Some(ref k) = bucket.key {
                            let key = crate::engine::string::string_init(k.as_str(), false);
                            let _ = crate::engine::hash::hash_add_or_update(
                                &mut result,
                                Some(&key),
                                0,
                                val,
                                0,
                            );
                        } else {
                            let _ = crate::engine::hash::hash_add_or_update(
                                &mut result,
                                None,
                                bucket.h,
                                val,
                                0,
                            );
                        }
                    }
                }
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
                PhpType::Array,
            )))
        }
        "array_reduce" => {
            if args.len() < 2 {
                return Err("array_reduce() expects at least 2 arguments".into());
            }
            let mut acc = if args.len() > 2 { clone_val(&args[2]) } else { null_val() };
            if let PhpValue::Array(ref arr) = args[0].value {
                for bucket in &arr.ar_data {
                    let r = super::callable::invoke_callable(
                        _execute_data,
                        &args[1],
                        &[acc.clone(), clone_val(&bucket.val)],
                    )?;
                    acc = r.unwrap_or_else(null_val);
                }
            }
            Ok(Some(acc))
        }
        "array_walk" => {
            if args.len() < 2 {
                return Err("array_walk() expects at least 2 arguments".into());
            }
            if let PhpValue::Array(ref arr) = args[0].value {
                for bucket in &arr.ar_data {
                    let key_arg = if let Some(ref k) = bucket.key {
                        string_val(k.as_str())
                    } else {
                        Val::new(PhpValue::Long(bucket.h as i64), PhpType::Long)
                    };
                    let _ = super::callable::invoke_callable(
                        _execute_data,
                        &args[1],
                        &[clone_val(&bucket.val), key_arg],
                    )?;
                }
            }
            Ok(Some(bool_val(true)))
        }

        // --- Additional array functions ---
        "array_combine" => {
            if args.len() < 2 {
                return Err("array_combine() expects 2 arguments".into());
            }
            let mut result = crate::engine::types::PhpArray::new();
            if let (PhpValue::Array(keys), PhpValue::Array(vals)) =
                (&args[0].value, &args[1].value)
            {
                for (k, v) in keys.ar_data.iter().zip(vals.ar_data.iter()) {
                    let key_str = crate::engine::operators::zval_get_string(&k.val);
                    let key = crate::engine::string::string_init(key_str.as_str(), false);
                    let _ = crate::engine::hash::hash_add_or_update(
                        &mut result,
                        Some(&key),
                        0,
                        clone_val(&v.val),
                        0,
                    );
                }
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
                PhpType::Array,
            )))
        }
        "array_flip" => {
            if args.is_empty() {
                return Ok(Some(Val::new(
                    PhpValue::Array(Box::new(crate::engine::types::PhpArray::new())),
                    PhpType::Array,
                )));
            }
            let mut result = crate::engine::types::PhpArray::new();
            if let PhpValue::Array(ref arr) = args[0].value {
                for bucket in &arr.ar_data {
                    let val_str = crate::engine::operators::zval_get_string(&bucket.val);
                    let key = crate::engine::string::string_init(val_str.as_str(), false);
                    let new_val = if let Some(ref k) = bucket.key {
                        string_val(k.as_str())
                    } else {
                        Val::new(PhpValue::Long(bucket.h as i64), PhpType::Long)
                    };
                    let _ = crate::engine::hash::hash_add_or_update(
                        &mut result,
                        Some(&key),
                        0,
                        new_val,
                        0,
                    );
                }
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
                PhpType::Array,
            )))
        }
        "array_search" => {
            if args.len() < 2 {
                return Err("array_search() expects at least 2 arguments".into());
            }
            let needle = crate::engine::operators::zval_get_string(&args[0]);
            if let PhpValue::Array(ref arr) = args[1].value {
                for bucket in &arr.ar_data {
                    let v = crate::engine::operators::zval_get_string(&bucket.val);
                    if v.as_str() == needle.as_str() {
                        if let Some(ref k) = bucket.key {
                            return Ok(Some(string_val(k.as_str())));
                        }
                        return Ok(Some(Val::new(
                            PhpValue::Long(bucket.h as i64),
                            PhpType::Long,
                        )));
                    }
                }
            }
            Ok(Some(bool_val(false)))
        }
        "array_unique" => {
            if args.is_empty() {
                return Ok(Some(Val::new(
                    PhpValue::Array(Box::new(crate::engine::types::PhpArray::new())),
                    PhpType::Array,
                )));
            }
            let mut result = crate::engine::types::PhpArray::new();
            let mut seen: Vec<String> = Vec::new();
            if let PhpValue::Array(ref arr) = args[0].value {
                for bucket in &arr.ar_data {
                    let v = crate::engine::operators::zval_get_string(&bucket.val);
                    if seen.iter().any(|s| s.as_str() == v.as_str()) {
                        continue;
                    }
                    seen.push(v.as_str().to_string());
                    let val = clone_val(&bucket.val);
                    if let Some(ref k) = bucket.key {
                        let key = crate::engine::string::string_init(k.as_str(), false);
                        let _ = crate::engine::hash::hash_add_or_update(
                            &mut result,
                            Some(&key),
                            0,
                            val,
                            0,
                        );
                    } else {
                        let _ = crate::engine::hash::hash_add_or_update(
                            &mut result,
                            None,
                            bucket.h,
                            val,
                            0,
                        );
                    }
                }
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
                PhpType::Array,
            )))
        }
        "array_column" => {
            if args.len() < 2 {
                return Err("array_column() expects at least 2 arguments".into());
            }
            let col = crate::engine::operators::zval_get_string(&args[1]);
            let mut result = crate::engine::types::PhpArray::new();
            let mut idx: u64 = 0;
            if let PhpValue::Array(ref arr) = args[0].value {
                for bucket in &arr.ar_data {
                    if let PhpValue::Array(ref row) = bucket.val.value {
                        if let Some(target) = row.ar_data.iter().find(|b| {
                            b.key
                                .as_ref()
                                .map(|k| k.as_str() == col.as_str())
                                .unwrap_or(false)
                        }) {
                            let _ = crate::engine::hash::hash_add_or_update(
                                &mut result,
                                None,
                                idx,
                                clone_val(&target.val),
                                0,
                            );
                            idx += 1;
                        }
                    }
                }
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
                PhpType::Array,
            )))
        }
        "array_sum" => {
            let mut sum: f64 = 0.0;
            if !args.is_empty() {
                if let PhpValue::Array(ref arr) = args[0].value {
                    for bucket in &arr.ar_data {
                        sum += crate::engine::operators::zval_get_double(&bucket.val);
                    }
                }
            }
            Ok(Some(numeric_val(sum)))
        }
        "array_product" => {
            let mut product: f64 = 1.0;
            if !args.is_empty() {
                if let PhpValue::Array(ref arr) = args[0].value {
                    for bucket in &arr.ar_data {
                        product *= crate::engine::operators::zval_get_double(&bucket.val);
                    }
                }
            }
            Ok(Some(numeric_val(product)))
        }
        "array_chunk" => {
            if args.len() < 2 {
                return Err("array_chunk() expects at least 2 arguments".into());
            }
            let size = crate::engine::operators::zval_get_long(&args[1]).max(1) as usize;
            let mut result = crate::engine::types::PhpArray::new();
            let mut chunk_idx: u64 = 0;
            if let PhpValue::Array(ref arr) = args[0].value {
                for batch in arr.ar_data.chunks(size) {
                    let mut chunk = crate::engine::types::PhpArray::new();
                    let mut inner: u64 = 0;
                    for bucket in batch {
                        let val = clone_val(&bucket.val);
                        if let Some(ref k) = bucket.key {
                            let key = crate::engine::string::string_init(k.as_str(), false);
                            let _ = crate::engine::hash::hash_add_or_update(
                                &mut chunk,
                                Some(&key),
                                0,
                                val,
                                0,
                            );
                        } else {
                            let _ = crate::engine::hash::hash_add_or_update(
                                &mut chunk,
                                None,
                                inner,
                                val,
                                0,
                            );
                            inner += 1;
                        }
                    }
                    let _ = crate::engine::hash::hash_add_or_update(
                        &mut result,
                        None,
                        chunk_idx,
                        Val::new(PhpValue::Array(Box::new(chunk)), PhpType::Array),
                        0,
                    );
                    chunk_idx += 1;
                }
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
                PhpType::Array,
            )))
        }
        "array_diff" => {
            if args.is_empty() {
                return Ok(Some(Val::new(
                    PhpValue::Array(Box::new(crate::engine::types::PhpArray::new())),
                    PhpType::Array,
                )));
            }
            let other_vals: Vec<String> = args[1..]
                .iter()
                .filter_map(|a| match &a.value {
                    PhpValue::Array(arr) => Some(
                        arr.ar_data
                            .iter()
                            .map(|b| crate::engine::operators::zval_get_string(&b.val).as_str().to_string())
                            .collect::<Vec<_>>(),
                    ),
                    _ => None,
                })
                .flatten()
                .collect();
            let mut result = crate::engine::types::PhpArray::new();
            if let PhpValue::Array(ref arr) = args[0].value {
                for bucket in &arr.ar_data {
                    let v = crate::engine::operators::zval_get_string(&bucket.val);
                    if other_vals.iter().any(|s| s.as_str() == v.as_str()) {
                        continue;
                    }
                    let val = clone_val(&bucket.val);
                    if let Some(ref k) = bucket.key {
                        let key = crate::engine::string::string_init(k.as_str(), false);
                        let _ = crate::engine::hash::hash_add_or_update(
                            &mut result,
                            Some(&key),
                            0,
                            val,
                            0,
                        );
                    } else {
                        let _ = crate::engine::hash::hash_add_or_update(
                            &mut result,
                            None,
                            bucket.h,
                            val,
                            0,
                        );
                    }
                }
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
                PhpType::Array,
            )))
        }
        "array_intersect" => {
            if args.is_empty() {
                return Ok(Some(Val::new(
                    PhpValue::Array(Box::new(crate::engine::types::PhpArray::new())),
                    PhpType::Array,
                )));
            }
            let other_vals: Vec<String> = args[1..]
                .iter()
                .filter_map(|a| match &a.value {
                    PhpValue::Array(arr) => Some(
                        arr.ar_data
                            .iter()
                            .map(|b| crate::engine::operators::zval_get_string(&b.val).as_str().to_string())
                            .collect::<Vec<_>>(),
                    ),
                    _ => None,
                })
                .flatten()
                .collect();
            let mut result = crate::engine::types::PhpArray::new();
            if let PhpValue::Array(ref arr) = args[0].value {
                for bucket in &arr.ar_data {
                    let v = crate::engine::operators::zval_get_string(&bucket.val);
                    if !other_vals.iter().any(|s| s.as_str() == v.as_str()) {
                        continue;
                    }
                    let val = clone_val(&bucket.val);
                    if let Some(ref k) = bucket.key {
                        let key = crate::engine::string::string_init(k.as_str(), false);
                        let _ = crate::engine::hash::hash_add_or_update(
                            &mut result,
                            Some(&key),
                            0,
                            val,
                            0,
                        );
                    } else {
                        let _ = crate::engine::hash::hash_add_or_update(
                            &mut result,
                            None,
                            bucket.h,
                            val,
                            0,
                        );
                    }
                }
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
                PhpType::Array,
            )))
        }
        "array_count_values" => {
            if args.is_empty() {
                return Ok(Some(Val::new(
                    PhpValue::Array(Box::new(crate::engine::types::PhpArray::new())),
                    PhpType::Array,
                )));
            }
            let mut result = crate::engine::types::PhpArray::new();
            if let PhpValue::Array(ref arr) = args[0].value {
                for bucket in &arr.ar_data {
                    let v = crate::engine::operators::zval_get_string(&bucket.val);
                    let key = crate::engine::string::string_init(v.as_str(), false);
                    let existing = crate::engine::hash::hash_find(&result, &key)
                        .map(|x| crate::engine::operators::zval_get_long(x))
                        .unwrap_or(0);
                    let _ = crate::engine::hash::hash_add_or_update(
                        &mut result,
                        Some(&key),
                        0,
                        Val::new(PhpValue::Long(existing + 1), PhpType::Long),
                        0,
                    );
                }
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
                PhpType::Array,
            )))
        }
        "array_fill" => {
            if args.len() < 3 {
                return Err("array_fill() expects 3 arguments".into());
            }
            let start = crate::engine::operators::zval_get_long(&args[0]);
            let count = crate::engine::operators::zval_get_long(&args[1]).max(0) as u64;
            let mut result = crate::engine::types::PhpArray::new();
            for i in 0..count {
                let _ = crate::engine::hash::hash_add_or_update(
                    &mut result,
                    None,
                    (start as u64) + i,
                    clone_val(&args[2]),
                    0,
                );
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
                PhpType::Array,
            )))
        }
        "array_pad" => {
            if args.len() < 3 {
                return Err("array_pad() expects 3 arguments".into());
            }
            let size = crate::engine::operators::zval_get_long(&args[1]);
            let pad_val = clone_val(&args[2]);
            let mut result = crate::engine::types::PhpArray::new();
            let entries: Vec<Val> = if let PhpValue::Array(ref arr) = args[0].value {
                arr.ar_data.iter().map(|b| clone_val(&b.val)).collect()
            } else {
                Vec::new()
            };
            let current = entries.len() as i64;
            let target = size.abs();
            let pad_count = (target - current).max(0);
            let mut idx: u64 = 0;
            let emit = |result: &mut crate::engine::types::PhpArray, idx: &mut u64, v: Val| {
                let _ = crate::engine::hash::hash_add_or_update(result, None, *idx, v, 0);
                *idx += 1;
            };
            if size < 0 {
                for _ in 0..pad_count {
                    emit(&mut result, &mut idx, clone_val(&pad_val));
                }
                for v in entries {
                    emit(&mut result, &mut idx, v);
                }
            } else {
                for v in entries {
                    emit(&mut result, &mut idx, v);
                }
                for _ in 0..pad_count {
                    emit(&mut result, &mut idx, clone_val(&pad_val));
                }
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
                PhpType::Array,
            )))
        }
        "range" => {
            if args.len() < 2 {
                return Err("range() expects at least 2 arguments".into());
            }
            let start = crate::engine::operators::zval_get_double(&args[0]);
            let end = crate::engine::operators::zval_get_double(&args[1]);
            let step = if args.len() > 2 {
                crate::engine::operators::zval_get_double(&args[2]).abs().max(1.0)
            } else {
                1.0
            };
            let mut result = crate::engine::types::PhpArray::new();
            let mut idx: u64 = 0;
            let push_val = |result: &mut crate::engine::types::PhpArray, idx: &mut u64, v: f64| {
                let _ = crate::engine::hash::hash_add_or_update(
                    result,
                    None,
                    *idx,
                    numeric_val(v),
                    0,
                );
                *idx += 1;
            };
            if start <= end {
                let mut x = start;
                while x <= end + 1e-9 {
                    push_val(&mut result, &mut idx, x);
                    x += step;
                }
            } else {
                let mut x = start;
                while x >= end - 1e-9 {
                    push_val(&mut result, &mut idx, x);
                    x -= step;
                }
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
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

        // --- WordPress hook shims (minimal behavior) ---
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
            // Returns $atts only; merging with $defaults is not implemented.
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
            // Returns the URL unchanged (no extra sanitization).
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
        "substr_count" => {
            if args.len() < 2 {
                return Err("substr_count() expects at least 2 arguments".into());
            }
            let haystack = crate::engine::operators::zval_get_string(&args[0]);
            let needle = crate::engine::operators::zval_get_string(&args[1]);
            if needle.as_str().is_empty() {
                return Ok(Some(Val::new(PhpValue::Long(0), PhpType::Long)));
            }
            let count = haystack.as_str().matches(needle.as_str()).count();
            Ok(Some(Val::new(PhpValue::Long(count as i64), PhpType::Long)))
        }
        "substr_replace" => {
            if args.len() < 3 {
                return Err("substr_replace() expects at least 3 arguments".into());
            }
            let s = crate::engine::operators::zval_get_string(&args[0]);
            let replacement = crate::engine::operators::zval_get_string(&args[1]);
            let src = s.as_str();
            let src_len = src.len() as i64;
            let mut start = crate::engine::operators::zval_get_long(&args[2]);
            if start < 0 {
                start = (src_len + start).max(0);
            }
            let start = (start.min(src_len)).max(0) as usize;
            let length = if args.len() > 3 {
                let l = crate::engine::operators::zval_get_long(&args[3]);
                if l < 0 {
                    (src_len + l - start as i64).max(0) as usize
                } else {
                    l as usize
                }
            } else {
                src_len as usize - start
            };
            let end = (start + length).min(src.len());
            let mut out = String::with_capacity(src.len() + replacement.as_str().len());
            out.push_str(&src[..start]);
            out.push_str(replacement.as_str());
            if end < src.len() {
                out.push_str(&src[end..]);
            }
            Ok(Some(string_val(&out)))
        }
        "strpbrk" => {
            if args.len() < 2 {
                return Err("strpbrk() expects 2 arguments".into());
            }
            let s = crate::engine::operators::zval_get_string(&args[0]);
            let chars = crate::engine::operators::zval_get_string(&args[1]);
            let src = s.as_str();
            let char_set: std::collections::HashSet<char> = chars.as_str().chars().collect();
            match src.find(|c: char| char_set.contains(&c)) {
                Some(pos) => Ok(Some(string_val(&src[pos..]))),
                None => Ok(Some(bool_val(false))),
            }
        }
        "substr_compare" => {
            if args.len() < 3 {
                return Err("substr_compare() expects at least 3 arguments".into());
            }
            let main = crate::engine::operators::zval_get_string(&args[0]);
            let other = crate::engine::operators::zval_get_string(&args[1]);
            let offset = crate::engine::operators::zval_get_long(&args[2]);
            let main_str = main.as_str();
            let main_len = main_str.len() as i64;
            let start = if offset < 0 {
                (main_len + offset).max(0) as usize
            } else {
                (offset as usize).min(main_len as usize)
            };
            let slice = if let Some(len_arg) = args.get(3) {
                let l = crate::engine::operators::zval_get_long(len_arg) as usize;
                &main_str[start..(start + l).min(main_str.len())]
            } else {
                &main_str[start..]
            };
            let case_insensitive = args.get(4).map(|a| crate::engine::operators::zval_get_bool(a)).unwrap_or(false);
            let (a, b): (String, String) = if case_insensitive {
                (slice.to_lowercase(), other.as_str().to_lowercase())
            } else {
                (slice.to_string(), other.as_str().to_string())
            };
            let cmp = a.as_str().cmp(b.as_str());
            Ok(Some(Val::new(
                PhpValue::Long(match cmp {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                }),
                PhpType::Long,
            )))
        }

        // --- Info ---
        "phpversion" => Ok(Some(string_val("8.3.0-phprs"))),
        "phpinfo" => {
            let _ = crate::php::output::php_output_write(b"PHP-RS 0.1.0 (Rust implementation)\n");
            Ok(None)
        }

        // --- Class / object introspection ---
        "class_exists" => {
            if args.is_empty() {
                return Ok(Some(bool_val(false)));
            }
            let name = crate::engine::operators::zval_get_string(&args[0]).as_str().to_string();
            let exists = _execute_data.class_table.contains_key(&name);
            Ok(Some(bool_val(exists)))
        }
        "interface_exists" => {
            if args.is_empty() {
                return Ok(Some(bool_val(false)));
            }
            let name = crate::engine::operators::zval_get_string(&args[0]).as_str().to_string();
            let exists = _execute_data.class_table.get(&name).map(|ce| ce.methods.is_empty() && ce.default_properties.is_empty() && ce.static_properties.is_empty()).unwrap_or(false);
            Ok(Some(bool_val(exists)))
        }
        "trait_exists" => {
            if args.is_empty() {
                return Ok(Some(bool_val(false)));
            }
            let name = crate::engine::operators::zval_get_string(&args[0]).as_str().to_string();
            let trait_key = format!("__trait_{}", name);
            let exists = _execute_data.class_table.contains_key(&trait_key);
            Ok(Some(bool_val(exists)))
        }
        "method_exists" => {
            if args.len() < 2 {
                return Ok(Some(bool_val(false)));
            }
            let method_name = crate::engine::operators::zval_get_string(&args[1]).as_str().to_string();
            let class_name = match &args[0].value {
                PhpValue::Object(obj) => Some(obj.class_name.clone()),
                _ => {
                    let s = crate::engine::operators::zval_get_string(&args[0]).as_str().to_string();
                    Some(s)
                }
            };
            let exists = class_name.and_then(|cn| {
                _execute_data.class_table.get(&cn).map(|ce| ce.methods.contains_key(&method_name))
            }).unwrap_or(false);
            Ok(Some(bool_val(exists)))
        }
        "property_exists" => {
            if args.len() < 2 {
                return Ok(Some(bool_val(false)));
            }
            let prop_name = crate::engine::operators::zval_get_string(&args[1]).as_str().to_string();
            let class_name = match &args[0].value {
                PhpValue::Object(obj) => Some(obj.class_name.clone()),
                _ => {
                    let s = crate::engine::operators::zval_get_string(&args[0]).as_str().to_string();
                    Some(s)
                }
            };
            let exists = class_name.and_then(|cn| {
                _execute_data.class_table.get(&cn).map(|ce| {
                    ce.default_properties.contains_key(&prop_name)
                        || ce.static_properties.contains_key(&prop_name)
                })
            }).unwrap_or(false);
            Ok(Some(bool_val(exists)))
        }
        "function_exists" => {
            if args.is_empty() {
                return Ok(Some(bool_val(false)));
            }
            let name = crate::engine::operators::zval_get_string(&args[0]).as_str().to_string();
            let user_exists = _execute_data.function_table.as_ref().and_then(|ft| {
                ft.downcast_ref::<crate::engine::compile::function_table::FunctionTable>()
            }).map(|ft| ft.has_function(&name)).unwrap_or(false);
            Ok(Some(bool_val(user_exists || is_builtin_function(&name))))
        }
        "get_class" => {
            let class_name = if args.is_empty() {
                match &_execute_data.get_var("this").value {
                    PhpValue::Object(obj) => Some(obj.class_name.clone()),
                    _ => None,
                }
            } else {
                match &args[0].value {
                    PhpValue::Object(obj) => Some(obj.class_name.clone()),
                    _ => None,
                }
            };
            match class_name {
                Some(cn) => Ok(Some(string_val(&cn))),
                None => Ok(Some(bool_val(false))),
            }
        }
        "get_parent_class" => {
            let class_name = if args.is_empty() {
                match &_execute_data.get_var("this").value {
                    PhpValue::Object(obj) => Some(obj.class_name.clone()),
                    _ => None,
                }
            } else {
                match &args[0].value {
                    PhpValue::Object(obj) => Some(obj.class_name.clone()),
                    _ => {
                        let s = crate::engine::operators::zval_get_string(&args[0]).as_str().to_string();
                        Some(s)
                    }
                }
            };
            let parent = class_name.and_then(|cn| {
                _execute_data.class_table.get(&cn).and_then(|ce| ce.parent_name.clone())
            });
            match parent {
                Some(ref p) => Ok(Some(string_val(p))),
                None => Ok(Some(bool_val(false))),
            }
        }
        "gettype" => {
            if args.is_empty() {
                return Ok(Some(string_val("NULL")));
            }
            let t = match args[0].get_type() {
                PhpType::Null => "NULL",
                PhpType::False | PhpType::True | PhpType::Bool => "boolean",
                PhpType::Long | PhpType::Double => "integer",
                PhpType::String => "string",
                PhpType::Array => "array",
                PhpType::Object => "object",
                PhpType::Resource => "resource",
                PhpType::Callable => "callable",
                PhpType::ConstantAst => "string",
                _ => "unknown type",
            };
            Ok(Some(string_val(t)))
        }

        // --- SPL autoloading ---
        "spl_autoload_register" => {
            if args.is_empty() {
                return Ok(Some(bool_val(false)));
            }
            let cb = crate::engine::operators::zval_get_string(&args[0]).as_str().to_string();
            if !_execute_data.autoload_functions.contains(&cb) {
                _execute_data.autoload_functions.push(cb);
            }
            Ok(Some(bool_val(true)))
        }
        "spl_autoload_unregister" => {
            if args.is_empty() {
                return Ok(Some(bool_val(false)));
            }
            let cb = crate::engine::operators::zval_get_string(&args[0]).as_str().to_string();
            let before = _execute_data.autoload_functions.len();
            _execute_data.autoload_functions.retain(|f| f != &cb);
            Ok(Some(bool_val(_execute_data.autoload_functions.len() < before)))
        }
        "spl_autoload_functions" => {
            let mut arr = crate::engine::types::PhpArray::new();
            for (i, f) in _execute_data.autoload_functions.iter().enumerate() {
                let bucket = crate::engine::types::Bucket {
                    val: string_val(f),
                    h: i as u64,
                    key: None,
                };
                arr.ar_data.push(bucket);
                arr.n_num_used += 1;
                arr.n_num_of_elements += 1;
            }
            arr.n_next_free_element = _execute_data.autoload_functions.len() as i64;
            Ok(Some(Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array)))
        }

        // --- Output buffering ---
        "ob_start" => {
            if args.is_empty() {
                crate::php::output::php_output_start()?;
            } else {
                let cb = crate::engine::operators::zval_get_string(&args[0]).as_str().to_string();
                crate::php::output::php_output_start_with_callback(cb)?;
            }
            Ok(Some(bool_val(true)))
        }
        "ob_end_clean" => match crate::php::output::php_output_end_clean() {
            Ok(()) => Ok(Some(bool_val(true))),
            Err(_) => Ok(Some(bool_val(false))),
        },
        "ob_end_flush" => {
            match crate::php::output::php_output_take() {
                Ok((contents, callback)) => {
                    let out = if let Some(cb_name) = callback {
                        let cb_arg = string_val(&contents);
                        match execute_builtin_function(&cb_name, &[cb_arg], _execute_data)? {
                            Some(result) => crate::engine::operators::zval_get_string(&result).as_str().to_string(),
                            None => contents,
                        }
                    } else {
                        contents
                    };
                    let _ = crate::php::output::php_output_write_to_active(out.as_bytes());
                    Ok(Some(bool_val(true)))
                }
                Err(_) => Ok(Some(bool_val(false))),
            }
        }
        "ob_get_clean" => {
            let contents = crate::php::output::php_output_get_clean().unwrap_or_default();
            Ok(Some(string_val(&contents)))
        }
        "ob_get_flush" => {
            match crate::php::output::php_output_take_clean() {
                Ok((contents, callback)) => {
                    let out = if let Some(cb_name) = callback {
                        let cb_arg = string_val(&contents);
                        match execute_builtin_function(&cb_name, &[cb_arg], _execute_data)? {
                            Some(result) => crate::engine::operators::zval_get_string(&result).as_str().to_string(),
                            None => contents,
                        }
                    } else {
                        contents
                    };
                    let _ = crate::php::output::php_output_write_to_active(out.as_bytes());
                    Ok(Some(string_val(&out)))
                }
                Err(_) => Ok(Some(string_val(""))),
            }
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

        // --- Additional math functions ---
        "intdiv" => {
            if args.len() < 2 {
                return Err("intdiv() expects 2 arguments".into());
            }
            let a = crate::engine::operators::zval_get_long(&args[0]);
            let b = crate::engine::operators::zval_get_long(&args[1]);
            if b == 0 {
                return Err("Division by zero in intdiv()".into());
            }
            Ok(Some(Val::new(PhpValue::Long(a / b), PhpType::Long)))
        }
        "fmod" => {
            if args.len() < 2 {
                return Err("fmod() expects 2 arguments".into());
            }
            let a = crate::engine::operators::zval_get_double(&args[0]);
            let b = crate::engine::operators::zval_get_double(&args[1]);
            Ok(Some(numeric_val(a % b)))
        }
        "hypot" => {
            if args.len() < 2 {
                return Err("hypot() expects 2 arguments".into());
            }
            let a = crate::engine::operators::zval_get_double(&args[0]);
            let b = crate::engine::operators::zval_get_double(&args[1]);
            Ok(Some(Val::new(PhpValue::Double(a.hypot(b)), PhpType::Double)))
        }
        "is_nan" => {
            if args.is_empty() {
                return Ok(Some(bool_val(false)));
            }
            Ok(Some(bool_val(
                crate::engine::operators::zval_get_double(&args[0]).is_nan(),
            )))
        }
        "is_infinite" => {
            if args.is_empty() {
                return Ok(Some(bool_val(false)));
            }
            Ok(Some(bool_val(
                crate::engine::operators::zval_get_double(&args[0]).is_infinite(),
            )))
        }
        "is_finite" => {
            if args.is_empty() {
                return Ok(Some(bool_val(false)));
            }
            Ok(Some(bool_val(
                crate::engine::operators::zval_get_double(&args[0]).is_finite(),
            )))
        }

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

        // --- Session functions ---
        "session_start" => {
            let ok = crate::php::session::session_start(_execute_data)?;
            Ok(Some(bool_val(ok)))
        }
        "session_destroy" => {
            let ok = crate::php::session::session_destroy(_execute_data)?;
            Ok(Some(bool_val(ok)))
        }
        "session_id" => {
            let id = crate::php::session::session_id(args, _execute_data)?;
            Ok(Some(string_val(&id)))
        }
        "session_name" => {
            let name = crate::php::session::session_name(args, _execute_data)?;
            Ok(Some(string_val(&name)))
        }

        // --- CSV functions ---
        "str_getcsv" => crate::php::csv::str_getcsv(args).map(Some),
        "fgetcsv" => crate::php::csv::fgetcsv(args).map(Some),
        "fputcsv" => crate::php::csv::fputcsv(args).map(Some),

        // --- Compression functions ---
        "gzcompress" => crate::php::compression::gzcompress(args).map(Some),
        "gzuncompress" => crate::php::compression::gzuncompress(args).map(Some),
        "gzencode" => crate::php::compression::gzencode(args).map(Some),
        "gzdecode" => crate::php::compression::gzdecode(args).map(Some),
        "gzdeflate" => crate::php::compression::gzdeflate(args).map(Some),
        "gzinflate" => crate::php::compression::gzinflate(args).map(Some),

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

        // --- Serialization ---
        "serialize" => crate::php::serialize::php_serialize(args).map(Some),
        "unserialize" => crate::php::serialize::php_unserialize(args).map(Some),

        // --- phprs: previously-listed string helpers (now implemented) ---
        "str_repeat" => {
            if args.len() < 2 {
                return Err("str_repeat() expects 2 arguments".into());
            }
            let s = crate::engine::operators::zval_get_string(&args[0]);
            let n = crate::engine::operators::zval_get_long(&args[1]).max(0) as usize;
            Ok(Some(string_val(&s.as_str().repeat(n))))
        }
        "ucwords" => {
            let s = require_string_arg(args, "ucwords")?;
            let mut out = String::with_capacity(s.len());
            let mut prev_alpha = false;
            for c in s.chars() {
                if c.is_alphabetic() {
                    out.push(if prev_alpha { c } else { c.to_ascii_uppercase() });
                    prev_alpha = true;
                } else {
                    out.push(c);
                    prev_alpha = false;
                }
            }
            Ok(Some(string_val(&out)))
        }
        "lcfirst" => {
            let s = require_string_arg(args, "lcfirst")?;
            let mut chars = s.chars();
            match chars.next() {
                Some(first) => {
                    let rest: String = chars.collect();
                    Ok(Some(string_val(&(first.to_lowercase().to_string() + &rest))))
                }
                None => Ok(Some(string_val(""))),
            }
        }
        "str_split" => {
            let s = require_string_arg(args, "str_split")?;
            let chunk = args
                .get(1)
                .map(|a| crate::engine::operators::zval_get_long(a).max(1) as usize)
                .unwrap_or(1);
            let mut result = crate::engine::types::PhpArray::new();
            let mut idx: u64 = 0;
            let mut buf = String::new();
            for c in s.chars() {
                buf.push(c);
                if buf.chars().count() >= chunk {
                    let _ = crate::engine::hash::hash_add_or_update(
                        &mut result,
                        None,
                        idx,
                        string_val(&buf),
                        0,
                    );
                    idx += 1;
                    buf.clear();
                }
            }
            if !buf.is_empty() {
                let _ = crate::engine::hash::hash_add_or_update(
                    &mut result,
                    None,
                    idx,
                    string_val(&buf),
                    0,
                );
            }
            Ok(Some(Val::new(
                PhpValue::Array(Box::new(result)),
                PhpType::Array,
            )))
        }
        "strrev" => {
            let s = require_string_arg(args, "strrev")?;
            // PHP reverses bytes; for ASCII/UTF-8 we reverse by char for readability.
            Ok(Some(string_val(&s.chars().rev().collect::<String>())))
        }
        "str_contains" => {
            if args.len() < 2 {
                return Err("str_contains() expects 2 arguments".into());
            }
            let h = crate::engine::operators::zval_get_string(&args[0]);
            let n = crate::engine::operators::zval_get_string(&args[1]);
            Ok(Some(bool_val(h.as_str().contains(n.as_str()))))
        }
        "str_starts_with" => {
            if args.len() < 2 {
                return Err("str_starts_with() expects 2 arguments".into());
            }
            let h = crate::engine::operators::zval_get_string(&args[0]);
            let n = crate::engine::operators::zval_get_string(&args[1]);
            Ok(Some(bool_val(h.as_str().starts_with(n.as_str()))))
        }
        "str_ends_with" => {
            if args.len() < 2 {
                return Err("str_ends_with() expects 2 arguments".into());
            }
            let h = crate::engine::operators::zval_get_string(&args[0]);
            let n = crate::engine::operators::zval_get_string(&args[1]);
            Ok(Some(bool_val(h.as_str().ends_with(n.as_str()))))
        }
        "strtr" => {
            if args.len() < 2 {
                return Err("strtr() expects at least 2 arguments".into());
            }
            let s = crate::engine::operators::zval_get_string(&args[0]);
            // Two forms: strtr(str, from, to) char-map, or strtr(str, replace_assoc).
            if args.len() >= 3 {
                let from = crate::engine::operators::zval_get_string(&args[1]);
                let to = crate::engine::operators::zval_get_string(&args[2]);
                let out: String = s
                    .as_str()
                    .chars()
                    .map(|c| {
                        if let Some(pos) = from.as_str().find(c) {
                            to.as_str().chars().nth(pos).unwrap_or(c)
                        } else {
                            c
                        }
                    })
                    .collect();
                Ok(Some(string_val(&out)))
            } else if let PhpValue::Array(arr) = &args[1].value {
                let mut out = s.as_str().to_string();
                // Longest-key-first so longer substrings win.
                let mut pairs: Vec<(String, String)> = arr
                    .ar_data
                    .iter()
                    .filter_map(|b| {
                        b.key.as_ref().map(|k| {
                            (k.as_str().to_string(), crate::engine::operators::zval_get_string(&b.val).as_str().to_string())
                        })
                    })
                    .collect();
                pairs.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
                for (from, to) in pairs {
                    if !from.is_empty() {
                        out = out.replace(&from, &to);
                    }
                }
                Ok(Some(string_val(&out)))
            } else {
                Ok(Some(string_val(s.as_str())))
            }
        }
        "str_ireplace" => {
            if args.len() < 3 {
                return Err("str_ireplace() expects 3 arguments".into());
            }
            let search = crate::engine::operators::zval_get_string(&args[0]);
            let replace = crate::engine::operators::zval_get_string(&args[1]);
            let subject = crate::engine::operators::zval_get_string(&args[2]);
            let s_low = subject.as_str().to_lowercase();
            let needle_low = search.as_str().to_lowercase();
            let mut out = String::with_capacity(subject.as_str().len());
            let bytes = subject.as_str();
            let mut i = 0;
            while i < bytes.len() {
                if let Some(pos) = s_low[i..].find(&needle_low) {
                    out.push_str(&bytes[i..i + pos]);
                    out.push_str(replace.as_str());
                    i += pos + needle_low.len();
                } else {
                    out.push_str(&bytes[i..]);
                    break;
                }
            }
            Ok(Some(string_val(&out)))
        }
        "nl2br" => {
            let s = require_string_arg(args, "nl2br")?;
            let out = s.replace("\r\n", "<br />\r\n").replace('\n', "<br />\n").replace('\r', "<br />\r");
            Ok(Some(string_val(&out)))
        }
        "chunk_split" => {
            let s = require_string_arg(args, "chunk_split")?;
            let chunklen = args
                .get(1)
                .map(|a| crate::engine::operators::zval_get_long(a).max(1) as usize)
                .unwrap_or(76);
            let end = args
                .get(2)
                .map(|a| crate::engine::operators::zval_get_string(a).as_str().to_string())
                .unwrap_or_else(|| "\r\n".to_string());
            let mut out = String::new();
            let bytes = s.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                let take = chunklen.min(bytes.len() - i);
                out.push_str(std::str::from_utf8(&bytes[i..i + take]).unwrap_or(""));
                out.push_str(&end);
                i += take;
            }
            Ok(Some(string_val(&out)))
        }
        "addslashes" => {
            let s = require_string_arg(args, "addslashes")?;
            let mut out = String::with_capacity(s.len());
            for c in s.chars() {
                match c {
                    '\'' | '"' | '\\' => {
                        out.push('\\');
                        out.push(c);
                    }
                    '\0' => out.push_str("\\0"),
                    _ => out.push(c),
                }
            }
            Ok(Some(string_val(&out)))
        }
        "stripslashes" => {
            let s = require_string_arg(args, "stripslashes")?;
            let mut out = String::with_capacity(s.len());
            let mut chars = s.chars();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    match chars.next() {
                        Some('0') => out.push('\0'),
                        Some(next) => out.push(next),
                        None => out.push('\\'),
                    }
                } else {
                    out.push(c);
                }
            }
            Ok(Some(string_val(&out)))
        }
        "quotemeta" => {
            let s = require_string_arg(args, "quotemeta")?;
            let mut out = String::with_capacity(s.len());
            for c in s.chars() {
                if matches!(c, '.' | '\\' | '+' | '*' | '?' | '[' | '^' | ']' | '(' | ')' | '$') {
                    out.push('\\');
                }
                out.push(c);
            }
            Ok(Some(string_val(&out)))
        }
        "strip_tags" => {
            let s = require_string_arg(args, "strip_tags")?;
            let mut out = String::new();
            let mut in_tag = false;
            for c in s.chars() {
                if c == '<' {
                    in_tag = true;
                } else if c == '>' {
                    in_tag = false;
                } else if !in_tag {
                    out.push(c);
                }
            }
            Ok(Some(string_val(&out)))
        }
        "htmlspecialchars_decode" => {
            let s = require_string_arg(args, "htmlspecialchars_decode")?;
            let out = s
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&#039;", "'")
                .replace("&#39;", "'")
                .replace("&amp;", "&");
            Ok(Some(string_val(&out)))
        }
        "wordwrap" => {
            let s = require_string_arg(args, "wordwrap")?;
            let width = args
                .get(1)
                .map(|a| crate::engine::operators::zval_get_long(a).max(1) as usize)
                .unwrap_or(75);
            let brk = args
                .get(2)
                .map(|a| crate::engine::operators::zval_get_string(a).as_str().to_string())
                .unwrap_or_else(|| "\n".to_string());
            let cut = args.get(3).map(|a| crate::engine::operators::zval_get_bool(a)).unwrap_or(false);
            let mut out = String::new();
            for (i, line) in s.split('\n').enumerate() {
                if i > 0 {
                    out.push('\n');
                }
                let mut line_len = 0usize;
                let words: Vec<&str> = line.split(' ').collect();
                for (w_i, word) in words.iter().enumerate() {
                    if w_i > 0 {
                        if line_len + 1 + word.chars().count() > width {
                            out.push_str(&brk);
                            line_len = 0;
                        } else {
                            out.push(' ');
                            line_len += 1;
                        }
                    }
                    if cut && word.chars().count() > width {
                        let mut count = 0;
                        for c in word.chars() {
                            if count == width {
                                out.push_str(&brk);
                                count = 0;
                            }
                            out.push(c);
                            count += 1;
                            line_len = count;
                        }
                    } else {
                        out.push_str(word);
                        line_len += word.chars().count();
                    }
                }
            }
            Ok(Some(string_val(&out)))
        }
        "number_format" => {
            let num = args
                .first()
                .map(crate::engine::operators::zval_get_double)
                .unwrap_or(0.0);
            let decimals = args
                .get(1)
                .map(crate::engine::operators::zval_get_long)
                .unwrap_or(0)
                .max(0) as usize;
            let dec_sep = args
                .get(2)
                .map(|a| crate::engine::operators::zval_get_string(a).as_str().to_string())
                .unwrap_or_else(|| ".".to_string());
            let thou_sep = args
                .get(3)
                .map(|a| crate::engine::operators::zval_get_string(a).as_str().to_string())
                .unwrap_or_else(|| ",".to_string());
            let factor = 10f64.powi(decimals as i32);
            let rounded = (num * factor).round() / factor;
            let neg = rounded < 0.0;
            let abs = rounded.abs();
            let scaled = (abs * factor).round() as u64;
            let int_part = scaled / factor as u64;
            let frac_part = scaled % factor as u64;
            let int_str = int_part.to_string();
            let grouped = {
                let bytes = int_str.as_bytes();
                let mut out = String::new();
                for (i, b) in bytes.iter().enumerate() {
                    if i > 0 && (bytes.len() - i) % 3 == 0 {
                        out.push_str(&thou_sep);
                    }
                    out.push(*b as char);
                }
                out
            };
            let mut result = grouped;
            if decimals > 0 {
                result.push_str(&dec_sep);
                let frac_str = format!("{:0width$}", frac_part, width = decimals);
                result.push_str(&frac_str);
            }
            if neg {
                result.insert(0, '-');
            }
            Ok(Some(string_val(&result)))
        }
        "money_format" => {
            // money_format() is locale-dependent and deprecated as of PHP 8.4.
            // Best-effort: format the number with 2 decimals. Not locale-aware.
            let num = args
                .get(1)
                .map(crate::engine::operators::zval_get_double)
                .unwrap_or_else(|| {
                    args.first()
                        .map(crate::engine::operators::zval_get_double)
                        .unwrap_or(0.0)
                });
            Ok(Some(string_val(&format!("{:.2}", num))))
        }
        "vsprintf" => {
            if args.len() < 2 {
                return Err("vsprintf() expects 2 arguments".into());
            }
            let mut collected: Vec<Val> = Vec::new();
            if let PhpValue::Array(arr) = &args[1].value {
                for b in &arr.ar_data {
                    collected.push(clone_val(&b.val));
                }
            }
            Ok(Some(string_val(&php_sprintf(
                &crate::engine::operators::zval_get_string(&args[0]),
                &collected,
            ))))
        }

        // --- phprs: base-conversion / trig helpers ---
        "decbin" => {
            let n = crate::engine::operators::zval_get_long(args.first().unwrap_or(&null_val()));
            Ok(Some(string_val(&format!("{:b}", n))))
        }
        "decoct" => {
            let n = crate::engine::operators::zval_get_long(args.first().unwrap_or(&null_val()));
            Ok(Some(string_val(&format!("{:o}", n))))
        }
        "dechex" => {
            let n = crate::engine::operators::zval_get_long(args.first().unwrap_or(&null_val()));
            Ok(Some(string_val(&format!("{:x}", n))))
        }
        "bindec" => {
            let s = require_string_arg(args, "bindec")?;
            let n = i64::from_str_radix(s.trim().trim_start_matches("0b"), 2).unwrap_or(0);
            Ok(Some(Val::new(PhpValue::Long(n), PhpType::Long)))
        }
        "octdec" => {
            let s = require_string_arg(args, "octdec")?;
            let n = i64::from_str_radix(s.trim(), 8).unwrap_or(0);
            Ok(Some(Val::new(PhpValue::Long(n), PhpType::Long)))
        }
        "hexdec" => {
            let s = require_string_arg(args, "hexdec")?;
            let n = i64::from_str_radix(s.trim().trim_start_matches("0x"), 16).unwrap_or(0);
            Ok(Some(Val::new(PhpValue::Long(n), PhpType::Long)))
        }
        "base_convert" => {
            if args.len() < 3 {
                return Err("base_convert() expects 3 arguments".into());
            }
            let s = crate::engine::operators::zval_get_string(&args[0]);
            let from = crate::engine::operators::zval_get_long(&args[1]).clamp(2, 36) as u32;
            let to = crate::engine::operators::zval_get_long(&args[2]).clamp(2, 36) as u32;
            let n = u64::from_str_radix(s.as_str().trim(), from).map_err(|_| "base_convert(): invalid".to_string())?;
            Ok(Some(string_val(&to_base(n, to))))
        }
        "deg2rad" => Ok(Some(Val::new(
            PhpValue::Double(
                crate::engine::operators::zval_get_double(args.first().unwrap_or(&null_val())).to_radians(),
            ),
            PhpType::Double,
        ))),
        "rad2deg" => Ok(Some(Val::new(
            PhpValue::Double(
                crate::engine::operators::zval_get_double(args.first().unwrap_or(&null_val())).to_degrees(),
            ),
            PhpType::Double,
        ))),

        // --- phprs: fuzzy string comparison ---
        "similar_text" => {
            if args.len() < 2 {
                return Err("similar_text() expects at least 2 arguments".into());
            }
            let a = crate::engine::operators::zval_get_string(&args[0]);
            let b = crate::engine::operators::zval_get_string(&args[1]);
            let sim = similar_text(a.as_str(), b.as_str());
            Ok(Some(Val::new(PhpValue::Long(sim as i64), PhpType::Long)))
        }
        "levenshtein" => {
            if args.len() < 2 {
                return Err("levenshtein() expects 2 arguments".into());
            }
            let a = crate::engine::operators::zval_get_string(&args[0]);
            let b = crate::engine::operators::zval_get_string(&args[1]);
            let dist = levenshtein(a.as_str(), b.as_str());
            Ok(Some(Val::new(PhpValue::Long(dist as i64), PhpType::Long)))
        }
        "soundex" => {
            let s = require_string_arg(args, "soundex")?;
            Ok(Some(string_val(&soundex(&s))))
        }
        "metaphone" => {
            // Simplified metaphone: uppercase, collapse duplicate consonants,
            // drop silent leading patterns. Not a full Lawrence-Philips metaphone.
            let s = require_string_arg(args, "metaphone")?;
            Ok(Some(string_val(&simplified_metaphone(&s))))
        }

        _ => Ok(None), // Unknown function — return None to signal not found
    }
}
