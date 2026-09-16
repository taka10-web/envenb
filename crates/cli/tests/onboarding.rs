//! The path a new user actually walks, in order.
//!
//! Each documented step is run against an empty vault, so a missing or
//! misremembered command shows up here rather than in someone's terminal. The
//! point is not to re-test the proxy — other suites do that — but to prove the
//! instructions are complete and in the right order.

use std::path::Path;
use std::process::{Command, Stdio};

fn envenb(home: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_envenb"));
    cmd.env("ENVENB_HOME", home)
        .env("CI", "1")
        .env("ENVENB_LANG", "en")
        .env("ENVENB_KEY_BACKEND", "file")
        .env("ENVENB_ALLOW_UNATTENDED", "1")
        .env_remove("RUST_LOG");
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
    let out = envenb(home).args(args).output().unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Walk the documented setup from an empty vault to a session token.
#[test]
fn a_new_user_can_get_from_nothing_to_a_session() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();

    // 1. Register the project and environment.
    let (ok, _, err) = run(
        home,
        &["project", "add", "my-app", "--path", home.to_str().unwrap()],
    );
    assert!(ok, "project add failed: {err}");
    let (ok, _, err) = run(home, &["env", "development", "--create"]);
    assert!(ok, "env --create failed: {err}");

    // 2. Store a secret. Reading from stdin is what the docs show.
    let mut cmd = envenb(home)
        .args(["var", "set-secret", "PROVIDER_KEY"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        cmd.stdin
            .as_mut()
            .unwrap()
            .write_all(b"sk-ONBOARDING-SECRET")
            .unwrap();
    }
    assert!(cmd.wait_with_output().unwrap().status.success());

    // 3. Describe the service. Without this the proxy has nowhere to send a
    //    call, so it belongs in the documented path.
    let (ok, out, err) = run(
        home,
        &[
            "connection",
            "add",
            "provider",
            "--kind",
            "generic_http",
            "--url",
            "https://api.example.test",
            "--secret",
            "PROVIDER_KEY",
            "--auth",
            "bearer",
        ],
    );
    assert!(ok, "connection add failed: {err}\n{out}");

    // 4. The connection is listed, and its credential is named but not shown.
    let (ok, out, err) = run(home, &["connection", "list"]);
    assert!(ok, "{err}");
    assert!(out.contains("provider"), "{out}");
    assert!(out.contains("PROVIDER_KEY"), "{out}");
    assert!(
        !out.contains("sk-ONBOARDING-SECRET"),
        "a connection listing must not print the credential: {out}"
    );

    // 5. `envenb session` needs a running agent, and says so when there is
    //    none. A new user hitting this must be told what to start.
    let (ok, _, err) = run(home, &["session"]);
    assert!(!ok, "session should fail without an agent");
    assert!(
        err.contains("envenb agent"),
        "the error must name the command to run: {err}"
    );
}

/// `envenb run` is the other half of the story: it starts a command, and the
/// secret is not in it.
#[test]
fn run_starts_a_command_without_handing_over_a_secret() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    assert!(run(home, &["project", "add", "my-app"]).0);
    assert!(run(home, &["env", "development", "--create"]).0);
    assert!(run(home, &["var", "set", "APP_URL", "http://localhost:3000"]).0);

    let mut cmd = envenb(home)
        .args(["var", "set-secret", "PROVIDER_KEY"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        cmd.stdin
            .as_mut()
            .unwrap()
            .write_all(b"sk-ONBOARDING-SECRET")
            .unwrap();
    }
    assert!(cmd.wait_with_output().unwrap().status.success());

    let (ok, out, err) = run(
        home,
        &["run", "sh", "-c", "printf '%s|%s' \"$APP_URL\" \"$PROVIDER_KEY\""],
    );
    assert!(ok, "{err}");
    assert_eq!(
        out.trim(),
        "http://localhost:3000|",
        "PUBLIC should be present and the secret absent"
    );
    // And the user is pointed at the proxy rather than left wondering.
    assert!(
        err.contains("envenb session"),
        "run should say how to reach the secret: {err}"
    );
}

/// The README opens with this exact command. If it ever stops printing
/// `undefined`, the headline claim is false and this must fail.
#[test]
fn the_readme_headline_example_is_true() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    assert!(run(home, &["project", "add", "my-app"]).0);
    assert!(run(home, &["env", "development", "--create"]).0);

    let mut cmd = envenb(home)
        .args(["var", "set-secret", "OPENAI_API_KEY"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write;
        cmd.stdin
            .as_mut()
            .unwrap()
            .write_all(b"sk-README-SECRET")
            .unwrap();
    }
    assert!(cmd.wait_with_output().unwrap().status.success());

    let (ok, out, err) = run(
        home,
        &["run", "node", "-e", "console.log(process.env.OPENAI_API_KEY)"],
    );
    if !ok && err.contains("failed to start") {
        return; // no node on this machine
    }
    assert!(ok, "{err}");
    assert_eq!(
        out.trim(),
        "undefined",
        "the README promises `undefined` here: {out}"
    );
}
