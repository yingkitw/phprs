//! Math functions
//!
//! PHP math functions implementation

use crate::engine::types::{PhpType, PhpValue, Val};
use crate::engine::operators::{zval_get_long, zval_get_double};
use std::f64::consts::{E, PI};

pub fn math_abs(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("abs() expects at least 1 parameter".to_string());
    }
    
    match &args[0].value {
        PhpValue::Long(i) => Ok(Val::new(PhpValue::Long(i.abs()), PhpType::Long)),
        PhpValue::Double(f) => Ok(Val::new(PhpValue::Double(f.abs()), PhpType::Double)),
        _ => {
            let num = zval_get_double(&args[0]);
            Ok(Val::new(PhpValue::Double(num.abs()), PhpType::Double))
        }
    }
}

pub fn math_ceil(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("ceil() expects at least 1 parameter".to_string());
    }
    Ok(Val::new(PhpValue::Double(zval_get_double(&args[0]).ceil()), PhpType::Double))
}

pub fn math_floor(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("floor() expects at least 1 parameter".to_string());
    }
    Ok(Val::new(PhpValue::Double(zval_get_double(&args[0]).floor()), PhpType::Double))
}

pub fn math_round(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("round() expects at least 1 parameter".to_string());
    }
    
    let value = zval_get_double(&args[0]);
    let precision = if args.len() > 1 {
        zval_get_long(&args[1]) as i32
    } else {
        0
    };
    
    if precision == 0 {
        Ok(Val::new(PhpValue::Double(value.round()), PhpType::Double))
    } else {
        let multiplier = 10_f64.powi(precision);
        Ok(Val::new(PhpValue::Double((value * multiplier).round() / multiplier), PhpType::Double))
    }
}

pub fn math_sqrt(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("sqrt() expects at least 1 parameter".to_string());
    }
    let value = zval_get_double(&args[0]);
    if value < 0.0 {
        return Ok(Val::new(PhpValue::Double(f64::NAN), PhpType::Double));
    }
    Ok(Val::new(PhpValue::Double(value.sqrt()), PhpType::Double))
}

pub fn math_pow(args: &[Val]) -> Result<Val, String> {
    if args.len() < 2 {
        return Err("pow() expects exactly 2 parameters".to_string());
    }
    let base = zval_get_double(&args[0]);
    let exp = zval_get_double(&args[1]);
    Ok(Val::new(PhpValue::Double(base.powf(exp)), PhpType::Double))
}

pub fn math_exp(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("exp() expects at least 1 parameter".to_string());
    }
    Ok(Val::new(PhpValue::Double(E.powf(zval_get_double(&args[0]))), PhpType::Double))
}

pub fn math_log(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("log() expects at least 1 parameter".to_string());
    }
    
    let value = zval_get_double(&args[0]);
    if value <= 0.0 {
        return Ok(Val::new(PhpValue::Double(f64::NAN), PhpType::Double));
    }
    
    let result = if args.len() > 1 {
        let base = zval_get_double(&args[1]);
        if base <= 0.0 || base == 1.0 {
            return Ok(Val::new(PhpValue::Double(f64::NAN), PhpType::Double));
        }
        value.log(base)
    } else {
        value.ln()
    };
    
    Ok(Val::new(PhpValue::Double(result), PhpType::Double))
}

pub fn math_log10(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("log10() expects at least 1 parameter".to_string());
    }
    let value = zval_get_double(&args[0]);
    if value <= 0.0 {
        return Ok(Val::new(PhpValue::Double(f64::NAN), PhpType::Double));
    }
    Ok(Val::new(PhpValue::Double(value.log10()), PhpType::Double))
}

pub fn math_sin(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("sin() expects at least 1 parameter".to_string());
    }
    Ok(Val::new(PhpValue::Double(zval_get_double(&args[0]).sin()), PhpType::Double))
}

pub fn math_cos(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("cos() expects at least 1 parameter".to_string());
    }
    Ok(Val::new(PhpValue::Double(zval_get_double(&args[0]).cos()), PhpType::Double))
}

pub fn math_tan(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("tan() expects at least 1 parameter".to_string());
    }
    Ok(Val::new(PhpValue::Double(zval_get_double(&args[0]).tan()), PhpType::Double))
}

pub fn math_asin(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("asin() expects at least 1 parameter".to_string());
    }
    Ok(Val::new(PhpValue::Double(zval_get_double(&args[0]).asin()), PhpType::Double))
}

pub fn math_acos(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("acos() expects at least 1 parameter".to_string());
    }
    Ok(Val::new(PhpValue::Double(zval_get_double(&args[0]).acos()), PhpType::Double))
}

pub fn math_atan(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("atan() expects at least 1 parameter".to_string());
    }
    Ok(Val::new(PhpValue::Double(zval_get_double(&args[0]).atan()), PhpType::Double))
}

pub fn math_atan2(args: &[Val]) -> Result<Val, String> {
    if args.len() < 2 {
        return Err("atan2() expects exactly 2 parameters".to_string());
    }
    let y = zval_get_double(&args[0]);
    let x = zval_get_double(&args[1]);
    Ok(Val::new(PhpValue::Double(y.atan2(x)), PhpType::Double))
}

pub fn math_pi(_args: &[Val]) -> Result<Val, String> {
    Ok(Val::new(PhpValue::Double(PI), PhpType::Double))
}

pub fn math_max(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("max() expects at least 1 parameter".to_string());
    }
    
    let mut max_val = args[0].clone();
    for arg in &args[1..] {
        if zval_get_double(arg) > zval_get_double(&max_val) {
            max_val = arg.clone();
        }
    }
    Ok(max_val)
}

