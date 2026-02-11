//! Val Factory - Facade for creating Val instances
//!
//! Provides a centralized interface for creating Val instances,
//! reducing code duplication and improving consistency.

use crate::engine::string::string_init;
use crate::engine::types::{PhpType, PhpValue, Val};

/// Factory trait for creating Val instances
pub trait ValFactory {
    /// Create a boolean Val
    fn bool_val(value: bool) -> Val;

    /// Create a long (integer) Val
    fn long_val(value: i64) -> Val;

    /// Create a double (float) Val
    fn double_val(value: f64) -> Val;

    /// Create a string Val
    fn string_val(value: &str) -> Val;

    /// Create a null Val
    fn null_val() -> Val;

    /// Create an array Val
    fn array_val() -> Val;

    /// Create a default/zero Val (useful for opcodes that need operands)
    fn zero_val() -> Val;

    /// Create a result Val for operations
    fn result_val(php_type: PhpType) -> Val;

    /// Clone a Val's value (deep copy for complex types)
    fn clone_val(source: &Val) -> Val;

    /// Create a copy of a Val for use as a result operand
    fn result_dup(source: &Val) -> Val;

    /// Create a string Val by copying an existing PhpString
    fn string_val_copy(value: &str, php_type: PhpType) -> Val;
}

/// Default implementation of ValFactory
pub struct StdValFactory;

impl ValFactory for StdValFactory {
    fn bool_val(value: bool) -> Val {
        // Use True/False instead of Bool for actual boolean values
        // Bool is a fake type used for type hinting (value 18)
        // True and False are the actual runtime types (values 2 and 3)
        Val::new(
            PhpValue::Long(if value { 1 } else { 0 }),
            if value {
                PhpType::True
            } else {
                PhpType::False
            },
        )
    }

    fn long_val(value: i64) -> Val {
        Val::new(PhpValue::Long(value), PhpType::Long)
    }

    fn double_val(value: f64) -> Val {
        Val::new(PhpValue::Double(value), PhpType::Double)
    }

    fn string_val(value: &str) -> Val {
        let str_val = string_init(value, false);
        Val::new(PhpValue::String(Box::new(str_val)), PhpType::String)
    }

    fn null_val() -> Val {
        Val::new(PhpValue::Long(0), PhpType::Null)
    }

    fn array_val() -> Val {
        Val::new(
            PhpValue::Array(Box::new(crate::engine::types::PhpArray::new())),
            PhpType::Array,
        )
    }

    fn zero_val() -> Val {
        Val::new(PhpValue::Long(0), PhpType::Long)
    }

    fn result_val(php_type: PhpType) -> Val {
        Val::new(PhpValue::Long(0), php_type)
    }

    fn clone_val(source: &Val) -> Val {
        match &source.value {
            PhpValue::Long(l) => Val::new(PhpValue::Long(*l), source.get_type()),
            PhpValue::Double(d) => Val::new(PhpValue::Double(*d), source.get_type()),
            PhpValue::String(s) => {
                let str_copy = string_init(s.as_str(), false);
                Val::new(PhpValue::String(Box::new(str_copy)), source.get_type())
            }
            PhpValue::Array(arr) => {
                // Deep copy of array
                let mut new_arr = crate::engine::types::PhpArray::new();
                for bucket in &arr.ar_data {
                    let cloned_val = Self::clone_val(&bucket.val);
                    let cloned_key = bucket.key.as_ref().map(|k| Box::new(string_init(k.as_str(), false)));
                    new_arr.ar_data.push(crate::engine::types::Bucket {
                        val: cloned_val,
                        h: bucket.h,
                        key: cloned_key,
                    });
                    new_arr.n_num_used += 1;
                    new_arr.n_num_of_elements += 1;
                }
                Val::new(PhpValue::Array(Box::new(new_arr)), source.get_type())
            }
            PhpValue::Object(obj) => {
                let mut new_obj = crate::engine::types::PhpObject::new(&obj.class_name);
                for (k, v) in &obj.properties {
                    new_obj.properties.insert(k.clone(), Self::clone_val(v));
                }
                Val::new(PhpValue::Object(Box::new(new_obj)), source.get_type())
            }
            _ => Val::new(PhpValue::Long(0), source.get_type()),
        }
    }

    fn result_dup(source: &Val) -> Val {
        Self::clone_val(source)
    }

    fn string_val_copy(value: &str, php_type: PhpType) -> Val {
        let str_copy = string_init(value, false);
        Val::new(PhpValue::String(Box::new(str_copy)), php_type)
    }
}

/// Convenience functions using the standard factory
pub fn bool_val(value: bool) -> Val {
    StdValFactory::bool_val(value)
}

pub fn long_val(value: i64) -> Val {
    StdValFactory::long_val(value)
}

pub fn double_val(value: f64) -> Val {
    StdValFactory::double_val(value)
}

pub fn string_val(value: &str) -> Val {
    StdValFactory::string_val(value)
}

pub fn null_val() -> Val {
    StdValFactory::null_val()
}

pub fn array_val() -> Val {
    StdValFactory::array_val()
}

pub fn zero_val() -> Val {
    StdValFactory::zero_val()
}

pub fn result_val(php_type: PhpType) -> Val {
    StdValFactory::result_val(php_type)
}

pub fn string_val_copy(value: &str, php_type: PhpType) -> Val {
    StdValFactory::string_val_copy(value, php_type)
}

pub fn clone_val(source: &Val) -> Val {
    StdValFactory::clone_val(source)
}

pub fn result_dup(source: &Val) -> Val {
    StdValFactory::result_dup(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_val() {
        let zval = bool_val(true);
        assert_eq!(zval.get_type(), PhpType::True);
        match zval.value {
            PhpValue::Long(1) => {}
            _ => panic!("Expected Long(1) for true"),
        }

        let zval = bool_val(false);
        assert_eq!(zval.get_type(), PhpType::False);
        match zval.value {
            PhpValue::Long(0) => {}
            _ => panic!("Expected Long(0) for false"),
        }
    }

    #[test]
    fn test_long_val() {
        let zval = long_val(42);
        assert_eq!(zval.get_type(), PhpType::Long);
        match zval.value {
            PhpValue::Long(42) => {}
            _ => panic!("Expected Long(42)"),
        }
    }

    #[test]
    fn test_double_val() {
        let zval = double_val(3.14);
        assert_eq!(zval.get_type(), PhpType::Double);
        match zval.value {
            PhpValue::Double(d) => assert!((d - 3.14).abs() < f64::EPSILON),
            _ => panic!("Expected Double"),
        }
    }

    #[test]
    fn test_string_val() {
        let zval = string_val("hello");
        assert_eq!(zval.get_type(), PhpType::String);
        match &zval.value {
            PhpValue::String(s) => assert_eq!(s.as_str(), "hello"),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_clone_val() {
        let original = long_val(123);
        let cloned = StdValFactory::clone_val(&original);
        assert_eq!(original.get_type(), cloned.get_type());
        match cloned.value {
            PhpValue::Long(123) => {}
            _ => panic!("Expected cloned value to be Long(123)"),
        }
    }
}
