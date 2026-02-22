//! Value formatting utilities (var_dump, print_r, json)

use crate::engine::types::{PhpType, PhpValue, Val};

/// var_dump formatting
pub(crate) fn var_dump_value(val: &Val) -> String {
    match val.get_type() {
        PhpType::Null => "NULL\n".to_string(),
        PhpType::True => "bool(true)\n".to_string(),
        PhpType::False => "bool(false)\n".to_string(),
        PhpType::Long => format!("int({})\n", crate::engine::operators::zval_get_long(val)),
        PhpType::Double => format!(
            "float({})\n",
            crate::engine::operators::zval_get_double(val)
        ),
        PhpType::String => {
            let s = crate::engine::operators::zval_get_string(val);
            format!("string({}) \"{}\"\n", s.as_str().len(), s.as_str())
        }
        PhpType::Array => {
            if let PhpValue::Array(ref arr) = val.value {
                let mut out = format!("array({}) {{\n", arr.ar_data.len());
                for bucket in &arr.ar_data {
                    if let Some(ref key) = bucket.key {
                        out.push_str(&format!("  [\"{}\"]=>\n  ", key.as_str()));
                    } else {
                        out.push_str(&format!("  [{}]=>\n  ", bucket.h));
                    }
                    out.push_str(&var_dump_value(&bucket.val));
                }
                out.push_str("}\n");
                out
            } else {
                "array(0) {}\n".to_string()
            }
        }
        _ => format!("unknown type({})\n", val.type_info),
    }
}

/// print_r formatting
pub(crate) fn print_r_value(val: &Val) -> String {
    match val.get_type() {
        PhpType::Null => String::new(),
        PhpType::True => "1".to_string(),
        PhpType::False => String::new(),
        PhpType::Long => format!("{}", crate::engine::operators::zval_get_long(val)),
        PhpType::Double => format!("{}", crate::engine::operators::zval_get_double(val)),
        PhpType::String => crate::engine::operators::zval_get_string(val)
            .as_str()
            .to_string(),
        PhpType::Array => {
            if let PhpValue::Array(ref arr) = val.value {
                let mut out = "Array\n(\n".to_string();
                for bucket in &arr.ar_data {
                    if let Some(ref key) = bucket.key {
                        out.push_str(&format!(
                            "    [{}] => {}\n",
                            key.as_str(),
                            print_r_value(&bucket.val)
                        ));
                    } else {
                        out.push_str(&format!(
                            "    [{}] => {}\n",
                            bucket.h,
                            print_r_value(&bucket.val)
                        ));
                    }
                }
                out.push_str(")\n");
                out
            } else {
                "Array\n(\n)\n".to_string()
            }
        }
        _ => String::new(),
    }
}

/// Convert Val to JSON string
pub(crate) fn zval_to_json(val: &Val) -> String {
    match val.get_type() {
        PhpType::Null => "null".to_string(),
        PhpType::True => "true".to_string(),
        PhpType::False => "false".to_string(),
        PhpType::Long => format!("{}", crate::engine::operators::zval_get_long(val)),
        PhpType::Double => format!("{}", crate::engine::operators::zval_get_double(val)),
        PhpType::String => {
            let s = crate::engine::operators::zval_get_string(val);
            format!(
                "\"{}\"",
                s.as_str().replace('\\', "\\\\").replace('"', "\\\"")
            )
        }
        PhpType::Array => {
            if let PhpValue::Array(ref arr) = val.value {
                let is_list = arr.ar_data.iter().all(|b| b.key.is_none());
                if is_list {
                    let items: Vec<String> =
                        arr.ar_data.iter().map(|b| zval_to_json(&b.val)).collect();
                    format!("[{}]", items.join(","))
                } else {
                    let items: Vec<String> = arr
                        .ar_data
                        .iter()
                        .map(|b| {
                            let key = b
                                .key
                                .as_ref()
                                .map(|k| format!("\"{}\"", k.as_str()))
                                .unwrap_or_else(|| format!("{}", b.h));
                            format!("{}:{}", key, zval_to_json(&b.val))
                        })
                        .collect();
                    format!("{{{}}}", items.join(","))
                }
            } else {
                "[]".to_string()
            }
        }
        _ => "null".to_string(),
    }
}