pub fn math_min(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("min() expects at least 1 parameter".to_string());
    }
    
    let mut min_val = args[0].clone();
    for arg in &args[1..] {
        if zval_get_double(arg) < zval_get_double(&min_val) {
            min_val = arg.clone();
        }
    }
    Ok(min_val)
}

pub fn math_rand(args: &[Val]) -> Result<Val, String> {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    let random = (seed
        .wrapping_mul(1103515245)
        .wrapping_add(12345)
        / 65536)
        % 32768;
    
    if args.len() >= 2 {
        let min = zval_get_long(&args[0]);
        let max = zval_get_long(&args[1]);
        let range = (max - min + 1) as u64;
        Ok(Val::new(PhpValue::Long(min + (random % range) as i64), PhpType::Long))
    } else {
        Ok(Val::new(PhpValue::Long(random as i64), PhpType::Long))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abs() {
        assert_eq!(zval_get_long(&math_abs(&[Val::new(PhpValue::Long(-5), PhpType::Long)]).unwrap()), 5);
        assert_eq!(zval_get_long(&math_abs(&[Val::new(PhpValue::Long(5), PhpType::Long)]).unwrap()), 5);
        assert!((zval_get_double(&math_abs(&[Val::new(PhpValue::Double(-3.14), PhpType::Double)]).unwrap()) - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_ceil_floor() {
        assert_eq!(zval_get_double(&math_ceil(&[Val::new(PhpValue::Double(3.14), PhpType::Double)]).unwrap()), 4.0);
        assert_eq!(zval_get_double(&math_floor(&[Val::new(PhpValue::Double(3.14), PhpType::Double)]).unwrap()), 3.0);
    }

    #[test]
    fn test_round() {
        assert_eq!(zval_get_double(&math_round(&[Val::new(PhpValue::Double(3.14), PhpType::Double)]).unwrap()), 3.0);
        assert_eq!(zval_get_double(&math_round(&[Val::new(PhpValue::Double(3.56), PhpType::Double)]).unwrap()), 4.0);
    }

    #[test]
    fn test_sqrt() {
        assert_eq!(zval_get_double(&math_sqrt(&[Val::new(PhpValue::Long(9), PhpType::Long)]).unwrap()), 3.0);
        assert_eq!(zval_get_double(&math_sqrt(&[Val::new(PhpValue::Long(16), PhpType::Long)]).unwrap()), 4.0);
    }

    #[test]
    fn test_pow() {
        assert_eq!(zval_get_double(&math_pow(&[Val::new(PhpValue::Long(2), PhpType::Long), Val::new(PhpValue::Long(3), PhpType::Long)]).unwrap()), 8.0);
        assert_eq!(zval_get_double(&math_pow(&[Val::new(PhpValue::Long(10), PhpType::Long), Val::new(PhpValue::Long(2), PhpType::Long)]).unwrap()), 100.0);
    }

    #[test]
    fn test_trig() {
        let pi = std::f64::consts::PI;
        assert!((zval_get_double(&math_sin(&[Val::new(PhpValue::Double(pi / 2.0), PhpType::Double)]).unwrap()) - 1.0).abs() < 0.001);
        assert!((zval_get_double(&math_cos(&[Val::new(PhpValue::Double(0.0), PhpType::Double)]).unwrap()) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_min_max() {
        assert_eq!(zval_get_long(&math_max(&[Val::new(PhpValue::Long(1), PhpType::Long), Val::new(PhpValue::Long(5), PhpType::Long), Val::new(PhpValue::Long(3), PhpType::Long)]).unwrap()), 5);
        assert_eq!(zval_get_long(&math_min(&[Val::new(PhpValue::Long(1), PhpType::Long), Val::new(PhpValue::Long(5), PhpType::Long), Val::new(PhpValue::Long(3), PhpType::Long)]).unwrap()), 1);
    }

    #[test]
    fn test_exp_log_log10() {
        assert!((zval_get_double(&math_exp(&[Val::new(PhpValue::Double(1.0), PhpType::Double)]).unwrap()) - std::f64::consts::E).abs() < 1e-9);
        assert!((zval_get_double(&math_log(&[Val::new(PhpValue::Double(std::f64::consts::E), PhpType::Double)]).unwrap()) - 1.0).abs() < 1e-9);
        assert!((zval_get_double(&math_log10(&[Val::new(PhpValue::Double(100.0), PhpType::Double)]).unwrap()) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_tan_atan2_atan_asin_acos() {
        assert!((zval_get_double(&math_tan(&[Val::new(PhpValue::Double(0.0), PhpType::Double)]).unwrap()) - 0.0).abs() < 1e-9);
        assert!((zval_get_double(&math_atan2(&[Val::new(PhpValue::Double(1.0), PhpType::Double), Val::new(PhpValue::Double(1.0), PhpType::Double)]).unwrap()) - std::f64::consts::FRAC_PI_4).abs() < 1e-9);
        assert!((zval_get_double(&math_atan(&[Val::new(PhpValue::Double(1.0), PhpType::Double)]).unwrap()) - std::f64::consts::FRAC_PI_4).abs() < 1e-9);
        assert!((zval_get_double(&math_asin(&[Val::new(PhpValue::Double(1.0), PhpType::Double)]).unwrap()) - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert!((zval_get_double(&math_acos(&[Val::new(PhpValue::Double(1.0), PhpType::Double)]).unwrap()) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_pi_and_rand() {
        assert!((zval_get_double(&math_pi(&[]).unwrap()) - std::f64::consts::PI).abs() < 1e-9);
        let r = math_rand(&[Val::new(PhpValue::Long(5), PhpType::Long), Val::new(PhpValue::Long(5), PhpType::Long)]).unwrap();
        assert_eq!(zval_get_long(&r), 5);
    }
}
