//! PHP Reflection API implementation
//!
//! Provides ReflectionClass, ReflectionMethod, and ReflectionProperty
//! as builtin classes registered in the VM class table.

use crate::engine::types::{ClassEntry, PhpArray, PhpType, PhpValue, Val};
use crate::engine::vm::execute_data::{clone_val, ExecuteData};

/// Register built-in reflection classes into the execute_data class table
pub fn register_reflection_classes(execute_data: &mut ExecuteData) {
    register_reflection_class(execute_data);
    register_reflection_method(execute_data);
    register_reflection_property(execute_data);
}

fn register_reflection_class(execute_data: &mut ExecuteData) {
    let mut ce = ClassEntry::new("ReflectionClass");
    ce.default_properties.insert("name".to_string(), Val::new(PhpValue::Long(0), PhpType::Null));
    // Methods are handled specially in execute_do_method_call
    execute_data.class_table.insert("ReflectionClass".to_string(), ce);
}

fn register_reflection_method(execute_data: &mut ExecuteData) {
    let mut ce = ClassEntry::new("ReflectionMethod");
    ce.default_properties.insert("class".to_string(), Val::new(PhpValue::Long(0), PhpType::Null));
    ce.default_properties.insert("name".to_string(), Val::new(PhpValue::Long(0), PhpType::Null));
    execute_data.class_table.insert("ReflectionMethod".to_string(), ce);
}

fn register_reflection_property(execute_data: &mut ExecuteData) {
    let mut ce = ClassEntry::new("ReflectionProperty");
    ce.default_properties.insert("class".to_string(), Val::new(PhpValue::Long(0), PhpType::Null));
    ce.default_properties.insert("name".to_string(), Val::new(PhpValue::Long(0), PhpType::Null));
    execute_data.class_table.insert("ReflectionProperty".to_string(), ce);
}

