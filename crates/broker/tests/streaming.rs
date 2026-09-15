//! The streaming proxy against a mock upstream.
//!
//! Three properties matter here and none of them can be checked by reading the
//! code: chunks arrive as they are produced rather than at the end, the
//! caller's own credentials never reach the provider, and the real credential
//! never reaches the caller.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use envenb_broker::{Broker, BrokerRequest};
use envenb_core::{EnvEnb, NewConnection, SecretValue};
use envenb_core::vault::FileMasterKeyProvider;
use futures_util::StreamExt;

/// What the mock upstream saw, so a test can assert on the request it received.
struct Seen {
    headers: Vec<String>,
    first_line: String,
}

/// An upstream that emits three SSE chunks with a gap between them, so a
/// buffering proxy is distinguishable from a streaming one.
fn spawn_sse_upstream(seen: mpsc::Sender<Seen>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let Ok((stream, _)) = listener.accept() else {
            return;
        };
        serve_sse(stream, seen);
    });
    format!("http://{addr}")
}

fn serve_sse(mut stream: TcpStream, seen: mpsc::Sender<Seen>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut first_line = String::new();
    reader.read_line(&mut first_line).unwrap();
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap() == 0 || line.trim().is_empty() {
            break;
        }
        headers.push(line.trim().to_string());
    }
    let _ = seen.send(Seen {
        headers,
        first_line: first_line.trim().to_string(),
    });

    stream
        .write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n",
        )
        .unwrap();
    stream.flush().unwrap();
    // Three chunks, 150 ms apart. The secret is echoed back in the last one to
    // prove the scrubber runs on the streamed path too.
    for body in [
        "data: one\n\n".to_string(),
        "data: two\n\n".to_string(),
        "data: sk-UPSTREAM-LEAK\n\n".to_string(),
    ] {
        std::thread::sleep(Duration::from_millis(150));
        write!(stream, "{:x}\r\n{}\r\n", body.len(), body).unwrap();
        stream.flush().unwrap();
    }
    let _ = stream.write_all(b"0\r\n\r\n");
    let _ = stream.flush();
}

async fn app_with_connection(base_url: &str) -> (std::sync::Arc<EnvEnb>, envenb_core::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let provider = FileMasterKeyProvider::new(dir.path().join("master.key"));
    let app = EnvEnb::open_in_memory(&provider).await.unwrap();
    let p = app.create_project("my-app", None).await.unwrap();
    let e = app.create_environment(&p.id, "development").await.unwrap();
    app.set_secret_variable(&e.id, "API_KEY", SecretValue::new("sk-UPSTREAM-LEAK"))
        .await
        .unwrap();
    let conn = app
        .create_connection(NewConnection {
            environment_id: e.id.clone(),
            kind: envenb_core::ConnectionKind::GenericHttp,
            name: "mock".into(),
            base_url: Some(base_url.to_string()),
            auth_secret: Some("API_KEY".into()),
            auth_style: Some("bearer".into()),
            metadata: None,
        })
        .await
        .unwrap();
    std::mem::forget(dir);
    (std::sync::Arc::new(app), conn)
}

#[tokio::test]
async fn chunks_arrive_as_they_are_produced() {
    let (tx, rx) = mpsc::channel();
    let base = spawn_sse_upstream(tx);
    let (app, conn) = app_with_connection(&base).await;
    let broker = Broker::new(app);

    let started = Instant::now();
    let mut resp = broker
        .stream(
            &conn,
            &BrokerRequest {
                method: "POST".into(),
                path: "/v1/chat".into(),
                query: vec![],
                // An SDK pointed at the proxy always sends its own placeholder.
                headers: vec![("authorization".into(), "Bearer envenb-placeholder".into())],
                body: None,
            },
            Some(bytes::Bytes::from_static(b"{}")),
        )
        .await
        .expect("stream should start");
    assert_eq!(resp.status, 200);

    let mut first_chunk_at = None;
    let mut collected = String::new();
    while let Some(chunk) = resp.body.next().await {
        let chunk = chunk.expect("chunk");
        if first_chunk_at.is_none() {
            first_chunk_at = Some(started.elapsed());
        }
        collected.push_str(std::str::from_utf8(&chunk).unwrap());
    }

    // A buffering proxy could only deliver the first chunk after the last one
    // (~450 ms). Streaming delivers it around 150 ms.
    let first = first_chunk_at.expect("at least one chunk");
    assert!(
        first < Duration::from_millis(400),
        "first chunk took {first:?}; the body was buffered instead of streamed"
    );

    assert!(collected.contains("data: one"));
    assert!(collected.contains("data: two"));
    // The upstream echoed the credential back; it must not reach the caller.
    assert!(
        !collected.contains("sk-UPSTREAM-LEAK"),
        "the credential leaked into the streamed body: {collected}"
    );
    assert!(collected.contains("[REDACTED]"));

    // And the provider saw the real credential, not the SDK's placeholder.
    let seen = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(seen.first_line, "POST /v1/chat HTTP/1.1");
    let auth: Vec<_> = seen
        .headers
        .iter()
        .filter(|h| h.to_ascii_lowercase().starts_with("authorization:"))
        .collect();
    assert_eq!(auth.len(), 1, "exactly one Authorization header: {auth:?}");
    assert!(
        auth[0].contains("sk-UPSTREAM-LEAK"),
        "the real credential must be attached: {:?}",
        auth[0]
    );
    assert!(
        !auth[0].contains("envenb-placeholder"),
        "the caller's placeholder must be discarded: {:?}",
        auth[0]
    );
}
