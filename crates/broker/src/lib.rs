//! # envenb-broker
//!
//! Performs HTTP calls against a [`Connection`] on behalf of a caller that must
//! never see the credential. The broker:
//!
//! 1. resolves the connection's `auth_secret` through `EnvEnb::with_secret`,
//! 2. injects it per `auth_style` (`bearer`, `header:<Name>`, `query:<name>`, `supabase`, `none`),
//! 3. sends the request with `reqwest`,
//! 4. scrubs any occurrence of the credential from the response before returning it.
//!
//! Permission checks and audit logging are the caller's job (see `envenb-mcp`);
//! the broker only knows how to make the call safely.

pub mod sigv4;

use std::sync::Arc;
use std::time::Duration;

use envenb_core::{Connection, ConnectionKind, EnvEnb};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum BrokerError {
    #[error("connection has no credential configured (auth_secret is empty)")]
    NoCredential,
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("unsupported HTTP method: {0}")]
    InvalidMethod(String),
    #[error("invalid header name or value: {0}")]
    InvalidHeader(String),
    #[error("unsupported auth style: {0}")]
    InvalidAuthStyle(String),
    #[error("aws connection is missing metadata.{0}")]
    MissingAwsMetadata(&'static str),
    #[error("signing failed: {0}")]
    Signing(String),
    #[error("request failed: {0}")]
    Http(String),
    #[error(transparent)]
    Core(#[from] envenb_core::CoreError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerRequest {
    pub method: String,
    /// Path relative to the connection's base URL, e.g. `/rest/v1/beans`.
    pub path: String,
    #[serde(default)]
    pub query: Vec<(String, String)>,
    /// Extra non-auth headers. `Authorization`, `apikey`, `Cookie` are rejected.
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerResponse {
    pub status: u16,
    pub content_type: Option<String>,
    /// Parsed JSON when possible, otherwise the (truncated, scrubbed) text.
    pub body: serde_json::Value,
    pub truncated: bool,
}

const MAX_BODY_BYTES: usize = 256 * 1024;
const FORBIDDEN_CALLER_HEADERS: [&str; 5] = [
    "authorization",
    "apikey",
    "cookie",
    "x-api-key",
    "proxy-authorization",
];

pub struct Broker {
    core: Arc<EnvEnb>,
    client: reqwest::Client,
}

impl Broker {
    pub fn new(core: Arc<EnvEnb>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("envenb-broker/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client");
        Self { core, client }
    }

    /// Human-readable summary for audit logs, e.g. `GET /rest/v1/beans?select=*`.
    pub fn summarize(req: &BrokerRequest) -> String {
        let mut s = format!("{} {}", req.method.to_ascii_uppercase(), req.path);
        if !req.query.is_empty() {
            let q: Vec<String> = req.query.iter().map(|(k, v)| format!("{k}={v}")).collect();
            s.push('?');
            s.push_str(&q.join("&"));
        }
        s
    }

    pub async fn call(
        &self,
        connection: &Connection,
        req: &BrokerRequest,
    ) -> Result<BrokerResponse, BrokerError> {
        let method = reqwest::Method::from_bytes(req.method.to_ascii_uppercase().as_bytes())
            .map_err(|_| BrokerError::InvalidMethod(req.method.clone()))?;
        let url = build_url(&connection.base_url, &req.path)?;

        let mut builder = self.client.request(method, url).query(&req.query);
        for (name, value) in &req.headers {
            if FORBIDDEN_CALLER_HEADERS.contains(&name.to_ascii_lowercase().as_str()) {
                return Err(BrokerError::InvalidHeader(format!(
                    "{name} is managed by the broker"
                )));
            }
            builder = builder.header(
                reqwest::header::HeaderName::from_bytes(name.as_bytes())
                    .map_err(|_| BrokerError::InvalidHeader(name.clone()))?,
                reqwest::header::HeaderValue::from_str(value)
                    .map_err(|_| BrokerError::InvalidHeader(name.clone()))?,
            );
        }
        if let Some(extra) = connection.metadata.get("headers").and_then(|h| h.as_object()) {
            for (k, v) in extra {
                if let Some(v) = v.as_str() {
                    builder = builder.header(k.as_str(), v);
                }
            }
        }
        if let Some(body) = &req.body {
            builder = builder.json(body);
        }

        // Inject the credential inside the closure so the plaintext lives only here.
        let mut fingerprint: Option<String> = None;
        let request = match (&connection.auth_secret, connection.auth_style.as_str()) {
            (_, "none") => builder.build().map_err(|e| BrokerError::Http(e.to_string()))?,
            (None, _) => return Err(BrokerError::NoCredential),
            (Some(secret_name), "sigv4") => {
                let meta = |k: &'static str| {
                    connection
                        .metadata
                        .get(k)
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .ok_or(BrokerError::MissingAwsMetadata(k))
                };
                let region = meta("region")?.to_string();
                let service = meta("service")?.to_string();
                let id_secret = meta("access_key_id_secret")?.to_string();
                let mut request = builder.build().map_err(|e| BrokerError::Http(e.to_string()))?;
                let access_key_id = self
                    .core
                    .with_secret(&connection.environment_id, &id_secret, |s| s.to_string())
                    .await?;
                let signed = self
                    .core
                    .with_secret(&connection.environment_id, secret_name, |secret| {
                        fingerprint = Some(secret.to_string());
                        sigv4::sign(
                            &mut request,
                            &sigv4::SigningScope {
                                region: &region,
                                service: &service,
                            },
                            &access_key_id,
                            secret,
                            chrono::Utc::now(),
                        )
                    })
                    .await?;
                signed.map_err(BrokerError::Signing)?;
                request
            }
            (Some(secret_name), style) => {
                let style = style.to_string();
                let kind = connection.kind;
                let built = self
                    .core
                    .with_secret(&connection.environment_id, secret_name, |secret| {
                        fingerprint = Some(secret.to_string());
                        apply_auth(builder, &style, kind, secret)
                    })
                    .await?;
                built?.build().map_err(|e| BrokerError::Http(e.to_string()))?
            }
        };

        let response = self
            .client
            .execute(request)
            .await
            .map_err(|e| BrokerError::Http(redact_err(e.to_string(), fingerprint.as_deref())))?;
        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let bytes = response
            .bytes()
            .await
            .map_err(|e| BrokerError::Http(e.to_string()))?;
        let truncated = bytes.len() > MAX_BODY_BYTES;
        let slice = &bytes[..bytes.len().min(MAX_BODY_BYTES)];
        let mut text = String::from_utf8_lossy(slice).into_owned();
        if let Some(fp) = fingerprint.as_deref()
            && !fp.is_empty()
        {
            text = text.replace(fp, "[REDACTED]");
        }
        // Drop the plaintext copy as soon as the scrub is done.
        if let Some(mut fp) = fingerprint.take() {
            unsafe { fp.as_bytes_mut() }.fill(0);
        }
        let body = if !truncated {
            serde_json::from_str(&text).unwrap_or(serde_json::Value::String(text))
        } else {
            serde_json::Value::String(text)
        };
        Ok(BrokerResponse {
            status,
            content_type,
            body,
            truncated,
        })
    }
}

fn redact_err(msg: String, secret: Option<&str>) -> String {
    match secret {
        Some(s) if !s.is_empty() => msg.replace(s, "[REDACTED]"),
        _ => msg,
    }
}

fn build_url(base: &str, path: &str) -> Result<reqwest::Url, BrokerError> {
    if path.contains("://") || path.starts_with("//") {
        return Err(BrokerError::InvalidPath(
            "path must be relative to the connection base URL".into(),
        ));
    }
    if path.contains("..") {
        return Err(BrokerError::InvalidPath("path must not contain '..'".into()));
    }
    let joined = format!("{}/{}", base.trim_end_matches('/'), path.trim_start_matches('/'));
    let url = reqwest::Url::parse(&joined).map_err(|e| BrokerError::InvalidPath(e.to_string()))?;
    // The final host must be the connection's host: no smuggling via userinfo etc.
    let base_url = reqwest::Url::parse(base).map_err(|e| BrokerError::InvalidPath(e.to_string()))?;
    if url.host_str() != base_url.host_str()
        || url.port_or_known_default() != base_url.port_or_known_default()
    {
        return Err(BrokerError::InvalidPath(
            "path escaped the connection host".into(),
        ));
    }
    Ok(url)
}

fn apply_auth(
    builder: reqwest::RequestBuilder,
    style: &str,
    kind: ConnectionKind,
    secret: &str,
) -> Result<reqwest::RequestBuilder, BrokerError> {
    let header = |b: reqwest::RequestBuilder,
                  name: &str,
                  value: String|
     -> Result<reqwest::RequestBuilder, BrokerError> {
        let n = reqwest::header::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| BrokerError::InvalidHeader(name.to_string()))?;
        let mut v = reqwest::header::HeaderValue::from_str(&value)
            .map_err(|_| BrokerError::InvalidHeader("credential is not a valid header value".into()))?;
        v.set_sensitive(true);
        Ok(b.header(n, v))
    };
    match style {
        "bearer" => header(builder, "authorization", format!("Bearer {secret}")),
        "supabase" => {
            let b = header(builder, "apikey", secret.to_string())?;
            header(b, "authorization", format!("Bearer {secret}"))
        }
        s if s.starts_with("header:") => header(builder, &s["header:".len()..], secret.to_string()),
        s if s.starts_with("query:") => Ok(builder.query(&[(&s["query:".len()..], secret)])),
        "none" => Ok(builder),
        "sigv4" => Err(BrokerError::InvalidAuthStyle(format!(
            "sigv4 is handled by the broker for {kind} connections"
        ))),
        other => Err(BrokerError::InvalidAuthStyle(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_building_is_confined_to_the_host() {
        assert_eq!(
            build_url("https://api.openai.com/v1", "models").unwrap().as_str(),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            build_url("https://x.supabase.co", "/rest/v1/beans")
                .unwrap()
                .as_str(),
            "https://x.supabase.co/rest/v1/beans"
        );
        assert!(build_url("https://a.com", "https://evil.com/x").is_err());
        assert!(build_url("https://a.com", "//evil.com/x").is_err());
        assert!(build_url("https://a.com", "../../x").is_err());
    }

    #[test]
    fn summary_has_no_body_or_headers() {
        let req = BrokerRequest {
            method: "get".into(),
            path: "/rest/v1/beans".into(),
            query: vec![("select".into(), "*".into())],
            headers: vec![("X-Trace".into(), "1".into())],
            body: Some(serde_json::json!({"secret": "no"})),
        };
        assert_eq!(Broker::summarize(&req), "GET /rest/v1/beans?select=*");
    }

    #[test]
    fn auth_styles() {
        let c = reqwest::Client::new();
        let b = apply_auth(
            c.get("https://a.com"),
            "bearer",
            ConnectionKind::GenericHttp,
            "s3cr3t",
        )
        .unwrap();
        let r = b.build().unwrap();
        assert_eq!(r.headers()["authorization"], "Bearer s3cr3t");
        assert!(r.headers()["authorization"].is_sensitive());

        let b = apply_auth(c.get("https://a.com"), "supabase", ConnectionKind::Supabase, "k").unwrap();
        let r = b.build().unwrap();
        assert_eq!(r.headers()["apikey"], "k");

        let b = apply_auth(
            c.get("https://a.com"),
            "header:X-Api-Key",
            ConnectionKind::GenericHttp,
            "k",
        )
        .unwrap();
        assert_eq!(b.build().unwrap().headers()["x-api-key"], "k");

        let b = apply_auth(
            c.get("https://a.com"),
            "query:key",
            ConnectionKind::GenericHttp,
            "k",
        )
        .unwrap();
        assert_eq!(b.build().unwrap().url().query(), Some("key=k"));

        assert!(apply_auth(c.get("https://a.com"), "magic", ConnectionKind::GenericHttp, "k").is_err());
    }
}
