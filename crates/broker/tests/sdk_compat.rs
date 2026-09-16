//! Real SDKs pointed at the proxy.
//!
//! The proxy is only useful if an unmodified SDK can talk to it, so these
//! tests run the actual `openai` / `@google/genai` clients (and their Python
//! equivalents) against a mock provider, with nothing but a base URL and a
//! session token. They assert the two things that matter:
//!
//! * the SDK works, including streaming, and
//! * the provider sees the real credential while the SDK process never does.
//!
//! The SDKs are installed on demand into a cache directory. When they cannot
//! be installed (offline CI, no node/python) the test skips rather than fails:
//! a missing toolchain is not a regression in EnvEnb.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use envenb_core::session::{SessionScope, SessionStore};
use envenb_core::vault::FileMasterKeyProvider;
use envenb_core::{EnvEnb, NewConnection, SecretValue};

const REAL_SECRET: &str = "sk-PROVIDER-ONLY-SECRET";

/// Headers the mock provider saw, so a test can prove what was forwarded.
struct Seen {
    headers: Vec<String>,
    path: String,
}

/// A provider that answers both OpenAI-shaped and Gemini-shaped requests,
/// streaming when asked.
fn spawn_provider(seen: mpsc::Sender<Seen>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        while let Ok((stream, _)) = listener.accept() {
            let seen = seen.clone();
            std::thread::spawn(move || serve_one(stream, seen));
        }
    });
    format!("http://{addr}")
}

fn serve_one(mut stream: TcpStream, seen: mpsc::Sender<Seen>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let path = request_line.split_whitespace().nth(1).unwrap_or("").to_string();

    let mut headers = Vec::new();
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) if line.trim().is_empty() => break,
            Ok(_) => {
                let line = line.trim().to_string();
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    content_length = v.trim().parse().unwrap_or(0);
                }
                headers.push(line);
            }
            Err(_) => break,
        }
    }
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        use std::io::Read;
        let _ = reader.read_exact(&mut body);
    }
    let body = String::from_utf8_lossy(&body).into_owned();
    let _ = seen.send(Seen {
        headers,
        path: path.clone(),
    });

    let wants_stream = body.contains("\"stream\":true")
        || body.contains("\"stream\": true")
        || path.contains("streamGenerateContent")
        || path.contains("alt=sse");

    if wants_stream {
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n",
            )
            .unwrap();
        stream.flush().unwrap();
        let events = if path.contains("generateContent") || path.contains("gemini") {
            vec![
                format!("data: {}\n\n", gemini_chunk("Hel")),
                format!("data: {}\n\n", gemini_chunk("lo")),
            ]
        } else {
            vec![
                format!("data: {}\n\n", openai_chunk("Hel")),
                format!("data: {}\n\n", openai_chunk("lo")),
                "data: [DONE]\n\n".to_string(),
            ]
        };
        for event in events {
            std::thread::sleep(Duration::from_millis(60));
            let _ = write!(stream, "{:x}\r\n{}\r\n", event.len(), event);
            let _ = stream.flush();
        }
        let _ = stream.write_all(b"0\r\n\r\n");
    } else {
        // Echo the credential back so the redaction path is exercised too.
        let payload = if path.contains("generateContent") {
            gemini_full()
        } else {
            openai_full()
        };
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            payload.len(),
            payload
        );
    }
    let _ = stream.flush();
}

fn openai_chunk(text: &str) -> String {
    serde_json::json!({
        "id": "chatcmpl-mock",
        "object": "chat.completion.chunk",
        "created": 0,
        "model": "mock",
        "choices": [{"index": 0, "delta": {"content": text}, "finish_reason": null}]
    })
    .to_string()
}

