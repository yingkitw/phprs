//! Hash and cryptographic functions
//!
//! PHP hash functions implementation

use crate::engine::types::{PhpType, PhpValue, Val};
use crate::engine::operators::zval_get_string;
use sha2::{Digest, Sha256, Sha512};
use std::fmt::Write;

pub fn hash_md5(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("md5() expects at least 1 parameter".to_string());
    }
    
    let input = zval_get_string(&args[0]).as_str().to_string();
    let digest = md5_hash(input.as_bytes());
    Ok(Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&digest, false))), PhpType::String))
}

pub fn hash_sha1(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("sha1() expects at least 1 parameter".to_string());
    }
    
    let input = zval_get_string(&args[0]).as_str().to_string();
    let digest = sha1_hash(input.as_bytes());
    Ok(Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&digest, false))), PhpType::String))
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
    
    Ok(Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&hex, false))), PhpType::String))
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
    
    Ok(Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&hex, false))), PhpType::String))
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
            Ok(Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&digest, false))), PhpType::String))
        }
        "sha1" => {
            let digest = sha1_hash(data.as_bytes());
            Ok(Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&digest, false))), PhpType::String))
        }
        "sha256" => {
            let mut hasher = Sha256::new();
            hasher.update(data.as_bytes());
            let result = hasher.finalize();
            
            let mut hex = String::new();
            for byte in result.iter() {
                write!(&mut hex, "{:02x}", byte).unwrap();
            }
            Ok(Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&hex, false))), PhpType::String))
        }
        "sha512" => {
            let mut hasher = Sha512::new();
            hasher.update(data.as_bytes());
            let result = hasher.finalize();
            
            let mut hex = String::new();
            for byte in result.iter() {
                write!(&mut hex, "{:02x}", byte).unwrap();
            }
            Ok(Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&hex, false))), PhpType::String))
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
    Ok(Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&encoded, false))), PhpType::String))
}

pub fn base64_decode(args: &[Val]) -> Result<Val, String> {
    if args.is_empty() {
        return Err("base64_decode() expects at least 1 parameter".to_string());
    }
    
    let input = zval_get_string(&args[0]).as_str().to_string();
    match base64_decode_bytes(input.as_str()) {
        Ok(decoded) => Ok(Val::new(PhpValue::String(Box::new(crate::engine::string::string_init(&String::from_utf8_lossy(&decoded), false))), PhpType::String)),
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
            
            let temp = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(w[i]);
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
        let b2 = chunk.get(1).map(|&c| decode_char(c)).transpose()?.unwrap_or(0);
        let b3 = chunk.get(2).map(|&c| decode_char(c)).transpose()?.unwrap_or(0);
        let b4 = chunk.get(3).map(|&c| decode_char(c)).transpose()?.unwrap_or(0);
        
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5() {
        let result = hash_md5(&[Val::new(PhpValue::String(Box::new(crate::engine::string::string_init("hello", false))), PhpType::String)]).unwrap();
        assert_eq!(zval_get_string(&result).as_str().len(), 32);
    }

    #[test]
    fn test_sha1() {
        let result = hash_sha1(&[Val::new(PhpValue::String(Box::new(crate::engine::string::string_init("hello", false))), PhpType::String)]).unwrap();
        assert_eq!(zval_get_string(&result).as_str().len(), 40);
    }

    #[test]
    fn test_base64() {
        let encoded = base64_encode(&[Val::new(PhpValue::String(Box::new(crate::engine::string::string_init("hello world", false))), PhpType::String)]).unwrap();
        assert_eq!(zval_get_string(&encoded).as_str(), "aGVsbG8gd29ybGQ=");
        
        let decoded = base64_decode(&[Val::new(PhpValue::String(Box::new(crate::engine::string::string_init("aGVsbG8gd29ybGQ=", false))), PhpType::String)]).unwrap();
        assert_eq!(zval_get_string(&decoded).as_str(), "hello world");
    }
}
