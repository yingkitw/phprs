//! PHP `serialize()` / `unserialize()`.
//!
//! Implements the PHP serialization format for scalar values, arrays, and
//! plain objects (serialized via their `properties` map). Magic method hooks
//! (`__serialize` / `__unserialize`) require method invocation from builtins,
//! which is tracked as follow-up work; this module handles the data format.

use crate::engine::hash::hash_add_or_update;
use crate::engine::operators::{zval_get_double, zval_get_long, zval_get_string};
use crate::engine::string::string_init;
use crate::engine::types::{PhpArray, PhpObject, PhpType, PhpValue, Val};

fn long_val(n: i64) -> Val {
    Val::new(PhpValue::Long(n), PhpType::Long)
}

fn str_val(s: &str) -> Val {
    Val::new(
        PhpValue::String(Box::new(string_init(s, false))),
        PhpType::String,
    )
}

/// Recursively serialize a Val into the PHP serialization format.
fn serialize_into(out: &mut String, val: &Val) {
    // In this engine booleans/null are represented as Long(0/1) with a type tag,
    // so dispatch on the runtime type for those before matching the value variant.
    let t = val.get_type();
    if t == PhpType::Null {
        out.push_str("N;");
        return;
    }
    if t == PhpType::True || t == PhpType::False || t == PhpType::Bool {
        out.push_str(&format!("b:{};", if t == PhpType::True { 1 } else { 0 }));
        return;
    }
    match &val.value {
        PhpValue::Long(n) => {
            out.push_str(&format!("i:{n};"));
        }
        PhpValue::Double(d) => {
            // PHP's default serialize_precision (-1) emits the shortest round-trip repr.
            out.push_str(&format!("d:{d};"));
        }
        PhpValue::String(s) => {
            let bytes = s.as_bytes();
            out.push_str(&format!("s:{}:\"", bytes.len()));
            out.push_str(s.as_str());
            out.push_str("\";");
        }
        PhpValue::Array(arr) => {
            // PHP treats arrays that look like 0..n sequences as packed, but the
            // serialized form is identical: a:count:{ key val key val ... };
            out.push_str(&format!("a:{}:{{", arr.ar_data.len()));
            for bucket in &arr.ar_data {
                if let Some(k) = &bucket.key {
                    let kb = k.as_bytes();
                    out.push_str(&format!("s:{}:\"", kb.len()));
                    out.push_str(k.as_str());
                    out.push_str("\";");
                } else {
                    out.push_str(&format!("i:{};", bucket.h));
                }
                serialize_into(out, &bucket.val);
            }
            out.push('}');
        }
        PhpValue::Object(obj) => {
            let name = obj.class_name.as_str();
            out.push_str(&format!("O:{}:\"{}\":{}:{{", name.len(), name, obj.properties.len()));
            for (pname, pval) in &obj.properties {
                let pb = pname.as_bytes();
                out.push_str(&format!("s:{}:\"", pb.len()));
                out.push_str(pname);
                out.push_str("\";");
                serialize_into(out, pval);
            }
            out.push('}');
        }
        // Everything else (resources, etc.) serializes as null.
        _ => out.push_str("N;"),
    }
}

/// `serialize($value): string`
pub fn php_serialize(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Ok(str_val(""));
    }
    let mut out = String::new();
    serialize_into(&mut out, &args[0]);
    Ok(str_val(&out))
}

/// Cursor over the serialized byte buffer used during unserialization.
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            data: s.as_bytes(),
            pos: 0,
        }
    }
    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }
    fn expect(&mut self, byte: u8) -> Result<(), String> {
        if self.peek() == Some(byte) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!(
                "unserialize(): expected '{}' at offset {}",
                byte as char, self.pos
            ))
        }
    }
    fn read_until(&mut self, byte: u8) -> Result<&'a [u8], String> {
        let start = self.pos;
        while self.pos < self.data.len() && self.data[self.pos] != byte {
            self.pos += 1;
        }
        if self.pos >= self.data.len() {
            return Err("unserialize(): unexpected end of data".to_string());
        }
        let slice = &self.data[start..self.pos];
        self.pos += 1; // consume delimiter
        Ok(slice)
    }
    fn read_fixed(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.pos + n > self.data.len() {
            return Err("unserialize(): unexpected end of data".to_string());
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }
}

