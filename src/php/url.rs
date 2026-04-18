//! URL handling functions
//!
//! PHP URL functions: parse_url, http_build_query, urlencode, urldecode,
//! rawurlencode, rawurldecode, parse_str, urldecode

use crate::engine::operators::zval_get_string;
use crate::engine::string::string_init;
use crate::engine::types::{PhpType, PhpValue, Val};

fn string_val(s: &str) -> Val {
    Val::new(
        PhpValue::String(Box::new(string_init(s, false))),
        PhpType::String,
    )
}

fn make_array(pairs: Vec<(Option<String>, Val)>) -> Val {
    let mut arr = crate::engine::types::PhpArray::new();
    for (i, (key, val)) in pairs.into_iter().enumerate() {
        match key {
            Some(k) => {
                let ks = Box::new(string_init(&k, false));
                let _ = crate::engine::hash::hash_add_or_update(&mut arr, Some(&*ks), 0, val, 0);
            }
            None => {
                let _ = crate::engine::hash::hash_add_or_update(&mut arr, None, i as u64, val, 0);
            }
        }
    }
    Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array)
}

/// parse_url($url, $component = -1)
///
/// Parses a URL and returns its components.
/// If component is specified, returns that component as a string (or int for PHP_URL_PORT).
pub fn parse_url(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("parse_url() expects at least 1 argument".to_string());
    }

    let url = zval_get_string(&args[0]).as_str().to_string();
    let component = if args.len() > 1 {
        crate::engine::operators::zval_get_long(&args[1]) as i64
    } else {
        -1
    };

    let parsed = url::Url::parse(&url);

    let scheme = parsed
        .as_ref()
        .ok()
        .and_then(|u| Some(u.scheme().to_string()));
    let host = parsed
        .as_ref()
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()));
    let port = parsed.as_ref().ok().and_then(|u| u.port());
    let user = parsed
        .as_ref()
        .ok()
        .and_then(|u| u.username().to_string().into());
    let pass = parsed
        .as_ref()
        .ok()
        .and_then(|u| u.password().map(|p| p.to_string()));
    let path = parsed
        .as_ref()
        .ok()
        .map(|u| u.path().to_string())
        .unwrap_or_default();
    let query = parsed
        .as_ref()
        .ok()
        .and_then(|u| u.query().map(|q| q.to_string()));
    let fragment = parsed
        .as_ref()
        .ok()
        .and_then(|u| u.fragment().map(|f| f.to_string()));

    // PHP component constants: PHP_URL_SCHEME=1, HOST=2, PORT=3, USER=4, PASS=5, PATH=6, QUERY=7, FRAGMENT=8
    if component != -1 {
        let result = match component {
            1 => scheme.map(|s| string_val(&s)),
            2 => host.map(|h| string_val(&h)),
            3 => port.map(|p| Val::new(PhpValue::Long(p as i64), PhpType::Long)),
            4 => {
                let u = user.unwrap_or_default();
                if u.is_empty() {
                    None
                } else {
                    Some(string_val(&u))
                }
            }
            5 => pass.map(|p| string_val(&p)),
            6 => {
                let p = if path.is_empty() {
                    return Ok(Val::new(PhpValue::Long(0), PhpType::Null));
                } else {
                    path
                };
                Some(string_val(&p))
            }
            7 => query.map(|q| string_val(&q)),
            8 => fragment.map(|f| string_val(&f)),
            _ => return Ok(Val::new(PhpValue::Long(0), PhpType::Null)),
        };
        return Ok(result.unwrap_or_else(|| Val::new(PhpValue::Long(0), PhpType::Null)));
    }

    let mut pairs: Vec<(Option<String>, Val)> = Vec::new();

    if let Some(s) = scheme {
        pairs.push((Some("scheme".to_string()), string_val(&s)));
    }
    if let Some(h) = host {
        pairs.push((Some("host".to_string()), string_val(&h)));
    }
    if let Some(p) = port {
        pairs.push((
            Some("port".to_string()),
            Val::new(PhpValue::Long(p as i64), PhpType::Long),
        ));
    }
    let user_val = user.unwrap_or_default();
    if !user_val.is_empty() {
        pairs.push((Some("user".to_string()), string_val(&user_val)));
    }
    if let Some(p) = pass {
        pairs.push((Some("pass".to_string()), string_val(&p)));
    }
    if !path.is_empty() {
        pairs.push((Some("path".to_string()), string_val(&path)));
    }
    if let Some(q) = query {
        pairs.push((Some("query".to_string()), string_val(&q)));
    }
    if let Some(f) = fragment {
        pairs.push((Some("fragment".to_string()), string_val(&f)));
    }

    if parsed.is_err() {
        return Ok(Val::new(PhpValue::Long(0), PhpType::False));
    }

    Ok(make_array(pairs))
}

