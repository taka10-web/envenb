//! AWS Signature Version 4 for the Broker.
//!
//! Signs an already-built `reqwest::Request` in place: adds `host`, `x-amz-date`,
//! `x-amz-content-sha256` and `Authorization`. Credentials are passed in as
//! `&str` and are not retained. The primitives are verified against the worked
//! example in the AWS documentation (see tests).

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

pub struct SigningScope<'a> {
    pub region: &'a str,
    pub service: &'a str,
}

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("hmac accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// RFC 3986 "unreserved" percent-encoding as required by SigV4.
fn uri_encode(input: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b'/' if !encode_slash => out.push('/'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn canonical_query(url: &reqwest::Url) -> String {
    let mut pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (uri_encode(&k, true), uri_encode(&v, true)))
        .collect();
    pairs.sort();
    pairs
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn canonical_path(url: &reqwest::Url, service: &str) -> String {
    let path = if url.path().is_empty() { "/" } else { url.path() };
    // S3 wants the path exactly once-encoded; other services double-encode.
    if service == "s3" {
        path.to_string()
    } else {
        uri_encode(path, false)
    }
}

/// Sign `request` with `access_key_id` / `secret_access_key` at `now`.
pub fn sign(
    request: &mut reqwest::Request,
    scope: &SigningScope<'_>,
    access_key_id: &str,
    secret_access_key: &str,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();
    let host = request
        .url()
        .host_str()
        .map(|h| match request.url().port() {
            Some(p) => format!("{h}:{p}"),
            None => h.to_string(),
        })
        .ok_or("request has no host")?;

    let payload_hash = match request.body() {
        None => sha256_hex(b""),
        Some(body) => match body.as_bytes() {
            Some(bytes) => sha256_hex(bytes),
            None => return Err("streaming bodies cannot be signed".into()),
        },
    };

    {
        let headers = request.headers_mut();
        headers.insert("host", HeaderValue::from_str(&host).map_err(|e| e.to_string())?);
        headers.insert(
            "x-amz-date",
            HeaderValue::from_str(&amz_date).map_err(|e| e.to_string())?,
        );
        headers.insert(
            "x-amz-content-sha256",
            HeaderValue::from_str(&payload_hash).map_err(|e| e.to_string())?,
        );
    }

    let (signed_headers, canonical_headers) = canonical_headers(request.headers());
    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        request.method().as_str(),
        canonical_path(request.url(), scope.service),
        canonical_query(request.url()),
        canonical_headers,
        signed_headers,
        payload_hash
    );

    let credential_scope = format!("{date_stamp}/{}/{}/aws4_request", scope.region, scope.service);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );
    let signature = hex::encode(hmac(
        &signing_key(secret_access_key, &date_stamp, scope),
        string_to_sign.as_bytes(),
    ));

    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={access_key_id}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );
    let mut value = HeaderValue::from_str(&authorization).map_err(|e| e.to_string())?;
    value.set_sensitive(true);
    request
        .headers_mut()
        .insert(HeaderName::from_static("authorization"), value);
    Ok(())
}

fn signing_key(secret_access_key: &str, date_stamp: &str, scope: &SigningScope<'_>) -> Vec<u8> {
    let k_date = hmac(
        format!("AWS4{secret_access_key}").as_bytes(),
        date_stamp.as_bytes(),
    );
    let k_region = hmac(&k_date, scope.region.as_bytes());
    let k_service = hmac(&k_region, scope.service.as_bytes());
    hmac(&k_service, b"aws4_request")
}

/// Lower-cased, sorted header names + canonical `name:value\n` block.
fn canonical_headers(headers: &HeaderMap) -> (String, String) {
    let mut entries: Vec<(String, String)> = headers
        .iter()
        .filter(|(name, _)| {
            let n = name.as_str();
            n == "host" || n == "content-type" || n.starts_with("x-amz-")
        })
        .map(|(name, value)| {
            let v = value
                .to_str()
                .unwrap_or("")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            (name.as_str().to_ascii_lowercase(), v)
        })
        .collect();
    entries.sort();
    let signed = entries
        .iter()
        .map(|(n, _)| n.as_str())
        .collect::<Vec<_>>()
        .join(";");
    let canonical = entries
        .iter()
        .map(|(n, v)| format!("{n}:{v}\n"))
        .collect::<String>();
    (signed, canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    const SCOPE: SigningScope<'static> = SigningScope {
        region: "us-east-1",
        service: "iam",
    };
    const SECRET: &str = "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY";

    /// Reproduces the worked example from the AWS SigV4 documentation exactly
    /// (canonical request hash and final signature).
    #[test]
    fn matches_the_aws_documentation_example() {
        let canonical_request = "GET\n/\nAction=ListUsers&Version=2010-05-08\ncontent-type:application/x-www-form-urlencoded; charset=utf-8\nhost:iam.amazonaws.com\nx-amz-date:20150830T123600Z\n\ncontent-type;host;x-amz-date\ne3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(
            sha256_hex(canonical_request.as_bytes()),
            "f536975d06c0309214f805bb90ccff089219ecd68b2577efef23edd43b7e1a59"
        );
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n20150830T123600Z\n20150830/us-east-1/iam/aws4_request\n{}",
            sha256_hex(canonical_request.as_bytes())
        );
        assert_eq!(
            hex::encode(hmac(
                &signing_key(SECRET, "20150830", &SCOPE),
                string_to_sign.as_bytes()
            )),
            "5d672d79c15b13162d9279b0855cfba6789a8edb4c82c400e06b5924a6f2b5d7"
        );
    }

    #[test]
    fn signs_a_request_end_to_end() {
        let client = reqwest::Client::new();
        let mut request = client
            .get("https://iam.amazonaws.com/?Action=ListUsers&Version=2010-05-08")
            .header("content-type", "application/x-www-form-urlencoded; charset=utf-8")
            .build()
            .unwrap();
        let now = Utc.with_ymd_and_hms(2015, 8, 30, 12, 36, 0).unwrap();
        sign(&mut request, &SCOPE, "AKIDEXAMPLE", SECRET, now).unwrap();
        let auth = request.headers()["authorization"].to_str().unwrap();
        assert!(
            auth.starts_with("AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/iam/aws4_request, ")
        );
        assert!(auth.contains("SignedHeaders=content-type;host;x-amz-content-sha256;x-amz-date, Signature="));
        assert_eq!(auth.rsplit('=').next().unwrap().len(), 64);
        assert!(request.headers()["authorization"].is_sensitive());
        assert_eq!(request.headers()["x-amz-date"], "20150830T123600Z");
        assert_eq!(
            request.headers()["x-amz-content-sha256"],
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        // The secret never appears in any header.
        for (_, v) in request.headers() {
            assert!(!v.to_str().unwrap_or("").contains(SECRET));
        }
    }

    #[test]
    fn encodes_query_and_path_canonically() {
        let url = reqwest::Url::parse("https://x.amazonaws.com/a b/c?z=1&a=2&a=1&sp=a%20b").unwrap();
        assert_eq!(canonical_query(&url), "a=1&a=2&sp=a%20b&z=1");
        assert_eq!(canonical_path(&url, "s3"), "/a%20b/c");
        assert_eq!(canonical_path(&url, "iam"), "/a%2520b/c");
    }
}
