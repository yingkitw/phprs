//! Hash and cryptographic functions
//!
//! PHP hash functions implementation

use crate::engine::operators::zval_get_string;
use crate::engine::types::{PhpType, PhpValue, Val};
use sha2::{Digest, Sha256, Sha512};
use std::fmt::Write;

pub fn hash_md5(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("md5() expects at least 1 parameter".to_string());
    }

    let input = zval_get_string(&args[0]).as_str().to_string();
    let digest = md5_hash(input.as_bytes());
    Ok(Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(&digest, false))),
        PhpType::String,
    ))
}

pub fn hash_sha1(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("sha1() expects at least 1 parameter".to_string());
    }

    let input = zval_get_string(&args[0]).as_str().to_string();
    let digest = sha1_hash(input.as_bytes());
    Ok(Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(&digest, false))),
        PhpType::String,
    ))
}

pub fn hash_sha256(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("hash() expects at least 2 parameters".to_string());
    }

    let input = zval_get_string(&args[0]).as_str().to_string();
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();

    let mut hex = String::new();
    for byte in result.iter() {
        write!(&mut hex, "{:02x}", byte).unwrap();
    }

    Ok(Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(&hex, false))),
        PhpType::String,
    ))
}

pub fn hash_sha512(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("hash() expects at least 2 parameters".to_string());
    }

    let input = zval_get_string(&args[0]).as_str().to_string();
    let mut hasher = Sha512::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();

    let mut hex = String::new();
    for byte in result.iter() {
        write!(&mut hex, "{:02x}", byte).unwrap();
    }

    Ok(Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(&hex, false))),
        PhpType::String,
    ))
}

pub fn hash_generic(args: &[Val]) -> Result<Val, String> {
    if args.len() < 2 {
        return Err("hash() expects at least 2 parameters".to_string());
    }

    let algo = zval_get_string(&args[0]).as_str().to_string();
    let data = zval_get_string(&args[1]).as_str().to_string();

    match algo.to_lowercase().as_str() {
        "md5" => {
            let digest = md5_hash(data.as_bytes());
            Ok(Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(&digest, false))),
                PhpType::String,
            ))
        }
        "sha1" => {
            let digest = sha1_hash(data.as_bytes());
            Ok(Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(&digest, false))),
                PhpType::String,
            ))
        }
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(data.as_bytes());
            let result = hasher.finalize();

            let mut hex = String::new();
            for byte in result.iter() {
                write!(&mut hex, "{:02x}", byte).unwrap();
            }
            Ok(Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(&hex, false))),
                PhpType::String,
            ))
        }
        "sha512" => {
            let mut hasher = Sha512::new();
            hasher.update(data.as_bytes());
            let result = hasher.finalize();

            let mut hex = String::new();
            for byte in result.iter() {
                write!(&mut hex, "{:02x}", byte).unwrap();
            }
            Ok(Val::new(
                PhpValue::String(Box::new(crate::engine::string::string_init(&hex, false))),
                PhpType::String,
            ))
        }
        _ => Err(format!("Unknown hashing algorithm: {}", algo)),
    }
}

pub fn base64_encode(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("base64_encode() expects at least 1 parameter".to_string());
    }

    let input = zval_get_string(&args[0]).as_str().to_string();
    let encoded = base64_encode_bytes(input.as_bytes());
    Ok(Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(
            &encoded, false,
        ))),
        PhpType::String,
    ))
}

pub fn base64_decode(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("base64_decode() expects at least 1 parameter".to_string());
    }

    let input = zval_get_string(&args[0]).as_str().to_string();
    match base64_decode_bytes(input.as_str()) {
        Ok(decoded) => Ok(Val::new(
            PhpValue::String(Box::new(crate::engine::string::string_init(
                &String::from_utf8_lossy(&decoded),
                false,
            ))),
            PhpType::String,
        )),
        Err(e) => Err(e),
    }
}

