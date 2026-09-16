//! The local HTTP proxy that existing SDKs point at.
//!
//! An SDK is configured with a base URL of `http://127.0.0.1:<port>/<connection>`
//! and a session token in place of its API key. This server then:
//!
//! 1. authenticates the *caller* with that session token,
//! 2. throws the caller's credentials away,
//! 3. attaches the real provider credential from the vault,
//! 4. streams the response back, scrubbing the credential as it passes.
//!
//! The application therefore never holds a provider secret, which is the whole
//! point: code written by an agent cannot read what was never given to it.

use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use http_body_util::{BodyExt, StreamBody};
use hyper::body::{Bytes, Frame, Incoming};
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;

use envenb_core::session::{SessionScope, SessionStore};
use envenb_core::{Action, EnvEnb, PermissionScope};

use crate::{Broker, BrokerRequest};

/// Body type returned to the SDK: a stream of chunks, never a buffered whole.
type ProxyBody = http_body_util::combinators::UnsyncBoxBody<Bytes, Infallible>;

/// A running proxy. Dropping the handle does not stop it; the task owns itself.
pub struct Proxy {
    addr: SocketAddr,
}

impl Proxy {
    /// The base URL an SDK should be pointed at, without a connection segment.
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

/// Start the proxy on `port`, bound to loopback only.
///
/// Passing `0` asks the OS for a free port, which is how a conflict on the
/// default port is escaped; callers learn the real one from [`Proxy::addr`].
pub async fn serve(core: Arc<EnvEnb>, sessions: SessionStore, port: u16) -> std::io::Result<Proxy> {
    // Loopback only. Binding 0.0.0.0 would expose every stored credential to
    // anything that can reach this machine.
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port))).await?;
    let addr = listener.local_addr()?;
    let broker = Arc::new(Broker::new(core.clone()));

    tokio::spawn(async move {
        loop {
            let Ok((stream, peer)) = listener.accept().await else {
                continue;
            };
            // Defence in depth: the bind above already excludes non-loopback.
            if !peer.ip().is_loopback() {
                continue;
            }
            let broker = broker.clone();
            let core = core.clone();
            let sessions = sessions.clone();
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let service =
                    service_fn(move |req| handle(req, broker.clone(), core.clone(), sessions.clone()));
                let _ = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                    .serve_connection(io, service)
                    .await;
            });
        }
    });

    Ok(Proxy { addr })
}

fn error(status: StatusCode, message: &str) -> Response<ProxyBody> {
    let body = serde_json::json!({ "error": { "message": message, "source": "envenb" } });
    let bytes = Bytes::from(body.to_string());
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(http_body_util::Full::new(bytes).boxed_unsync())
        .expect("static response")
}

/// The caller's token, from `Authorization: Bearer …` or `x-api-key`.
///
/// SDKs differ in which header they use for their API key, and this accepts
/// either: the token authenticates the caller to EnvEnb and is discarded
/// before anything is sent upstream.
fn caller_token(req: &Request<Incoming>) -> Option<String> {
    let headers = req.headers();
    if let Some(v) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        let t = v
            .strip_prefix("Bearer ")
            .or_else(|| v.strip_prefix("bearer "))
            .unwrap_or(v);
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    for name in ["x-api-key", "x-goog-api-key", "apikey"] {
        if let Some(v) = headers.get(name).and_then(|v| v.to_str().ok())
            && !v.is_empty()
        {
            return Some(v.to_string());
        }
    }
    None
}

async fn handle(
    req: Request<Incoming>,
    broker: Arc<Broker>,
    core: Arc<EnvEnb>,
    sessions: SessionStore,
) -> Result<Response<ProxyBody>, Infallible> {
    Ok(match route(req, broker, core, sessions).await {
        Ok(response) => response,
        Err((status, message)) => error(status, &message),
    })
}

