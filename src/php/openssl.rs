//! OpenSSL-compatible functions backed by Rust crypto (`sha2`).
//!
//! Implements `openssl_digest` for common SHA digests. MD5/SHA1 are handled
//! by the existing `md5()`/`sha1()` builtins. AES encryption/decryption
//! requires the `aes`/`cbc` crates which are not yet dependencies, so those
//! functions return `None` until added.

use sha2::{Digest, Sha256, Sha224, Sha384, Sha512};

/// Compute a digest of `data` using `method`. Returns hex string (or raw
/// bytes if `raw` is true). Returns `None` for unknown methods.
pub fn digest(data: &str, method: &str, raw: bool) -> Option<String> {
    let bytes = data.as_bytes();
    let result: Vec<u8> = match method.to_ascii_lowercase().as_str() {
        "sha224" => {
            let mut h = Sha224::new();
            h.update(bytes);
            h.finalize().to_vec()
        }
        "sha256" => {
            let mut h = Sha256::new();
            h.update(bytes);
            h.finalize().to_vec()
        }
        "sha384" => {
            let mut h = Sha384::new();
            h.update(bytes);
            h.finalize().to_vec()
        }
        "sha512" => {
            let mut h = Sha512::new();
            h.update(bytes);
            h.finalize().to_vec()
        }
        _ => return None,
    };
    if raw {
        Some(String::from_utf8_lossy(&result).to_string())
    } else {
        Some(result.iter().map(|b| format!("{b:02x}")).collect())
    }
}

/// AES-CBC encryption. Not yet implemented (requires `aes`/`cbc` crates).
pub fn encrypt(_data: &str, _method: &str, _key: &str, _iv: &str) -> Option<String> {
    None
}

/// AES-CBC decryption. Not yet implemented (requires `aes`/`cbc` crates).
pub fn decrypt(_data: &str, _method: &str, _key: &str, _iv: &str) -> Option<String> {
    None
}