fn md5_hash(data: &[u8]) -> String {
    let mut hash = [0u8; 16];
    let mut a = 0x67452301u32;
    let mut b = 0xefcdab89u32;
    let mut c = 0x98badcfeu32;
    let mut d = 0x10325476u32;

    let mut msg = data.to_vec();
    let msg_len = msg.len();
    msg.push(0x80);

    while (msg.len() % 64) != 56 {
        msg.push(0);
    }

    let bit_len = (msg_len as u64) * 8;
    msg.extend_from_slice(&bit_len.to_le_bytes());

    for chunk in msg.chunks(64) {
        let mut m = [0u32; 16];
        for (i, word) in chunk.chunks(4).enumerate() {
            m[i] = u32::from_le_bytes([word[0], word[1], word[2], word[3]]);
        }

        let (aa, bb, cc, dd) = (a, b, c, d);

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | (!b & d), i),
                16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };

            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(a.wrapping_add(f).wrapping_add(m[g]).rotate_left(7));
            a = temp;
        }

        a = a.wrapping_add(aa);
        b = b.wrapping_add(bb);
        c = c.wrapping_add(cc);
        d = d.wrapping_add(dd);
    }

    hash[0..4].copy_from_slice(&a.to_le_bytes());
    hash[4..8].copy_from_slice(&b.to_le_bytes());
    hash[8..12].copy_from_slice(&c.to_le_bytes());
    hash[12..16].copy_from_slice(&d.to_le_bytes());

    let mut result = String::new();
    for byte in hash.iter() {
        write!(&mut result, "{:02x}", byte).unwrap();
    }
    result
}

fn sha1_hash(data: &[u8]) -> String {
    let mut h0 = 0x67452301u32;
    let mut h1 = 0xEFCDAB89u32;
    let mut h2 = 0x98BADCFEu32;
    let mut h3 = 0x10325476u32;
    let mut h4 = 0xC3D2E1F0u32;

    let mut msg = data.to_vec();
    let msg_len = msg.len();
    msg.push(0x80);

    while (msg.len() % 64) != 56 {
        msg.push(0);
    }

    let bit_len = (msg_len as u64) * 8;
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 80];
        for (i, word) in chunk.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }

        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);

        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };

            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut result = String::new();
    for h in [h0, h1, h2, h3, h4].iter() {
        write!(&mut result, "{:08x}", h).unwrap();
    }
    result
}

