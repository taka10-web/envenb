//! End-to-end checks of the `envfish` binary against an isolated data directory.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn envfish(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_envfish"));
    cmd.env("ENVFISH_HOME", home)
        .env("CI", "1")
        .env("ENVFISH_LANG", "en")
        .env_remove("RUST_LOG");
    cmd
}

fn run(home: &Path, args: &[&str]) -> (bool, String, String) {
    let out = envfish(home).args(args).output().expect("spawn envfish");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn project_add_and_list() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();

    let (ok, out, err) = run(home, &["project", "add", "my-app", "--path", "/tmp/my-app"]);
    assert!(ok, "stderr: {err}");
    assert!(out.contains("Registered project my-app"));

    let (ok, out, _) = run(home, &["project", "add", "Goldfish App"]);
    assert!(ok);
    assert!(out.contains("Goldfish App"));

    let (ok, _, err) = run(home, &["project", "add", "my-app"]);
    assert!(!ok, "duplicate name must fail");
    assert!(err.contains("already exists"));

    let (ok, out, _) = run(home, &["project", "list"]);
    assert!(ok);
    assert!(out.contains("my-app") && out.contains("Goldfish App"));
    assert!(out.contains("/tmp/my-app"));

    let (ok, out, _) = run(home, &["project", "list", "--json"]);
    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&out).expect("valid json");
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[test]
fn status_use_env_and_variables() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();

    assert!(run(home, &["project", "add", "my-app"]).0);
    assert!(run(home, &["use", "my-app"]).0);
    assert!(run(home, &["env", "development", "--create"]).0);

    let (ok, out, _) = run(home, &["status"]);
    assert!(ok);
    assert!(out.contains("Project:") && out.contains("my-app"));
    assert!(out.contains("Environment:") && out.contains("development"));

    assert!(run(home, &["var", "set", "APP_URL", "http://localhost:3000"]).0);

    // Secret comes in via stdin, never argv.
    let needle = "sk-live-INTEGRATION-NEEDLE";
    let mut child = envfish(home)
        .args(["var", "set-secret", "OPENAI_API_KEY"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(needle.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(!String::from_utf8_lossy(&out.stdout).contains(needle));

    let (ok, out, _) = run(home, &["var", "list"]);
    assert!(ok);
    assert!(out.contains("APP_URL") && out.contains("PUBLIC") && out.contains("http://localhost:3000"));
    assert!(out.contains("OPENAI_API_KEY") && out.contains("SECRET"));
    assert!(!out.contains(needle), "secret must never be printed");

    let (ok, out, _) = run(home, &["var", "list", "--json"]);
    assert!(ok);
    assert!(!out.contains(needle));

    // Nothing on disk may contain the plaintext: db, WAL, state.
    for entry in std::fs::read_dir(home).unwrap() {
        let path = entry.unwrap().path();
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            !bytes.windows(needle.len()).any(|w| w == needle.as_bytes()),
            "plaintext found in {}",
            path.display()
        );
    }

    let (ok, out, _) = run(home, &["status", "--json"]);
    assert!(ok);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["secret_count"], 1);
}

#[test]
fn no_animation_and_non_tty_output_is_plain() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    assert!(run(home, &["project", "add", "P"]).0);
    let (ok, out, _) = run(home, &["--no-animation", "status"]);
    assert!(ok);
    assert!(!out.contains('\u{1b}'), "no ANSI escapes when piped");
    assert!(!out.contains("><(((°>"), "no fish when piped");
}

#[test]
fn japanese_help_and_messages() {
    let dir = tempfile::tempdir().unwrap();
    let out = envfish(dir.path())
        .env("ENVFISH_LANG", "ja")
        .arg("--help")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(text.contains("使い方:") && text.contains("コマンド:") && text.contains("ヘルプを表示"));

    // No subcommand: usage on stdout, exit 0.
    let out = envfish(dir.path()).env("ENVFISH_LANG", "ja").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("クイックスタート"));

    let out = envfish(dir.path())
        .env("ENVFISH_LANG", "ja")
        .args(["use", "nope"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("プロジェクトが見つかりません"));
}

