//! End-to-end checks of the `envfish` binary against an isolated data directory.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn envfish(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_envfish"));
    cmd.env("ENVFISH_HOME", home)
        .env("CI", "1")
        .env("ENVFISH_LANG", "en")
        .env("ENVFISH_KEY_BACKEND", "file")
        .env("ENVFISH_ALLOW_UNATTENDED", "1")
        .env_remove("RUST_LOG");
    // The test suite itself may run inside an agent session; the harness must
    // look like a plain shell so the human gate is exercised deliberately.
    for marker in [
        "CLAUDECODE",
        "CLAUDE_CODE_ENTRYPOINT",
        "CODEX_SANDBOX",
        "CODEX_CI",
        "CURSOR_AGENT",
        "GEMINI_CLI",
    ] {
        cmd.env_remove(marker);
    }
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

#[test]
fn credentials_store_copy_guard_and_run_injection() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    assert!(run(home, &["project", "add", "my-app"]).0);
    assert!(run(home, &["env", "development", "--create"]).0);

    let (ok, out, err) = run(
        home,
        &[
            "cred",
            "add",
            "qa-admin",
            "--kind",
            "account",
            "--field",
            "url=https://staging.example.com/login",
            "--field",
            "username=qa-user-NEEDLE",
            "--field",
            "password=pw-NEEDLE",
            "--field",
            "totp_secret=JBSWY3DPEHPK3PXP",
        ],
    );
    assert!(ok, "{err}");
    assert!(out.contains("Stored credential qa-admin"));

    let key = home.join("id_test");
    std::fs::write(
        &key,
        "-----BEGIN OPENSSH PRIVATE KEY-----\nKEY-NEEDLE\n-----END OPENSSH PRIVATE KEY-----\n",
    )
    .unwrap();
    let (ok, _, err) = run(
        home,
        &[
            "cred",
            "add",
            "bastion",
            "--kind",
            "ssh",
            "--field",
            "host=bastion.example.com",
            "--field",
            "user=deploy",
            "--field",
            &format!("private_key=@{}", key.display()),
        ],
    );
    assert!(ok, "{err}");

    // Required fields are enforced.
    let (ok, _, err) = run(
        home,
        &[
            "cred", "add", "broken", "--kind", "database", "--field", "host=db",
        ],
    );
    assert!(!ok && err.contains("require"), "{err}");

    // Listing and JSON never carry secret values.
    let (ok, out, _) = run(home, &["cred", "list", "--json"]);
    assert!(ok && out.contains("qa-admin") && out.contains("https://staging.example.com/login"));
    assert!(!out.contains("NEEDLE"));
    let (ok, out, _) = run(home, &["cred", "show", "qa-admin"]);
    assert!(ok && out.contains("••••••••") && !out.contains("NEEDLE"));

    // Copy refuses when stdout is not a TTY (this test harness).
    let (ok, _, err) = run(home, &["cred", "copy", "qa-admin"]);
    assert!(!ok && err.contains("interactive terminal"));

    // run --with-credentials injects into the child only.
    let (ok, out, err) = run(
        home,
        &[
            "run",
            "--with-credentials",
            "--",
            "sh",
            "-c",
            "printf '%s|%s' \"$ENVFISH_CRED_QA_ADMIN_USERNAME\" \"$ENVFISH_CRED_BASTION_HOST\"",
        ],
    );
    assert!(ok, "{err}");
    assert_eq!(out.trim(), "qa-user-NEEDLE|bastion.example.com");

    // Nothing on disk in the data dir holds plaintext (the key file we wrote lives in `home` too, skip it).
    for entry in std::fs::read_dir(home).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().is_some_and(|n| n == "id_test") {
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
fn export_env_writes_real_values_with_guard_rails() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    assert!(run(home, &["project", "add", "my-app"]).0);
    assert!(run(home, &["env", "development", "--create"]).0);
    assert!(run(home, &["var", "set", "APP_URL", "http://localhost:3000"]).0);
    let mut child = envfish(home)
        .args(["var", "set-secret", "API_KEY"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"sk-EXPORT with space")
        .unwrap();
    assert!(child.wait_with_output().unwrap().status.success());

    let target = home.join("out").join(".env.local");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    let (ok, _, err) = run(home, &["export-env", target.to_str().unwrap()]);
    assert!(ok, "{err}");
    let text = std::fs::read_to_string(&target).unwrap();
    assert!(text.contains("APP_URL=http://localhost:3000"));
    assert!(text.contains("API_KEY=\"sk-EXPORT with space\""));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&target).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    // Existing file needs --force.
    let (ok, _, err) = run(home, &["export-env", target.to_str().unwrap()]);
    assert!(!ok && err.contains("--force"));
    assert!(run(home, &["export-env", target.to_str().unwrap(), "--force"]).0);
}