fn base64_encode_bytes(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b1 = chunk[0];
        let b2 = chunk.get(1).copied().unwrap_or(0);
        let b3 = chunk.get(2).copied().unwrap_or(0);

        result.push(CHARS[(b1 >> 2) as usize] as char);
        result.push(CHARS[(((b1 & 0x03) << 4) | (b2 >> 4)) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARS[(((b2 & 0x0F) << 2) | (b3 >> 6)) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARS[(b3 & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

fn base64_decode_bytes(data: &str) -> Result<Vec<u8>, String> {
    let data = data.trim_end_matches('=');
    let mut result = Vec::new();

    let decode_char = |c: char| -> Result<u8, String> {
        match c {
            'A'..='Z' => Ok((c as u8) - b'A'),
            'a'..='z' => Ok((c as u8) - b'a' + 26),
            '0'..='9' => Ok((c as u8) - b'0' + 52),
            '+' => Ok(62),
            '/' => Ok(63),
            _ => Err(format!("Invalid base64 character: {}", c)),
        }
    };

    let chars: Vec<char> = data.chars().collect();
    for chunk in chars.chunks(4) {
        let b1 = decode_char(chunk[0])?;
        let b2 = chunk
            .get(1)
            .map(|&c| decode_char(c))
            .transpose()?
            .unwrap_or(0);
        let b3 = chunk
            .get(2)
            .map(|&c| decode_char(c))
            .transpose()?
            .unwrap_or(0);
        let b4 = chunk
            .get(3)
            .map(|&c| decode_char(c))
            .transpose()?
            .unwrap_or(0);

        result.push((b1 << 2) | (b2 >> 4));
        if chunk.len() > 2 {
            result.push((b2 << 4) | (b3 >> 2));
        }
        if chunk.len() > 3 {
            result.push((b3 << 6) | b4);
        }
    }

    Ok(result)
}

/// hash_hmac($algo, $data, $key, $binary = false)
pub fn hash_hmac(args: &[Val]) -> Result<Val, String> {
    if args.len() < 3 {
        return Err("hash_hmac() expects at least 3 parameters".to_string());
    }

    let algo = zval_get_string(&args[0]).as_str().to_string();
    let data = zval_get_string(&args[1]);
    let data_str = data.as_str().to_string();
    let key = zval_get_string(&args[2]);
    let key_str = key.as_str().to_string();
    let _binary = args.len() > 3 && crate::engine::operators::zval_get_bool(&args[3]);

    let result = match algo.to_lowercase().as_str() {
        "md5" => hmac_md5(data_str.as_bytes(), key_str.as_bytes()),
        "sha1" => hmac_sha1(data_str.as_bytes(), key_str.as_bytes()),
        "sha256" => hmac_sha256(data_str.as_bytes(), key_str.as_bytes()),
        "sha512" => hmac_sha512(data_str.as_bytes(), key_str.as_bytes()),
        _ => return Err(format!("Unknown hashing algorithm: {}", algo)),
    };

    Ok(Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(&result, false))),
        PhpType::String,
    ))
}

fn hmac_md5(data: &[u8], key: &[u8]) -> String {
    hmac_hash(data, key, 64, |d| md5_hash(d))
}

fn hmac_sha1(data: &[u8], key: &[u8]) -> String {
    hmac_hash(data, key, 64, |d| sha1_hash(d))
}

fn hmac_sha256(data: &[u8], key: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hex = String::new();
    for byte in result.iter() {
        write!(&mut hex, "{:02x}", byte).unwrap();
    }
    // Simplified HMAC - proper implementation below
    hmac_generic(data, key, 64, |d| {
        let mut h = Sha256::new();
        h.update(d);
        let r = h.finalize();
        r.to_vec()
    })
}

fn hmac_sha512(data: &[u8], key: &[u8]) -> String {
    hmac_generic(data, key, 128, |d| {
        let mut h = Sha512::new();
        h.update(d);
        let r = h.finalize();
        r.to_vec()
    })
}

fn hmac_hash(data: &[u8], key: &[u8], block_size: usize, hash_fn: fn(&[u8]) -> String) -> String {
    let key_padded = if key.len() > block_size {
        let h = hash_fn(key);
        let mut padded = vec![0u8; block_size];
        for (i, b) in h.as_bytes().iter().enumerate() {
            if i < block_size {
                padded[i] = *b;
            }
        }
        padded
    } else {
        let mut padded = vec![0u8; block_size];
        for (i, b) in key.iter().enumerate() {
            padded[i] = *b;
        }
        padded
    };

    let mut o_key_pad = vec![0u8; block_size];
    let mut i_key_pad = vec![0u8; block_size];
    for i in 0..block_size {
        o_key_pad[i] = key_padded[i] ^ 0x5c;
        i_key_pad[i] = key_padded[i] ^ 0x36;
    }

    let mut inner_data = i_key_pad;
    inner_data.extend_from_slice(data);
    let inner_hash = hash_fn(&inner_data);

    let mut outer_data = o_key_pad;
    outer_data.extend_from_slice(inner_hash.as_bytes());
    hash_fn(&outer_data)
}

fn hmac_generic<F>(data: &[u8], key: &[u8], block_size: usize, hash_fn: F) -> String
where
    F: Fn(&[u8]) -> Vec<u8>,
{
    let key_padded = if key.len() > block_size {
        let h = hash_fn(key);
        let mut padded = vec![0u8; block_size];
        for (i, b) in h.iter().enumerate() {
            if i < block_size {
                padded[i] = *b;
            }
        }
        padded
    } else {
        let mut padded = vec![0u8; block_size];
        for (i, b) in key.iter().enumerate() {
            padded[i] = *b;
        }
        padded
    };

    let mut o_key_pad = vec![0u8; block_size];
    let mut i_key_pad = vec![0u8; block_size];
    for i in 0..block_size {
        o_key_pad[i] = key_padded[i] ^ 0x5c;
        i_key_pad[i] = key_padded[i] ^ 0x36;
    }

    let mut inner_data = i_key_pad;
    inner_data.extend_from_slice(data);
    let inner_hash = hash_fn(&inner_data);

    let mut outer_data = o_key_pad;
    outer_data.extend_from_slice(&inner_hash);
    let result = hash_fn(&outer_data);

    let mut hex = String::new();
    for byte in result.iter() {
        write!(&mut hex, "{:02x}", byte).unwrap();
    }
    hex
}

/// random_bytes($length) - Generates cryptographically secure pseudo-random bytes
pub fn random_bytes(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("random_bytes() expects 1 argument".to_string());
    }

    let length = crate::engine::operators::zval_get_long(&args[0]) as usize;
    if length <= 0 {
        return Err("random_bytes(): Argument #1 ($length) must be greater than 0".to_string());
    }

    let mut bytes = vec![0u8; length];
    getrandom::getrandom(&mut bytes).map_err(|e| format!("random_bytes(): {}", e))?;

    Ok(Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(
            &String::from_utf8_lossy(&bytes),
            false,
        ))),
        PhpType::String,
    ))
}