#[test]
fn phase2_connections_permissions_import_run() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    assert!(run(home, &["project", "add", "my-app"]).0);
    assert!(run(home, &["env", "development", "--create"]).0);
    assert!(run(home, &["env", "production", "--create"]).0);
    assert!(run(home, &["env", "development"]).0);

    // Secret via stdin, then a connection referencing it by name.
    let mut child = envfish(home)
        .args(["var", "set-secret", "SUPABASE_KEY"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"sb-secret-NEEDLE")
        .unwrap();
    assert!(child.wait_with_output().unwrap().status.success());

    let (ok, _, err) = run(
        home,
        &[
            "connection",
            "add",
            "supabase",
            "--kind",
            "supabase",
            "--url",
            "https://demo.supabase.co",
            "--secret",
            "NOPE",
        ],
    );
    assert!(
        !ok && err.contains("variable not found"),
        "unknown credential must be rejected: {err}"
    );
    let (ok, out, err) = run(
        home,
        &[
            "connection",
            "add",
            "supabase",
            "--kind",
            "supabase",
            "--url",
            "https://demo.supabase.co",
            "--secret",
            "SUPABASE_KEY",
        ],
    );
    assert!(ok, "{err}");
    assert!(out.contains("Added connection supabase"));
    let (ok, out, _) = run(home, &["connection", "list", "--json"]);
    assert!(ok && out.contains("SUPABASE_KEY") && !out.contains("sb-secret-NEEDLE"));

    // Permission engine defaults and an explicit rule.
    assert!(run(home, &["ai", "register", "Claude Code", "--kind", "claude_code"]).0);
    let (ok, out, _) = run(
        home,
        &[
            "ai",
            "check",
            "--client",
            "claude code",
            "--connection",
            "supabase",
            "--json",
        ],
    );
    assert!(ok, "{out}");
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["READ"], "ALLOW");
    assert_eq!(json["WRITE"], "ASK");
    assert_eq!(json["DELETE"], "DENY");
    assert!(
        run(
            home,
            &[
                "ai",
                "permit",
                "WRITE",
                "ALLOW",
                "--client",
                "claude code",
                "--connection",
                "supabase"
            ]
        )
        .0
    );
    let (_, out, _) = run(
        home,
        &[
            "ai",
            "check",
            "--client",
            "claude code",
            "--connection",
            "supabase",
            "--json",
        ],
    );
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["WRITE"], "ALLOW");
    // Production defaults are stricter.
    assert!(run(home, &["env", "production"]).0);
    let (_, out, _) = run(home, &["ai", "check", "--json"]);
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["READ"], "ASK");
    assert_eq!(json["WRITE"], "DENY");
    assert!(run(home, &["env", "development"]).0);

    // .env import with classification, then export-example without secrets.
    let env_file = home.join("test.env");
    std::fs::write(
        &env_file,
        "NODE_ENV=development\nSTRIPE_SECRET_KEY=sk_test_NEEDLE\nDATABASE_URL=\"postgres://u:pw@h/db\"\n",
    )
    .unwrap();
    let (ok, out, err) = run(home, &["import", env_file.to_str().unwrap(), "--yes", "--json"]);
    assert!(ok, "{err}");
    let report: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(report["public_added"], 1);
    assert_eq!(report["secret_added"], 2);
    let (ok, out, _) = run(home, &["export-example", "-"]);
    assert!(ok);
    assert!(out.contains("NODE_ENV=development") && out.contains("STRIPE_SECRET_KEY=\n"));
    assert!(!out.contains("NEEDLE"));

    // `run` injects into the child only.
    let (ok, out, err) = run(
        home,
        &[
            "run",
            "sh",
            "-c",
            "printf '%s|%s' \"$STRIPE_SECRET_KEY\" \"$ENVFISH_ENVIRONMENT\"",
        ],
    );
    assert!(ok, "{err}");
    assert_eq!(out.trim(), "sk_test_NEEDLE|development");

    // Settings shared with the desktop app.
    let (ok, out, _) = run(home, &["config", "theme", "dark", "--json"]);
    assert!(ok && out.contains("\"theme\": \"dark\""));

    // Nothing on disk holds plaintext.
    for entry in std::fs::read_dir(home).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().is_some_and(|n| n == "test.env") {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            !bytes.windows(6).any(|w| w == b"NEEDLE"),
            "plaintext found in {}",
            path.display()
        );
    }
}

#[test]
fn mcp_stdio_handshake_and_no_secret_tools() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    assert!(run(home, &["project", "add", "P"]).0);
    let mut child = envfish(home)
        .args(["mcp", "--client", "test-client"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-06-18\"}}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n")
            .unwrap();
    }
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    let init: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(init["result"]["serverInfo"]["name"], "envfish");
    let tools: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    let names: Vec<String> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.iter().any(|n| n == "call_service"));
    assert!(
        !names
            .iter()
            .any(|n| n.contains("secret") || n.contains("dump") || n.contains("export"))
    );
    // The client registered itself.
    let (ok, out, _) = run(home, &["ai", "clients", "--json"]);
    assert!(ok && out.contains("test-client"));
}