#[test]
fn clean_deletes_only_fully_stored_dotenv_files() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let repo = home.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join(".env"), "APP_URL=http://x\nTOKEN=t\n").unwrap();
    std::fs::write(repo.join(".env.staging"), "ONLY_HERE=1\n").unwrap();
    std::fs::write(repo.join(".env.example"), "APP_URL=\n").unwrap();
    assert!(
        run(
            home,
            &["project", "add", "my-app", "--path", repo.to_str().unwrap()]
        )
        .0
    );
    assert!(run(home, &["env", "development", "--create"]).0);

    // Import .env with --delete but without --yes: non-interactive → file kept.
    let (ok, out, err) = run(
        home,
        &["import", repo.join(".env").to_str().unwrap(), "--yes", "--delete"],
    );
    assert!(ok, "{err}");
    assert!(out.contains("Deleted"), "{out}");
    assert!(!repo.join(".env").exists());

    // clean: .env.staging is not stored → kept; template untouched.
    let (ok, out, _) = run(home, &["clean", "--yes"]);
    assert!(ok, "{out}");
    assert!(out.contains("ONLY_HERE"));
    assert!(repo.join(".env.staging").exists());
    assert!(repo.join(".env.example").exists());
}

#[test]
fn plaintext_commands_refuse_agents_and_non_terminals() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    assert!(run(home, &["project", "add", "my-app"]).0);
    assert!(run(home, &["env", "development", "--create"]).0);
    assert!(run(home, &["var", "set", "APP_URL", "http://x"]).0);

    // No terminal and no override → refused.
    let out = envfish(home)
        .env_remove("ENVFISH_ALLOW_UNATTENDED")
        .args(["run", "sh", "-c", "echo $APP_URL"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("interactive terminal"));

    // Inside a Claude Code session → refused even with the override absent/present.
    let out = envfish(home)
        .env("CLAUDECODE", "1")
        .args(["run", "sh", "-c", "echo $APP_URL"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("AI agent session"));

    // Metadata commands keep working for agents.
    let out = envfish(home)
        .env("CLAUDECODE", "1")
        .args(["var", "list", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("APP_URL"));
    let out = envfish(home)
        .env_remove("ENVFISH_ALLOW_UNATTENDED")
        .args(["export-env", home.join("x.env").to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(!home.join("x.env").exists());
}

#[test]
fn selection_errors_name_the_next_command() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();

    // Nothing registered yet.
    let (ok, _, err) = run(home, &["import", ".env"]);
    assert!(!ok);
    assert!(
        err.contains("no projects yet") && err.contains("envfish project add"),
        "{err}"
    );

    // A project exists but none is selected.
    assert!(run(home, &["project", "add", "my-app"]).0);
    assert!(run(home, &["project", "add", "other"]).0);
    let state = home.join("state.json");
    let raw = std::fs::read_to_string(&state).unwrap().replace(
        &format!("\"current_project_id\": \"{}\"", current_project_id(home)),
        "\"current_project_id\": null",
    );
    std::fs::write(&state, raw).unwrap();
    let (ok, _, err) = run(home, &["import", ".env"]);
    assert!(!ok);
    assert!(
        err.contains("no project selected") && err.contains("envfish use my-app"),
        "{err}"
    );
    assert!(err.contains("my-app, other"), "lists what is available: {err}");

    // A project is selected but it has no environments.
    assert!(run(home, &["use", "my-app"]).0);
    let (ok, _, err) = run(home, &["import", ".env"]);
    assert!(!ok);
    assert!(
        err.contains("no environments in my-app") && err.contains("--create"),
        "{err}"
    );
}

fn current_project_id(home: &Path) -> String {
    let (_, out, _) = run(home, &["project", "list", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    v[0]["id"].as_str().unwrap().to_string()
}