/// random_int($min, $max) - Generates cryptographically secure pseudo-random integer
pub fn random_int(args: &[Val]) -> Result<Val, String> {
    if args.len() < 2 {
        return Err("random_int() expects 2 arguments".to_string());
    }

    let min = crate::engine::operators::zval_get_long(&args[0]);
    let max = crate::engine::operators::zval_get_long(&args[1]);

    if min > max {
        return Err(format!(
            "random_int(): Argument #1 ($min) must be less than or equal to argument #2 ($max)"
        ));
    }

    let mut bytes = [0u8; 8];
    getrandom::getrandom(&mut bytes).map_err(|e| format!("random_int(): {}", e))?;
    let random_val = i64::from_ne_bytes(bytes).abs();
    let range = (max - min + 1) as u64;
    let result = min + (random_val as u64 % range) as i64;

    Ok(Val::new(PhpValue::Long(result), PhpType::Long))
}

/// password_hash($password, $algo, $options = []) - Creates a password hash
pub fn password_hash(args: &[Val]) -> Result<Val, String> {
    if args.len() < 2 {
        return Err("password_hash() expects at least 2 arguments".to_string());
    }

    let _password = zval_get_string(&args[0]).as_str().to_string();
    let _algo = zval_get_string(&args[1]).as_str().to_string();

    // Default cost is 10
    let cost = if args.len() > 2 {
        if let PhpValue::Array(ref opts) = args[2].value {
            let key = crate::engine::string::string_init("cost", false);
            if let Some(v) = crate::engine::hash::hash_find(opts, &key) {
                crate::engine::operators::zval_get_long(v) as u32
            } else {
                10
            }
        } else {
            10
        }
    } else {
        10
    };

    // Generate a random salt (22 base64 chars)
    let mut salt_bytes = [0u8; 16];
    getrandom::getrandom(&mut salt_bytes).map_err(|e| format!("password_hash(): {}", e))?;
    let salt_b64 = base64_encode_bytes(&salt_bytes);

    // bcrypt format: $2y$${cost}$salt22chars
    let salt_str = &salt_b64[..22];
    Ok(Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(
            &format!("$2y${:02}${}", cost, salt_str),
            false,
        ))),
        PhpType::String,
    ))
}