fn parse_value(cur: &mut Cursor) -> Result<Val, String> {
    let type_byte = cur
        .peek()
        .ok_or_else(|| "unserialize(): empty data".to_string())?;
    let type_char = char::from(type_byte);
    cur.pos += 1; // consume type char
    match type_char {
        'N' => {
            cur.expect(b';')?;
            Ok(Val::new(PhpValue::Long(0), PhpType::Null))
        }
        'b' => {
            cur.expect(b':')?;
            let raw = cur.read_until(b';')?;
            let b = std::str::from_utf8(raw).map_err(|_| "unserialize(): invalid UTF-8".to_string())?
                .trim()
                .parse::<u8>()
                .map_err(|_| "unserialize(): invalid bool".to_string())?;
            Ok(Val::new(
                PhpValue::Long(if b != 0 { 1 } else { 0 }),
                if b != 0 { PhpType::True } else { PhpType::False },
            ))
        }
        'i' => {
            cur.expect(b':')?;
            let raw = cur.read_until(b';')?;
            let n: i64 = std::str::from_utf8(raw).map_err(|_| "unserialize(): invalid UTF-8".to_string())?
                .trim()
                .parse()
                .map_err(|_| "unserialize(): invalid integer".to_string())?;
            Ok(long_val(n))
        }
        'd' => {
            cur.expect(b':')?;
            let raw = cur.read_until(b';')?;
            let d: f64 = std::str::from_utf8(raw).map_err(|_| "unserialize(): invalid UTF-8".to_string())?
                .trim()
                .parse()
                .map_err(|_| "unserialize(): invalid double".to_string())?;
            Ok(Val::new(PhpValue::Double(d), PhpType::Double))
        }
        's' => {
            cur.expect(b':')?;
            let len_raw = cur.read_until(b':')?;
            let len: usize = std::str::from_utf8(len_raw).map_err(|_| "unserialize(): invalid UTF-8".to_string())?
                .trim()
                .parse()
                .map_err(|_| "unserialize(): invalid string length".to_string())?;
            cur.expect(b'"')?;
            let bytes = cur.read_fixed(len)?;
            let s = std::str::from_utf8(bytes).unwrap_or("").to_string();
            cur.expect(b'"')?;
            cur.expect(b';')?;
            Ok(str_val(&s))
        }
        'a' => {
            cur.expect(b':')?;
            let count_raw = cur.read_until(b':')?;
            let count: usize = std::str::from_utf8(count_raw).map_err(|_| "unserialize(): invalid UTF-8".to_string())?
                .trim()
                .parse()
                .map_err(|_| "unserialize(): invalid array count".to_string())?;
            cur.expect(b'{')?;
            let mut arr = PhpArray::new();
            let mut next_idx: u64 = 0;
            for _ in 0..count {
                let key_val = parse_value(cur)?;
                let val = parse_value(cur)?;
                match &key_val.value {
                    PhpValue::Long(i) => {
                        let h = *i as u64;
                        let _ = hash_add_or_update(&mut arr, None, h, val, 0);
                        if (h as i64) >= 0 && (h as i64) >= next_idx as i64 {
                            next_idx = h + 1;
                        }
                    }
                    PhpValue::String(s) => {
                        let key = string_init(s.as_str(), false);
                        let _ = hash_add_or_update(&mut arr, Some(&key), 0, val, 0);
                    }
                    _ => {
                        let _ = hash_add_or_update(&mut arr, None, next_idx, val, 0);
                        next_idx += 1;
                    }
                }
            }
            arr.n_next_free_element = next_idx as i64;
            cur.expect(b'}')?;
            Ok(Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array))
        }
        'O' => {
            cur.expect(b':')?;
            let name_len_raw = cur.read_until(b':')?;
            let name_len: usize = std::str::from_utf8(name_len_raw).map_err(|_| "unserialize(): invalid UTF-8".to_string())?
                .trim()
                .parse()
                .map_err(|_| "unserialize(): invalid class name length".to_string())?;
            cur.expect(b'"')?;
            let name_bytes = cur.read_fixed(name_len)?;
            let class_name = std::str::from_utf8(name_bytes).unwrap_or("stdClass").to_string();
            cur.expect(b'"')?;
            cur.expect(b':')?;
            let prop_count_raw = cur.read_until(b':')?;
            let prop_count: usize = std::str::from_utf8(prop_count_raw).map_err(|_| "unserialize(): invalid UTF-8".to_string())?
                .trim()
                .parse()
                .map_err(|_| "unserialize(): invalid property count".to_string())?;
            cur.expect(b'{')?;
            let mut obj = PhpObject::new(&class_name);
            for _ in 0..prop_count {
                let pname_val = parse_value(cur)?;
                let pval = parse_value(cur)?;
                if let PhpValue::String(s) = &pname_val.value {
                    obj.properties.insert(s.as_str().to_string(), pval);
                }
            }
            cur.expect(b'}')?;
            Ok(Val::new(PhpValue::Object(Box::new(obj)), PhpType::Object))
        }
        other => Err(format!("unserialize(): unknown type '{other}'")),
    }
}

/// `unserialize($string): mixed` — returns the value, or `false` on parse failure.
pub fn php_unserialize(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Ok(Val::new(PhpValue::Long(0), PhpType::False));
    }
    let s = zval_get_string(&args[0]);
    let trimmed = s.as_str().trim_start();
    if trimmed.is_empty() {
        return Ok(Val::new(PhpValue::Long(0), PhpType::False));
    }
    let mut cur = Cursor::new(trimmed);
    match parse_value(&mut cur) {
        Ok(v) => Ok(v),
        Err(_) => Ok(Val::new(PhpValue::Long(0), PhpType::False)),
    }
}