/// Execute a ReflectionClass method. Called from execute_do_method_call when
/// the object class is "ReflectionClass".
pub fn execute_reflection_class_method(
    method_name: &str,
    args: &[Val],
    execute_data: &mut ExecuteData,
) -> Option<Val> {
    let this_val = execute_data.get_var("this");
    let reflected_name = if let PhpValue::Object(ref obj) = this_val.value {
        obj.properties.get("name").map(|v| {
            crate::engine::operators::zval_get_string(v).as_str().to_string()
        })
    } else {
        None
    };

    match method_name {
        "__construct" => {
            if let Some(class_name) = args.first() {
                let name = crate::engine::operators::zval_get_string(class_name).as_str().to_string();
                let mut updated = clone_val(&this_val);
                if let PhpValue::Object(ref mut obj_mut) = updated.value {
                    obj_mut.properties.insert("name".to_string(), Val::new(
                        PhpValue::String(Box::new(crate::engine::string::string_init(&name, false))),
                        PhpType::String,
                    ));
                }
                execute_data.set_var("this", updated);
            }
            None
        }
        "getName" => {
            reflected_name.map(|n| Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(&n, false))),
                PhpType::String,
            ))
        }
        "getMethods" => {
            let mut result = PhpArray::new();
            if let Some(ref cn) = reflected_name {
                if let Some(reflected_ce) = execute_data.class_table.get(cn) {
                    let mut idx: u64 = 0;
                    for (method_name, _method) in &reflected_ce.methods {
                        let val = Val::new(
                            PhpValue::String(Box::new(crate::engine::string::string_init(method_name, false))),
                            PhpType::String,
                        );
                        let _ = crate::engine::hash::hash_add_or_update(&mut result, None, idx, val, 0);
                        idx += 1;
                    }
                }
            }
            Some(Val::new(PhpValue::Array(Box::new(result)), PhpType::Array))
        }
        "getProperties" => {
            let mut result = PhpArray::new();
            if let Some(ref cn) = reflected_name {
                if let Some(reflected_ce) = execute_data.class_table.get(cn) {
                    let mut idx: u64 = 0;
                    for prop_name in reflected_ce.default_properties.keys() {
                        let val = Val::new(
                            PhpValue::String(Box::new(crate::engine::string::string_init(prop_name, false))),
                            PhpType::String,
                        );
                        let _ = crate::engine::hash::hash_add_or_update(&mut result, None, idx, val, 0);
                        idx += 1;
                    }
                }
            }
            Some(Val::new(PhpValue::Array(Box::new(result)), PhpType::Array))
        }
        "hasMethod" => {
            let exists = if let (Some(ref cn), Some(ref arg)) = (reflected_name, args.first()) {
                let method_name = crate::engine::operators::zval_get_string(arg).as_str().to_string();
                execute_data.class_table.get(cn).map(|ce| ce.methods.contains_key(&method_name)).unwrap_or(false)
            } else {
                false
            };
            Some(Val::new(PhpValue::Long(if exists { 1 } else { 0 }), if exists { PhpType::True } else { PhpType::False }))
        }
        "hasProperty" => {
            let exists = if let (Some(ref cn), Some(ref arg)) = (reflected_name, args.first()) {
                let prop_name = crate::engine::operators::zval_get_string(arg).as_str().to_string();
                execute_data.class_table.get(cn).map(|ce| {
                    ce.default_properties.contains_key(&prop_name)
                        || ce.static_properties.contains_key(&prop_name)
                }).unwrap_or(false)
            } else {
                false
            };
            Some(Val::new(PhpValue::Long(if exists { 1 } else { 0 }), if exists { PhpType::True } else { PhpType::False }))
        }
        "getParentClass" => {
            if let Some(ref cn) = reflected_name {
                execute_data.class_table.get(cn).and_then(|ce| ce.parent_name.clone()).map(|p| {
                    Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&p, false))), PhpType::String)
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Execute a ReflectionMethod method.
pub fn execute_reflection_method(
    method_name: &str,
    _args: &[Val],
    execute_data: &mut ExecuteData,
) -> Option<Val> {
    let this_val = execute_data.get_var("this");
    let (reflected_class, reflected_method) = if let PhpValue::Object(ref obj) = this_val.value {
        (
            obj.properties.get("class").map(|v| crate::engine::operators::zval_get_string(v).as_str().to_string()),
            obj.properties.get("name").map(|v| crate::engine::operators::zval_get_string(v).as_str().to_string()),
        )
    } else {
        (None, None)
    };

    match method_name {
        "__construct" => {
            if let (Some(class_val), Some(name_val)) = (_args.get(0), _args.get(1)) {
                let class_name = crate::engine::operators::zval_get_string(class_val).as_str().to_string();
                let method_name = crate::engine::operators::zval_get_string(name_val).as_str().to_string();
                let mut updated = clone_val(&this_val);
                if let PhpValue::Object(ref mut obj_mut) = updated.value {
                    obj_mut.properties.insert("class".to_string(), Val::new(
                        PhpValue::String(Box::new(crate::engine::string::string_init(&class_name, false))),
                        PhpType::String,
                    ));
                    obj_mut.properties.insert("name".to_string(), Val::new(
                        PhpValue::String(Box::new(crate::engine::string::string_init(&method_name, false))),
                        PhpType::String,
                    ));
                }
                execute_data.set_var("this", updated);
            }
            None
        }
        "getName" => {
            reflected_method.map(|n| Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(&n, false))),
                PhpType::String,
            ))
        }
        "getDeclaringClass" => {
            reflected_class.map(|n| Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(&n, false))),
                PhpType::String,
            ))
        }
        _ => None,
    }
}

/// Execute a ReflectionProperty method.
pub fn execute_reflection_property(
    method_name: &str,
    _args: &[Val],
    execute_data: &mut ExecuteData,
) -> Option<Val> {
    let this_val = execute_data.get_var("this");
    let (reflected_class, reflected_prop) = if let PhpValue::Object(ref obj) = this_val.value {
        (
            obj.properties.get("class").map(|v| crate::engine::operators::zval_get_string(v).as_str().to_string()),
            obj.properties.get("name").map(|v| crate::engine::operators::zval_get_string(v).as_str().to_string()),
        )
    } else {
        (None, None)
    };

    match method_name {
        "__construct" => {
            if let (Some(class_val), Some(name_val)) = (_args.get(0), _args.get(1)) {
                let class_name = crate::engine::operators::zval_get_string(class_val).as_str().to_string();
                let prop_name = crate::engine::operators::zval_get_string(name_val).as_str().to_string();
                let mut updated = clone_val(&this_val);
                if let PhpValue::Object(ref mut obj_mut) = updated.value {
                    obj_mut.properties.insert("class".to_string(), Val::new(
                        PhpValue::String(Box::new(crate::engine::string::string_init(&class_name, false))),
                        PhpType::String,
                    ));
                    obj_mut.properties.insert("name".to_string(), Val::new(
                        PhpValue::String(Box::new(crate::engine::string::string_init(&prop_name, false))),
                        PhpType::String,
                    ));
                }
                execute_data.set_var("this", updated);
            }
            None
        }
        "getName" => {
            reflected_prop.map(|n| Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(&n, false))),
                PhpType::String,
            ))
        }
        "getDeclaringClass" => {
            reflected_class.map(|n| Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(&n, false))),
                PhpType::String,
            ))
        }
        _ => None,
    }
}