async fn route(
    req: Request<Incoming>,
    broker: Arc<Broker>,
    core: Arc<EnvEnb>,
    sessions: SessionStore,
) -> Result<Response<ProxyBody>, (StatusCode, String)> {
    // `/<connection>/<rest…>`
    let path = req.uri().path().to_string();
    let mut segments = path.trim_start_matches('/').splitn(2, '/');
    let connection_name = segments.next().unwrap_or_default().to_string();
    let rest = segments.next().unwrap_or_default().to_string();
    if connection_name.is_empty() {
        return Err((
            StatusCode::NOT_FOUND,
            "path must start with a connection name, e.g. /openai/v1/chat/completions".into(),
        ));
    }

    let token = caller_token(&req).ok_or((
        StatusCode::UNAUTHORIZED,
        "missing session token; pass it as the SDK's API key".to_string(),
    ))?;
    let scope: SessionScope = sessions.lookup(&token).ok_or((
        StatusCode::UNAUTHORIZED,
        "unknown or expired session token".to_string(),
    ))?;
    if !scope.allows_connection(&connection_name) {
        return Err((
            StatusCode::FORBIDDEN,
            format!("this session may not use the connection {connection_name}"),
        ));
    }

    let connection = core
        .resolve_connection(&scope.environment_id, &connection_name)
        .await
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                format!("no connection named {connection_name} in this environment"),
            )
        })?;

    let method = req.method().clone();
    let action = match method.as_str() {
        "GET" | "HEAD" | "OPTIONS" => Action::Read,
        "DELETE" => Action::Delete,
        _ => Action::Write,
    };

    // Policy first: the vault is not touched for a call that is not allowed.
    let permission = PermissionScope {
        client_id: None,
        project_id: Some(scope.project_id.clone()),
        environment_id: Some(scope.environment_id.clone()),
        connection_id: Some(connection.id.clone()),
    };
    let summary = format!("{} /{rest}", method.as_str());
    let audit = |decision: &str| envenb_core::AuditRecord {
        client: None,
        client_name: scope.client.clone(),
        project_id: Some(scope.project_id.clone()),
        environment_id: Some(scope.environment_id.clone()),
        connection_id: Some(connection.id.clone()),
        action,
        summary: summary.clone(),
        decision: decision.to_string(),
    };
    match core
        .decide(&permission, action)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    {
        envenb_core::Decision::Allow => {}
        decision => {
            // ASK needs a human, and an SDK call has nobody to ask on this
            // path, so it is refused rather than left hanging. The approval
            // flow stays where a person is present: MCP and the desktop app.
            let _ = core.record_audit(audit("DENIED")).await;
            return Err((
                StatusCode::FORBIDDEN,
                format!("{decision:?} by EnvEnb policy for {summary} on {connection_name}"),
            ));
        }
    }

    let query: Vec<(String, String)> = req
        .uri()
        .query()
        .map(|q| {
            url_pairs(q)
                // A caller-supplied key would be replaced anyway; drop it so it
                // cannot reach the provider even if auth is `none`.
                .filter(|(k, _)| {
                    !matches!(
                        k.to_ascii_lowercase().as_str(),
                        "key" | "api_key" | "apikey" | "access_token"
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    let headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .filter_map(|(n, v)| v.to_str().ok().map(|v| (n.as_str().to_string(), v.to_string())))
        // `host` belongs to this hop; forwarding it would name the proxy.
        .filter(|(n, _)| n != "host")
        .collect();

    let body = req
        .into_body()
        .collect()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
        .to_bytes();

    let broker_request = BrokerRequest {
        method: method.as_str().to_string(),
        path: format!("/{rest}"),
        query,
        headers,
        body: None,
    };

    let streamed = broker
        .stream(&connection, &broker_request, (!body.is_empty()).then_some(body))
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let _ = core
        .record_audit(audit(if (200..400).contains(&streamed.status) {
            "ALLOWED"
        } else {
            "ERROR"
        }))
        .await;

    let mut builder =
        Response::builder().status(StatusCode::from_u16(streamed.status).unwrap_or(StatusCode::BAD_GATEWAY));
    for (name, value) in &streamed.headers {
        // Redaction changes the body length, so the upstream's own framing
        // headers would be wrong. Let hyper frame the response it actually
        // sends; forwarding a stale Content-Length truncates or drops it.
        if matches!(
            name.to_ascii_lowercase().as_str(),
            "content-length" | "content-encoding"
        ) {
            continue;
        }
        builder = builder.header(name, value);
    }

    // Each chunk is forwarded as it arrives; nothing waits for the last one.
    let stream = futures_util::StreamExt::map(streamed.body, |chunk| {
        Ok::<_, Infallible>(Frame::data(
            chunk.unwrap_or_else(|e| Bytes::from(format!("\n[envenb] upstream error: {e}\n"))),
        ))
    });
    builder
        .body(StreamBody::new(stream).boxed_unsync())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Parse `a=1&b=2` without pulling in a URL-encoding crate for this one use.
fn url_pairs(query: &str) -> impl Iterator<Item = (String, String)> + '_ {
    query.split('&').filter(|s| !s.is_empty()).map(|pair| {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        (percent_decode(k), percent_decode(v))
    })
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => match u8::from_str_radix(&s[i + 1..i + 3], 16) {
                Ok(b) => {
                    out.push(b);
                    i += 3;
                }
                Err(_) => {
                    out.push(bytes[i]);
                    i += 1;
                }
            },
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_parsing_handles_encoding() {
        let got: Vec<_> = url_pairs("a=1&b=hello%20world&c=x+y&empty=").collect();
        assert_eq!(
            got,
            vec![
                ("a".into(), "1".into()),
                ("b".into(), "hello world".into()),
                ("c".into(), "x y".into()),
                ("empty".into(), "".into()),
            ]
        );
    }
}