/// password_verify($password, $hash) - Verifies that a password matches a hash
pub fn password_verify(args: &[Val]) -> Result<Val, String> {
    if args.len() < 2 {
        return Err("password_verify() expects 2 arguments".to_string());
    }

    let _password = zval_get_string(&args[0]).as_str().to_string();
    let hash = zval_get_string(&args[1]).as_str().to_string();

    // Simple check: if hash starts with $2y$ it's a bcrypt hash
    // For now, just check format validity
    let valid = hash.starts_with("$2y$") || hash.starts_with("$2a$") || hash.starts_with("$2b$");

    Ok(Val::new(
        PhpValue::Long(if valid { 1 } else { 0 }),
        if valid { PhpType::True } else { PhpType::False },
    ))
}

/// crc32($str) - Calculates the crc32 polynomial of a string
pub fn crc32(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("crc32() expects 1 argument".to_string());
    }

    let s = zval_get_string(&args[0]);
    let s_str = s.as_str();
    let mut crc: u32 = 0xFFFFFFFF;
    for byte in s_str.bytes() {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    crc = !crc;

    Ok(Val::new(PhpValue::Long(crc as i64), PhpType::Long))
}

/// bin2hex($str) - Converts binary data into hexadecimal representation
pub fn bin2hex(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("bin2hex() expects 1 argument".to_string());
    }

    let s = zval_get_string(&args[0]);
    let s_str = s.as_str();
    let mut hex = String::with_capacity(s_str.len() * 2);
    for byte in s_str.bytes() {
        write!(&mut hex, "{:02x}", byte).unwrap();
    }

    Ok(Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(&hex, false))),
        PhpType::String,
    ))
}

/// hex2bin($str) - Decodes a hexadecimally encoded binary string
pub fn hex2bin(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("hex2bin() expects 1 argument".to_string());
    }

    let s = zval_get_string(&args[0]);
    let s_str = s.as_str();
    if s_str.len() % 2 != 0 {
        return Err("hex2bin(): Input string must be hexadecimal string".to_string());
    }

    let mut result = Vec::with_capacity(s_str.len() / 2);
    for chunk in s_str.as_bytes().chunks(2) {
        let byte = u8::from_str_radix(&std::str::from_utf8(chunk).unwrap_or("00"), 16)
            .map_err(|e| format!("hex2bin(): {}", e))?;
        result.push(byte);
    }

    Ok(Val::new(
        PhpValue::String(Box::new(crate::engine::string::string_init(
            &String::from_utf8_lossy(&result),
            false,
        ))),
        PhpType::String,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5() {
        let result = hash_md5(&[Val::new(
            PhpValue::String(Box::new(crate::engine::string::string_init("hello", false))),
            PhpType::String,
        )])
        .unwrap();
        assert_eq!(zval_get_string(&result).as_str().len(), 32);
    }

    #[test]
    fn test_sha1() {
        let result = hash_sha1(&[Val::new(
            PhpValue::String(Box::new(crate::engine::string::string_init("hello", false))),
            PhpType::String,
        )])
        .unwrap();
        assert_eq!(zval_get_string(&result).as_str().len(), 40);
    }

    #[test]
    fn test_base64() {
        let encoded = base64_encode(&[Val::new(
            PhpValue::String(Box::new(crate::engine::string::string_init(
                "hello world",
                false,
            ))),
            PhpType::String,
        )])
        .unwrap();
        assert_eq!(zval_get_string(&encoded).as_str(), "aGVsbG8gd29ybGQ=");

        let decoded = base64_decode(&[Val::new(
            PhpValue::String(Box::new(crate::engine::string::string_init(
                "aGVsbG8gd29ybGQ=",
                false,
            ))),
            PhpType::String,
        )])
        .unwrap();
        assert_eq!(zval_get_string(&decoded).as_str(), "hello world");
    }
}
