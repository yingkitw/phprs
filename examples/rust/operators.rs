//! Operators Example
//!
//! Demonstrates PHP operators and type conversions

use phprs::operators::{
    zval_add, zval_compare, zval_div, zval_get_double, zval_get_long, zval_mul, zval_sub,
};
use phprs::{PhpType, PhpValue, Val};

fn main() {
    println!("=== phprs Operators Example ===\n");

    // Create operands
    let a = Val::new(PhpValue::Long(10), PhpType::Long);
    let b = Val::new(PhpValue::Long(3), PhpType::Long);

    // Arithmetic operations
    let result_add = zval_add(&a, &b);
    println!(
        "{} + {} = {}",
        zval_get_long(&a),
        zval_get_long(&b),
        zval_get_long(&result_add)
    );

    let result_sub = zval_sub(&a, &b);
    println!(
        "{} - {} = {}",
        zval_get_long(&a),
        zval_get_long(&b),
        zval_get_long(&result_sub)
    );

    let result_mul = zval_mul(&a, &b);
    println!(
        "{} * {} = {}",
        zval_get_long(&a),
        zval_get_long(&b),
        zval_get_long(&result_mul)
    );

    match zval_div(&a, &b) {
        Ok(result_div) => println!(
            "{} / {} = {}",
            zval_get_long(&a),
            zval_get_long(&b),
            zval_get_long(&result_div)
        ),
        Err(e) => println!(
            "{} / {} = Error: {}",
            zval_get_long(&a),
            zval_get_long(&b),
            e
        ),
    }

    // Comparison operations
    println!("\nComparisons:");
    let cmp1 = zval_compare(&a, &b);
    println!(
        "  {} vs {}: {:?}",
        zval_get_long(&a),
        zval_get_long(&b),
        cmp1
    );

    let c = Val::new(PhpValue::Long(10), PhpType::Long);
    let cmp2 = zval_compare(&a, &c);
    println!(
        "  {} vs {}: {:?}",
        zval_get_long(&a),
        zval_get_long(&c),
        cmp2
    );

    // Type conversion
    let double_val = Val::new(PhpValue::Double(42.5), PhpType::Double);
    println!("\nType conversions:");
    println!(
        "  Double {} as long: {}",
        zval_get_double(&double_val),
        zval_get_long(&double_val)
    );
}