/// http_build_query($data, $numeric_prefix = "", $arg_separator = "&", $enc_type = PHP_QUERY_RFC1738)
pub fn http_build_query(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("http_build_query() expects at least 1 argument".to_string());
    }

    let prefix = if args.len() > 1 {
        zval_get_string(&args[1]).as_str().to_string()
    } else {
        String::new()
    };

    let separator = if args.len() > 2 {
        zval_get_string(&args[2]).as_str().to_string()
    } else {
        "&".to_string()
    };

    let pairs = build_query_pairs(&args[0], &prefix);
    Ok(string_val(&pairs.join(&separator)))
}

fn build_query_pairs(data: &Val, prefix: &str) -> Vec<String> {
    let mut result = Vec::new();

    if let PhpValue::Array(ref arr) = data.value {
        for bucket in &arr.ar_data {
            let key = if let Some(ref k) = bucket.key {
                k.as_str().to_string()
            } else {
                format!("{}", bucket.h)
            };
            let full_key = if prefix.is_empty() {
                key
            } else {
                format!("{}[{}]", prefix, key)
            };
            let val = zval_get_string(&bucket.val).as_str().to_string();
            if matches!(bucket.val.value, PhpValue::Array(_)) {
                result.extend(build_query_pairs(&bucket.val, &full_key));
            } else {
                result.push(format!(
                    "{}={}",
                    urlencode_str(&full_key),
                    urlencode_str(&val)
                ));
            }
        }
    } else {
        let val = zval_get_string(data).as_str().to_string();
        if prefix.is_empty() {
            result.push(urlencode_str(&val));
        } else {
            result.push(format!("{}={}", urlencode_str(prefix), urlencode_str(&val)));
        }
    }

    result
}

/// urlencode($str) - application/x-www-form-urlencoded encoding
pub fn urlencode(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("urlencode() expects 1 argument".to_string());
    }
    let s = zval_get_string(&args[0]).as_str().to_string();
    Ok(string_val(&urlencode_str(&s)))
}

fn urlencode_str(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            b' ' => result.push('+'),
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", byte));
            }
        }
    }
    result
}

/// urldecode($str) - decode application/x-www-form-urlencoded string
pub fn urldecode(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("urldecode() expects 1 argument".to_string());
    }
    let s = zval_get_string(&args[0]).as_str().to_string();
    Ok(string_val(&urldecode_str(&s)))
}

fn urldecode_str(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = &s[i + 1..i + 3];
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    result.push(byte as char);
                    i += 3;
                } else {
                    result.push(bytes[i] as char);
                    i += 1;
                }
            }
            b'+' => {
                result.push(' ');
                i += 1;
            }
            _ => {
                result.push(bytes[i] as char);
                i += 1;
            }
        }
    }
    result
}

/// rawurlencode($str) - RFC 3986 URL encoding
pub fn rawurlencode(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("rawurlencode() expects 1 argument".to_string());
    }
    let s = zval_get_string(&args[0]).as_str().to_string();
    let mut result = String::with_capacity(s.len() * 3);
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", byte));
            }
        }
    }
    Ok(string_val(&result))
}

/// rawurldecode($str) - decode RFC 3986 encoded string
pub fn rawurldecode(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("rawurldecode() expects 1 argument".to_string());
    }
    let s = zval_get_string(&args[0]).as_str().to_string();
    Ok(string_val(&urldecode_str(&s)))
}