/// Convenience helpers re-exported for builtins that want long/double coercion.
pub fn _coerce_long(v: &Val) -> i64 {
    zval_get_long(v)
}
pub fn _coerce_double(v: &Val) -> f64 {
    zval_get_double(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_long(n: i64) -> Val {
        Val::new(PhpValue::Long(n), PhpType::Long)
    }
    fn mk_double(n: f64) -> Val {
        Val::new(PhpValue::Double(n), PhpType::Double)
    }
    fn mk_str(s: &str) -> Val {
        str_val(s)
    }
    fn mk_null() -> Val {
        Val::new(PhpValue::Long(0), PhpType::Null)
    }
    fn mk_true() -> Val {
        Val::new(PhpValue::Long(1), PhpType::True)
    }

    fn array_of(pairs: &[(Option<Val>, Val)]) -> Val {
        let mut arr = PhpArray::new();
        let mut idx: u64 = 0;
        for (k, v) in pairs {
            match k {
                Some(kv) => {
                    if let PhpValue::String(s) = &kv.value {
                        let key = string_init(s.as_str(), false);
                        let _ = hash_add_or_update(&mut arr, Some(&key), 0, v.clone(), 0);
                    } else {
                        let _ = hash_add_or_update(
                            &mut arr,
                            None,
                            zval_get_long(kv) as u64,
                            v.clone(),
                            0,
                        );
                    }
                }
                None => {
                    let _ = hash_add_or_update(&mut arr, None, idx, v.clone(), 0);
                    idx += 1;
                }
            }
        }
        Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array)
    }

    #[test]
    fn round_trip_scalars_and_null() {
        let cases: &[(&str, Val)] = &[
            ("N;", mk_null()),
            ("b:1;", mk_true()),
            ("i:42;", mk_long(42)),
            ("i:-7;", mk_long(-7)),
            ("s:5:\"hello\";", mk_str("hello")),
        ];
        for (expected, val) in cases {
            let s = php_serialize(&[val.clone()]).unwrap();
            assert_eq!(zval_get_string(&s).as_str(), *expected);
            let back = php_unserialize(&[s]).unwrap();
            assert_eq!(zval_get_string(&back).as_str(), zval_get_string(val).as_str());
        }
    }

    #[test]
    fn round_trip_double() {
        let s = php_serialize(&[mk_double(3.5)]).unwrap();
        assert_eq!(zval_get_string(&s).as_str(), "d:3.5;");
        let back = php_unserialize(&[s]).unwrap();
        assert!((zval_get_double(&back) - 3.5).abs() < 1e-12);
    }

    #[test]
    fn round_trip_indexed_array() {
        let arr = array_of(&[
            (None, mk_long(1)),
            (None, mk_long(2)),
            (None, mk_str("x")),
        ]);
        let s = php_serialize(&[arr]).unwrap();
        let expected = "a:3:{i:0;i:1;i:1;i:2;i:2;s:1:\"x\";}";
        assert_eq!(zval_get_string(&s).as_str(), expected);
        let back = php_unserialize(&[s]).unwrap();
        if let PhpValue::Array(a) = &back.value {
            assert_eq!(a.ar_data.len(), 3);
            assert_eq!(zval_get_long(&a.ar_data[2].val), 0); // "x" -> string, zval_get_long=0
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn round_trip_assoc_array() {
        let arr = array_of(&[(Some(mk_str("a")), mk_long(1)), (Some(mk_str("b")), mk_str("y"))]);
        let s = php_serialize(&[arr]).unwrap();
        let back = php_unserialize(&[s]).unwrap();
        if let PhpValue::Array(a) = &back.value {
            assert_eq!(a.ar_data.len(), 2);
            assert_eq!(a.ar_data[0].key.as_ref().unwrap().as_str(), "a");
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn round_trip_object() {
        let mut obj = PhpObject::new("Point");
        obj.properties.insert("x".to_string(), mk_long(1));
        obj.properties.insert("y".to_string(), mk_long(2));
        let v = Val::new(PhpValue::Object(Box::new(obj)), PhpType::Object);
        let s = php_serialize(&[v]).unwrap();
        assert!(zval_get_string(&s).as_str().starts_with("O:5:\"Point\":2:{"));
        let back = php_unserialize(&[s]).unwrap();
        if let PhpValue::Object(o) = &back.value {
            assert_eq!(o.class_name, "Point");
            assert_eq!(zval_get_long(o.properties.get("x").unwrap()), 1);
        } else {
            panic!("expected object");
        }
    }

    #[test]
    fn unserialize_invalid_returns_false() {
        let r = php_unserialize(&[mk_str("not valid serialized data")]).unwrap();
        assert_eq!(r.get_type(), PhpType::False);
    }
}
