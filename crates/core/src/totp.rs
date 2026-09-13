//! RFC 6238 TOTP (SHA-1, 30 s, 6 digits) for `account` credentials.

use hmac::{Hmac, Mac};
use sha1::Sha1;

fn base32_decode(input: &str) -> Result<Vec<u8>, String> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bits: u32 = 0;
    let mut n_bits = 0;
    let mut out = Vec::new();
    for c in input.bytes().filter(|c| *c != b'=' && *c != b' ' && *c != b'-') {
        let v = ALPHABET
            .iter()
            .position(|a| *a == c.to_ascii_uppercase())
            .ok_or_else(|| "totp secret is not valid base32".to_string())? as u32;
        bits = (bits << 5) | v;
        n_bits += 5;
        if n_bits >= 8 {
            out.push((bits >> (n_bits - 8)) as u8);
            n_bits -= 8;
            bits &= (1 << n_bits) - 1;
        }
    }
    if out.is_empty() {
        return Err("totp secret is empty".into());
    }
    Ok(out)
}

pub fn code_at(seed_base32: &str, unix_seconds: u64) -> Result<String, String> {
    let key = base32_decode(seed_base32)?;
    let counter = unix_seconds / 30;
    let mut mac = Hmac::<Sha1>::new_from_slice(&key).map_err(|e| e.to_string())?;
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let bin = ((digest[offset] as u32 & 0x7f) << 24)
        | ((digest[offset + 1] as u32) << 16)
        | ((digest[offset + 2] as u32) << 8)
        | digest[offset + 3] as u32;
    Ok(format!("{:06}", bin % 1_000_000))
}

pub fn code_now(seed_base32: &str) -> Result<String, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    code_at(seed_base32, now)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238 test vector (SHA-1, seed "12345678901234567890" = base32 GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ).
    #[test]
    fn rfc6238_vectors() {
        let seed = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        assert_eq!(code_at(seed, 59).unwrap(), "287082");
        assert_eq!(code_at(seed, 1111111109).unwrap(), "081804");
        assert_eq!(code_at(seed, 1234567890).unwrap(), "005924");
    }

    #[test]
    fn rejects_bad_seed() {
        assert!(code_at("!!!", 0).is_err());
    }
}