fn openai_full() -> String {
    serde_json::json!({
        "id": "chatcmpl-mock",
        "object": "chat.completion",
        "created": 0,
        "model": "mock",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": format!("echo {REAL_SECRET}")},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    })
    .to_string()
}

fn gemini_chunk(text: &str) -> String {
    serde_json::json!({
        "candidates": [{"content": {"parts": [{"text": text}], "role": "model"}}]
    })
    .to_string()
}

fn gemini_full() -> String {
    serde_json::json!({
        "candidates": [{
            "content": {"parts": [{"text": format!("echo {REAL_SECRET}")}], "role": "model"},
            "finishReason": "STOP"
        }]
    })
    .to_string()
}

struct Harness {
    proxy_base: String,
    token: String,
    seen: mpsc::Receiver<Seen>,
}

async fn harness(connection: &str, auth_style: &str) -> Harness {
    let (tx, seen) = mpsc::channel();
    let provider = spawn_provider(tx);

    let dir = tempfile::tempdir().unwrap();
    let key = FileMasterKeyProvider::new(dir.path().join("master.key"));
    let app = EnvEnb::open_in_memory(&key).await.unwrap();
    let p = app.create_project("my-app", None).await.unwrap();
    let e = app.create_environment(&p.id, "development").await.unwrap();
    app.set_secret_variable(&e.id, "API_KEY", SecretValue::new(REAL_SECRET))
        .await
        .unwrap();
    app.create_connection(NewConnection {
        environment_id: e.id.clone(),
        kind: envenb_core::ConnectionKind::GenericHttp,
        name: connection.into(),
        base_url: Some(provider),
        auth_secret: Some("API_KEY".into()),
        auth_style: Some(auth_style.into()),
        metadata: None,
    })
    .await
    .unwrap();
    // These tests are about SDK compatibility, not policy: an SDK POSTs, and
    // WRITE defaults to ASK, which has nobody to ask here. Policy itself is
    // covered by proxy_http.rs.
    app.set_permission(
        envenb_core::PermissionScope {
            client_id: None,
            project_id: Some(p.id.clone()),
            environment_id: Some(e.id.clone()),
            connection_id: None,
        },
        envenb_core::Action::Write,
        envenb_core::Decision::Allow,
    )
    .await
    .unwrap();
    std::mem::forget(dir);

    let sessions = SessionStore::new();
    let token = sessions
        .issue(
            SessionScope {
                project_id: p.id,
                environment_id: e.id,
                connections: vec![],
                client: "sdk-test".into(),
            },
            Duration::from_secs(300),
        )
        .token;

    let proxy = envenb_broker::proxy::serve(std::sync::Arc::new(app), sessions, 0)
        .await
        .unwrap();

    Harness {
        proxy_base: proxy.base_url(),
        token,
        seen,
    }
}

impl Harness {
    /// Assert that the provider got the real credential and not the session token.
    fn assert_provider_saw_the_real_credential(&self) {
        let seen = self
            .seen
            .recv_timeout(Duration::from_secs(20))
            .expect("the provider should have been called");
        let joined = seen.headers.join("\n");
        assert!(
            joined.contains(REAL_SECRET) || seen.path.contains(REAL_SECRET),
            "the provider did not receive the real credential.\npath: {}\nheaders: {joined}",
            seen.path
        );
        assert!(
            !joined.contains(&self.token),
            "the session token was forwarded to the provider: {joined}"
        );
    }
}

// --------------------------------------------------------------------------
// Running the SDKs
// --------------------------------------------------------------------------

fn cache_dir() -> PathBuf {
    let base = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"));
    base.join("sdk-compat")
}

/// Install `packages` with npm into a cache dir. `None` when npm is unusable.
fn node_sdk_dir(packages: &[&str]) -> Option<PathBuf> {
    let dir = cache_dir().join("node");
    std::fs::create_dir_all(&dir).ok()?;
    if !dir.join("package.json").exists() {
        std::fs::write(dir.join("package.json"), r#"{"name":"sdk-compat","private":true,"type":"module"}"#).ok()?;
    }
    let missing: Vec<_> = packages
        .iter()
        .filter(|p| !dir.join("node_modules").join(p).exists())
        .collect();
    if !missing.is_empty() {
        let out = Command::new("npm")
            .arg("install")
            .arg("--no-audit")
            .arg("--no-fund")
            .args(packages)
            .current_dir(&dir)
            .output()
            .ok()?;
        if !out.status.success() {
            eprintln!(
                "skipping: npm install failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            return None;
        }
    }
    Some(dir)
}

/// Install `packages` into a venv. `None` when python is unusable.
fn python_sdk_dir(packages: &[&str]) -> Option<PathBuf> {
    let dir = cache_dir().join("python");
    std::fs::create_dir_all(&dir).ok()?;
    let venv = dir.join("venv");
    let python = venv.join("bin").join("python3");
    if !python.exists() {
        let out = Command::new("python3")
            .args(["-m", "venv", venv.to_str()?])
            .output()
            .ok()?;
        if !out.status.success() {
            eprintln!("skipping: could not create a venv");
            return None;
        }
    }
    for package in packages {
        let check = Command::new(&python)
            .args(["-c", &format!("import {}", package.replace('-', "_"))])
            .output()
            .ok()?;
        if !check.status.success() {
            let out = Command::new(&python)
                .args(["-m", "pip", "install", "--quiet", package])
                .output()
                .ok()?;
            if !out.status.success() {
                eprintln!(
                    "skipping: pip install {package} failed: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
                return None;
            }
        }
    }
    Some(venv)
}

/// Run `script` with the interpreter, returning stdout. The session token is
/// passed in the environment exactly as `envenb session` would export it.
fn run_script(program: &Path, dir: &Path, script: &str, name: &str, h: &Harness) -> Option<String> {
    let path = dir.join(name);
    std::fs::write(&path, script).ok()?;
    let out = Command::new(program)
        .arg(&path)
        .current_dir(dir)
        .env("ENVENB_PROXY_URL", &h.proxy_base)
        .env("ENVENB_SESSION_TOKEN", &h.token)
        // The point of the whole design: no provider key in the environment.
        .env_remove("OPENAI_API_KEY")
        .env_remove("GEMINI_API_KEY")
        .env_remove("GOOGLE_API_KEY")
        .output()
        .ok()?;
    if !out.status.success() {
        panic!(
            "{name} failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

// --------------------------------------------------------------------------
// JavaScript
// --------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn openai_js_sdk_streams_through_the_proxy() {
    let Some(dir) = node_sdk_dir(&["openai"]) else {
        return;
    };
    let h = harness("openai", "bearer").await;

    let script = r#"
import OpenAI from 'openai';

const client = new OpenAI({
  baseURL: `${process.env.ENVENB_PROXY_URL}/openai/v1`,
  apiKey: process.env.ENVENB_SESSION_TOKEN,
});

// Non-streaming first: proves the response is scrubbed.
const once = await client.chat.completions.create({
  model: 'mock',
  messages: [{ role: 'user', content: 'hi' }],
});
console.log('ONCE:' + once.choices[0].message.content);

// Then streaming: proves chunks survive the proxy.
const stream = await client.chat.completions.create({
  model: 'mock',
  messages: [{ role: 'user', content: 'hi' }],
  stream: true,
});
let text = '';
for await (const chunk of stream) {
  text += chunk.choices[0]?.delta?.content ?? '';
}
console.log('STREAM:' + text);
console.log('ENVKEY:' + (process.env.OPENAI_API_KEY ?? 'absent'));
"#;

    let Some(stdout) = run_script(Path::new("node"), &dir, script, "openai_test.mjs", &h) else {
        return;
    };

    assert!(stdout.contains("STREAM:Hello"), "streaming broke: {stdout}");
    assert!(
        stdout.contains("ENVKEY:absent"),
        "the SDK process should have no provider key: {stdout}"
    );
    assert!(
        !stdout.contains(REAL_SECRET),
        "the credential reached the SDK process: {stdout}"
    );
    assert!(
        stdout.contains("ONCE:echo [REDACTED]"),
        "the echoed credential was not redacted: {stdout}"
    );
    h.assert_provider_saw_the_real_credential();
}

#[tokio::test(flavor = "multi_thread")]
async fn google_genai_js_sdk_streams_through_the_proxy() {
    let Some(dir) = node_sdk_dir(&["@google/genai"]) else {
        return;
    };
    // Gemini authenticates with a header, not a bearer token.
    let h = harness("gemini", "header:x-goog-api-key").await;

    let script = r#"
import { GoogleGenAI } from '@google/genai';

const ai = new GoogleGenAI({
  apiKey: process.env.ENVENB_SESSION_TOKEN,
  httpOptions: { baseUrl: `${process.env.ENVENB_PROXY_URL}/gemini` },
});

const stream = await ai.models.generateContentStream({
  model: 'gemini-2.5-flash',
  contents: 'hi',
});
let text = '';
for await (const chunk of stream) {
  text += chunk.text ?? '';
}
console.log('STREAM:' + text);
console.log('ENVKEY:' + (process.env.GEMINI_API_KEY ?? 'absent'));
"#;

    let Some(stdout) = run_script(Path::new("node"), &dir, script, "gemini_test.mjs", &h) else {
        return;
    };

    assert!(stdout.contains("STREAM:Hello"), "streaming broke: {stdout}");
    assert!(stdout.contains("ENVKEY:absent"), "{stdout}");
    assert!(
        !stdout.contains(REAL_SECRET),
        "the credential reached the SDK process: {stdout}"
    );
    h.assert_provider_saw_the_real_credential();
}

// --------------------------------------------------------------------------
// Python
// --------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn openai_python_sdk_streams_through_the_proxy() {
    let Some(venv) = python_sdk_dir(&["openai"]) else {
        return;
    };
    let h = harness("openai", "bearer").await;
    let python = venv.join("bin").join("python3");

    let script = r#"
import os
from openai import OpenAI

client = OpenAI(
    base_url=f"{os.environ['ENVENB_PROXY_URL']}/openai/v1",
    api_key=os.environ["ENVENB_SESSION_TOKEN"],
)

once = client.chat.completions.create(
    model="mock", messages=[{"role": "user", "content": "hi"}]
)
print("ONCE:" + once.choices[0].message.content)

stream = client.chat.completions.create(
    model="mock", messages=[{"role": "user", "content": "hi"}], stream=True
)
text = ""
for chunk in stream:
    if chunk.choices and chunk.choices[0].delta.content:
        text += chunk.choices[0].delta.content
print("STREAM:" + text)
print("ENVKEY:" + os.environ.get("OPENAI_API_KEY", "absent"))
"#;

    let Some(stdout) = run_script(&python, venv.parent().unwrap(), script, "openai_test.py", &h)
    else {
        return;
    };

    assert!(stdout.contains("STREAM:Hello"), "streaming broke: {stdout}");
    assert!(stdout.contains("ENVKEY:absent"), "{stdout}");
    assert!(
        !stdout.contains(REAL_SECRET),
        "the credential reached the SDK process: {stdout}"
    );
    assert!(stdout.contains("ONCE:echo [REDACTED]"), "{stdout}");
    h.assert_provider_saw_the_real_credential();
}
