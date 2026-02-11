//! Basic Types Example
//!
//! Demonstrates creating and manipulating basic PHP types

use phprs::operators::{zval_get_bool, zval_get_double, zval_get_long, zval_get_string};
use phprs::string::string_init;
use phprs::{PhpType, PhpValue, Val};

fn main() {
    println!("=== phprs Basic Types Example ===\n");

    // Create a long (integer) value
    let long_val = Val::new(PhpValue::Long(42), PhpType::Long);
    println!("Long value: {}", zval_get_long(&long_val));

    // Create a double (float) value
    let double_val = Val::new(PhpValue::Double(3.14159), PhpType::Double);
    println!("Double value: {}", zval_get_double(&double_val));

    // Create a string value
    let str_val = string_init("Hello, phprs!", false);
    let val_str = Val::new(PhpValue::String(Box::new(str_val)), PhpType::String);
    println!("String value: {}", zval_get_string(&val_str).as_str());

    // Create boolean values
    let true_val = Val::new(PhpValue::Bool(true), PhpType::Bool);
    let false_val = Val::new(PhpValue::Bool(false), PhpType::Bool);
    println!("True value: {}", zval_get_bool(&true_val));
    println!("False value: {}", zval_get_bool(&false_val));

    // Create null value
    let null_val = Val::new(PhpValue::Long(0), PhpType::Null);
    println!("Null value type: {:?}", null_val.get_type());
}
