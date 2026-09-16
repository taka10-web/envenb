//! The local proxy as an SDK sees it: a base URL, an API key, and a path.
//!
//! These drive the real listener over real TCP, because the properties that
//! matter — what reaches the provider, what reaches the caller, and who is
//! allowed to connect at all — only exist once the whole path is wired up.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::time::Duration;

use envenb_core::session::{SessionScope, SessionStore};
use envenb_core::vault::FileMasterKeyProvider;
use envenb_core::{EnvEnb, NewConnection, SecretValue};

const REAL_SECRET: &str = "sk-REAL-PROVIDER-SECRET";

struct Seen {
    first_line: String,
    headers: Vec<String>,
}

/// A provider that reports what it received and echoes the credential back.
fn spawn_upstream(seen: mpsc::Sender<Seen>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        while let Ok((mut stream, _)) = listener.accept() {
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut first_line = String::new();
            if reader.read_line(&mut first_line).is_err() {
                continue;
            }
            let mut headers = Vec::new();
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) if line.trim().is_empty() => break,
                    Ok(_) => headers.push(line.trim().to_string()),
                    Err(_) => break,
                }
            }
            let _ = seen.send(Seen {
                first_line: first_line.trim().to_string(),
                headers,
            });
            // Echo the secret back to prove the response is scrubbed.
            let body = format!("{{\"echo\":\"{REAL_SECRET}\"}}");
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.flush();
        }
    });
    format!("http://{addr}")
}

struct Fixture {
    proxy_base: String,
    token: String,
    other_token: String,
    seen: mpsc::Receiver<Seen>,
}

async fn fixture() -> Fixture {
    let (tx, seen) = mpsc::channel();
    let upstream = spawn_upstream(tx);

    let dir = tempfile::tempdir().unwrap();
    let provider = FileMasterKeyProvider::new(dir.path().join("master.key"));
    let app = EnvEnb::open_in_memory(&provider).await.unwrap();
    let p = app.create_project("my-app", None).await.unwrap();
    let e = app.create_environment(&p.id, "development").await.unwrap();
    app.set_secret_variable(&e.id, "API_KEY", SecretValue::new(REAL_SECRET))
        .await
        .unwrap();
    app.create_connection(NewConnection {
        environment_id: e.id.clone(),
        kind: envenb_core::ConnectionKind::GenericHttp,
        name: "provider".into(),
        base_url: Some(upstream),
        auth_secret: Some("API_KEY".into()),
        auth_style: Some("bearer".into()),
        metadata: None,
    })
    .await
    .unwrap();
    std::mem::forget(dir);

    let sessions = SessionStore::new();
    let token = sessions
        .issue(
            SessionScope {
                project_id: p.id.clone(),
                environment_id: e.id.clone(),
                connections: vec!["provider".into()],
                client: "test-sdk".into(),
            },
            Duration::from_secs(60),
        )
        .token;
    // A session scoped to a connection that does not include `provider`.
    let other_token = sessions
        .issue(
            SessionScope {
                project_id: p.id.clone(),
                environment_id: e.id.clone(),
                connections: vec!["something-else".into()],
                client: "test-sdk".into(),
            },
            Duration::from_secs(60),
        )
        .token;

    let proxy = envenb_broker::proxy::serve(std::sync::Arc::new(app), sessions, 0)
        .await
        .unwrap();

    Fixture {
        proxy_base: proxy.base_url(),
        token,
        other_token,
        seen,
    }
}

/// Minimal HTTP client so the test does not depend on how reqwest is configured.
fn get(url: &str, auth: Option<&str>) -> (u16, String) {
    let rest = url.strip_prefix("http://").unwrap();
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let mut stream = TcpStream::connect(host).unwrap();
    let auth_line = auth
        .map(|t| format!("Authorization: Bearer {t}\r\n"))
        .unwrap_or_default();
    write!(
        stream,
        "GET /{path} HTTP/1.1\r\nHost: {host}\r\n{auth_line}Connection: close\r\n\r\n"
    )
    .unwrap();
    stream.flush().unwrap();
    let mut raw = String::new();
    use std::io::Read;
    stream.read_to_string(&mut raw).unwrap();
    let status = raw
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    (status, raw)
}

#[tokio::test(flavor = "multi_thread")]
async fn an_sdk_call_reaches_the_provider_without_ever_holding_the_secret() {
    let f = fixture().await;

    // This is exactly what an SDK does: base URL + connection, key in the
    // Authorization header.
    let (status, raw) = get(&format!("{}/provider/v1/models", f.proxy_base), Some(&f.token));
    assert_eq!(status, 200, "raw response was: {raw:?}");

    // The provider received the real credential, once, and not the session token.
    let seen = f.seen.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(seen.first_line, "GET /v1/models HTTP/1.1");
    let auth: Vec<_> = seen
        .headers
        .iter()
        .filter(|h| h.to_ascii_lowercase().starts_with("authorization:"))
        .collect();
    assert_eq!(auth.len(), 1, "exactly one Authorization: {auth:?}");
    assert!(auth[0].contains(REAL_SECRET), "{:?}", auth[0]);
    assert!(
        !auth[0].contains(&f.token),
        "the session token must not be forwarded upstream: {:?}",
        auth[0]
    );

    // And the caller got the response with the credential scrubbed out.
    assert!(
        !raw.contains(REAL_SECRET),
        "the provider secret reached the caller: {raw}"
    );
    assert!(raw.contains("[REDACTED]"), "{raw}");
}

#[tokio::test(flavor = "multi_thread")]
async fn calls_without_a_valid_session_are_refused() {
    let f = fixture().await;

    let (status, _) = get(&format!("{}/provider/v1/models", f.proxy_base), None);
    assert_eq!(status, 401, "a missing token must be rejected");

    let (status, _) = get(
        &format!("{}/provider/v1/models", f.proxy_base),
        Some("not-a-real-token"),
    );
    assert_eq!(status, 401, "an unknown token must be rejected");

    let (status, _) = get(
        &format!("{}/provider/v1/models", f.proxy_base),
        Some(&f.other_token),
    );
    assert_eq!(
        status, 403,
        "a session scoped to another connection must be refused"
    );

    // Nothing above should have reached the provider at all.
    assert!(
        f.seen.recv_timeout(Duration::from_millis(300)).is_err(),
        "a refused call must not touch the provider"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_connection_decides_the_host_not_the_caller() {
    let f = fixture().await;

    // Try to escape the connection's base URL via the path.
    for path in ["provider/../../etc/passwd", "provider/https://evil.example/x"] {
        let (status, _) = get(&format!("{}/{path}", f.proxy_base), Some(&f.token));
        assert!(status >= 400, "path {path} should not be forwarded, got {status}");
    }
    assert!(
        f.seen.recv_timeout(Duration::from_millis(300)).is_err(),
        "an escaping path must not reach any provider"
    );
}
