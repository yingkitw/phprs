//! Operators
//!
//! Operators
//!
//! This module implements PHP's type conversion and operator functions

use crate::engine::string::string_init;
use crate::engine::types::{PhpString, PhpType, PhpValue, Val};

/// Convert zval to long (integer)
pub fn zval_get_long(zval: &Val) -> i64 {
    match &zval.value {
        PhpValue::Long(l) => *l,
        PhpValue::Double(d) => *d as i64,
        PhpValue::String(s) => {
            // Try to parse string as integer
            s.as_str().parse().unwrap_or(0)
        }
        PhpValue::Array(_) => 1,  // Arrays are truthy
        PhpValue::Object(_) => 1, // Objects are truthy
        _ => 0,
    }
}

/// Convert zval to double (float)
pub fn zval_get_double(zval: &Val) -> f64 {
    match &zval.value {
        PhpValue::Long(l) => *l as f64,
        PhpValue::Double(d) => *d,
        PhpValue::String(s) => {
            // Try to parse string as float
            s.as_str().parse().unwrap_or(0.0)
        }
        _ => 0.0,
    }
}

/// Convert zval to string
pub fn zval_get_string(zval: &Val) -> PhpString {
    match &zval.value {
        PhpValue::String(s) => {
            // Create a new PhpString from the existing one
            string_init(s.as_str(), false)
        }
        PhpValue::Long(l) => string_init(&l.to_string(), false),
        PhpValue::Double(d) => string_init(&d.to_string(), false),
        PhpValue::Array(_) => string_init("Array", false),
        PhpValue::Object(_) => string_init("Object", false),
        _ => {
            // Check if it's null type
            if zval.get_type() == PhpType::Null {
                string_init("", false)
            } else {
                string_init("", false)
            }
        }
    }
}

/// Compare two zvals
pub fn zval_compare(z1: &Val, z2: &Val) -> i32 {
    match (&z1.value, &z2.value) {
        (PhpValue::Long(l1), PhpValue::Long(l2)) => {
            if l1 < l2 {
                -1
            } else if l1 > l2 {
                1
            } else {
                0
            }
        }
        (PhpValue::Double(d1), PhpValue::Double(d2)) => {
            if d1 < d2 {
                -1
            } else if d1 > d2 {
                1
            } else {
                0
            }
        }
        (PhpValue::String(s1), PhpValue::String(s2)) => s1.as_str().cmp(s2.as_str()) as i32,
        _ => {
            // Type juggling: convert to common type and compare
            let d1 = zval_get_double(z1);
            let d2 = zval_get_double(z2);
            if d1 < d2 {
                -1
            } else if d1 > d2 {
                1
            } else {
                0
            }
        }
    }
}

/// Check if two zvals are equal
pub fn zval_is_equal(z1: &Val, z2: &Val) -> bool {
    zval_compare(z1, z2) == 0
}

/// Add two zvals
pub fn zval_add(z1: &Val, z2: &Val) -> Val {
    match (&z1.value, &z2.value) {
        (PhpValue::Long(l1), PhpValue::Long(l2)) => {
            Val::new(PhpValue::Long(l1 + l2), PhpType::Long)
        }
        (PhpValue::Double(d1), PhpValue::Double(d2)) => {
            Val::new(PhpValue::Double(d1 + d2), PhpType::Double)
        }
        (PhpValue::String(s1), PhpValue::String(s2)) => {
            let result = format!("{}{}", s1.as_str(), s2.as_str());
            let str_val = string_init(&result, false);
            Val::new(PhpValue::String(Box::new(str_val)), PhpType::String)
        }
        _ => {
            // Type juggling: convert to numbers and add
            let d1 = zval_get_double(z1);
            let d2 = zval_get_double(z2);
            Val::new(PhpValue::Double(d1 + d2), PhpType::Double)
        }
    }
}

/// Subtract two zvals
pub fn zval_sub(z1: &Val, z2: &Val) -> Val {
    let d1 = zval_get_double(z1);
    let d2 = zval_get_double(z2);
    Val::new(PhpValue::Double(d1 - d2), PhpType::Double)
}

/// Multiply two zvals
pub fn zval_mul(z1: &Val, z2: &Val) -> Val {
    let d1 = zval_get_double(z1);
    let d2 = zval_get_double(z2);
    Val::new(PhpValue::Double(d1 * d2), PhpType::Double)
}

/// Divide two zvals
pub fn zval_div(z1: &Val, z2: &Val) -> Result<Val, String> {
    let d2 = zval_get_double(z2);
    if d2 == 0.0 {
        return Err("Division by zero".to_string());
    }
    let d1 = zval_get_double(z1);
    Ok(Val::new(PhpValue::Double(d1 / d2), PhpType::Double))
}

/// Convert zval to boolean
pub fn zval_get_bool(zval: &Val) -> bool {
    match &zval.value {
        PhpValue::Long(l) => *l != 0,
        PhpValue::Double(d) => *d != 0.0,
        PhpValue::String(s) => !s.as_str().is_empty() && s.as_str() != "0",
        PhpValue::Array(a) => !a.ar_data.is_empty(),
        PhpValue::Object(_) => true,
        _ => {
            // Check if it's null type
            zval.get_type() != PhpType::Null
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::string::string_init;

    #[test]
    fn test_zval_get_long() {
        let z = Val::new(PhpValue::Long(42), PhpType::Long);
        assert_eq!(zval_get_long(&z), 42);

        let z = Val::new(PhpValue::Double(3.14), PhpType::Double);
        assert_eq!(zval_get_long(&z), 3);
    }

    #[test]
    fn test_zval_get_double() {
        let z = Val::new(PhpValue::Long(42), PhpType::Long);
        assert_eq!(zval_get_double(&z), 42.0);
    }

    #[test]
    fn test_zval_add() {
        let z1 = Val::new(PhpValue::Long(10), PhpType::Long);
        let z2 = Val::new(PhpValue::Long(20), PhpType::Long);
        let result = zval_add(&z1, &z2);
        assert_eq!(zval_get_long(&result), 30);
    }

    #[test]
    fn test_zval_compare() {
        let z1 = Val::new(PhpValue::Long(10), PhpType::Long);
        let z2 = Val::new(PhpValue::Long(20), PhpType::Long);
        assert!(zval_compare(&z1, &z2) < 0);
        assert!(zval_compare(&z2, &z1) > 0);
        assert_eq!(zval_compare(&z1, &z1), 0);
    }

    #[test]
    fn test_zval_get_bool() {
        let z = Val::new(PhpValue::Long(1), PhpType::Long);
        assert!(zval_get_bool(&z));

        let z = Val::new(PhpValue::Long(0), PhpType::Long);
        assert!(!zval_get_bool(&z));
    }
}