/// parse_str($str, &$result) - Parses a query string into variables
pub fn parse_str(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("parse_str() expects at least 1 argument".to_string());
    }
    let s = zval_get_string(&args[0]).as_str().to_string();
    let mut arr = crate::engine::types::PhpArray::new();

    for pair in s.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let val = parts.next().unwrap_or("");
        let decoded_key = urldecode_str(key);
        let decoded_val = urldecode_str(val);
        let ks = Box::new(string_init(&decoded_key, false));
        let v = string_val(&decoded_val);
        let _ = crate::engine::hash::hash_add_or_update(&mut arr, Some(&*ks), 0, v, 0);
    }

    Ok(Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array))
}

/// get_headers($url) - Fetches all the headers sent by the server in response to an HTTP request
pub fn get_headers(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("get_headers() expects at least 1 argument".to_string());
    }

    let url = zval_get_string(&args[0]).as_str().to_string();

    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Ok(Val::new(PhpValue::Long(0), PhpType::False));
    }

    let mut headers_list: Vec<Val> = Vec::new();

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build();

    if let Ok(client) = client {
        let response = client.head(&url).send();
        if let Ok(resp) = response {
            headers_list.push(string_val(&format!(
                "HTTP/{} {}",
                match resp.version() {
                    reqwest::Version::HTTP_09 => "0.9",
                    reqwest::Version::HTTP_10 => "1.0",
                    reqwest::Version::HTTP_11 => "1.1",
                    reqwest::Version::HTTP_2 => "2",
                    reqwest::Version::HTTP_3 => "3",
                    _ => "1.1",
                },
                resp.status().as_u16()
            )));
            for (name, value) in resp.headers() {
                headers_list.push(string_val(&format!(
                    "{}: {}",
                    name,
                    value.to_str().unwrap_or("")
                )));
            }
        }
    }

    if headers_list.is_empty() {
        return Ok(Val::new(PhpValue::Long(0), PhpType::False));
    }

    let mut arr = crate::engine::types::PhpArray::new();
    let mut idx: u64 = 0;
    for val in headers_list.into_iter() {
        let _ = crate::engine::hash::hash_add_or_update(&mut arr, None, idx, val, 0);
        idx += 1;
    }
    Ok(Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencode() {
        let val = string_val("hello world");
        let result = urlencode(&[val]).unwrap();
        assert_eq!(zval_get_string(&result).as_str(), "hello+world");
    }

    #[test]
    fn test_urldecode() {
        let val = string_val("hello+world");
        let result = urldecode(&[val]).unwrap();
        assert_eq!(zval_get_string(&result).as_str(), "hello world");
    }

    #[test]
    fn test_rawurlencode() {
        let val = string_val("hello world");
        let result = rawurlencode(&[val]).unwrap();
        assert_eq!(zval_get_string(&result).as_str(), "hello%20world");
    }

    #[test]
    fn test_parse_url() {
        let val = string_val("https://example.com/path?query=1#frag");
        let result = parse_url(&[val]).unwrap();
        assert!(matches!(result.value, PhpValue::Array(_)));
    }

    #[test]
    fn test_parse_url_component() {
        let val = string_val("https://example.com/path");
        let result = parse_url(&[val, Val::new(PhpValue::Long(1), PhpType::Long)]).unwrap();
        assert_eq!(zval_get_string(&result).as_str(), "https");
    }

    #[test]
    fn test_parse_str() {
        let val = string_val("name=John&age=30");
        let result = parse_str(&[val]).unwrap();
        assert!(matches!(result.value, PhpValue::Array(_)));
    }

    #[test]
    fn test_http_build_query() {
        let mut arr = crate::engine::types::PhpArray::new();
        let ks = Box::new(string_init("name", false));
        let _ =
            crate::engine::hash::hash_add_or_update(&mut arr, Some(&*ks), 0, string_val("John"), 0);
        let ks2 = Box::new(string_init("age", false));
        let _ = crate::engine::hash::hash_add_or_update(
            &mut arr,
            Some(&*ks2),
            0,
            Val::new(PhpValue::Long(30), PhpType::Long),
            0,
        );
        let arr_val = Val::new(PhpValue::Array(Box::new(arr)), PhpType::Array);
        let result = http_build_query(&[arr_val]).unwrap();
        let s = zval_get_string(&result).as_str().to_string();
        assert!(s.contains("name=John"));
        assert!(s.contains("age=30"));
    }
}
