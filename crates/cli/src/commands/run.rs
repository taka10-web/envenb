use std::process::{Command, Stdio};

use anyhow::Context;

use crate::commands::Ctx;
use crate::i18n::tr;

/// `envfish run <cmd...>`: decrypt the current environment and inject it into the
/// child process only. This process's own environment is left untouched, and the
/// values never reach stdout/stderr.
pub async fn run(ctx: &Ctx, argv: Vec<String>, with_credentials: bool) -> anyhow::Result<()> {
    crate::human::require_human("envfish run")?;
    let project = ctx.current_project().await?;
    let env = ctx.current_environment().await?;
    let (program, rest) = argv.split_first().context("empty command")?;

    let process_env = ctx.app.resolve_process_env(&env.id).await?;
    if !ctx.json {
        eprintln!(
            "  EnvFish · {} / {} · {} {} · {} {}",
            project.name,
            env.name,
            process_env.public_count(),
            tr("public", "PUBLIC"),
            process_env.secret_count(),
            tr("secrets injected", "SECRET を注入")
        );
    }

    let mut cmd = Command::new(program);
    cmd.args(rest)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .env("ENVFISH_PROJECT", &project.name)
        .env("ENVFISH_ENVIRONMENT", &env.name);
    process_env.apply_to(&mut cmd);
    drop(process_env);

    // Optional: credentials as env vars / 0600 temp files, removed when we exit.
    let _tempdir = if with_credentials {
        let dir = tempfile::Builder::new().prefix("envfish-run-").tempdir()?;
        for c in ctx.app.list_credentials(&env.id).await? {
            let base = format!("ENVFISH_CRED_{}", sanitize(&c.name));
            for f in &c.fields {
                if c.kind == envfish_core::CredentialKind::File && f.field == "content" {
                    let filename = c
                        .fields
                        .iter()
                        .find(|x| x.field == "filename")
                        .and_then(|x| x.value.clone())
                        .unwrap_or_else(|| c.name.clone());
                    let path = dir.path().join(filename);
                    ctx.app
                        .with_credential_field(&c.id, "content", |v| super::cred::write_private(&path, v))
                        .await??;
                    cmd.env(format!("ENVFISH_FILE_{}", sanitize(&c.name)), &path);
                } else if f.secret {
                    let value = ctx
                        .app
                        .with_credential_field(&c.id, &f.field, |v| v.to_string())
                        .await?;
                    cmd.env(format!("{base}_{}", sanitize(&f.field)), value);
                } else if let Some(v) = &f.value {
                    cmd.env(format!("{base}_{}", sanitize(&f.field)), v);
                }
            }
        }
        Some(dir)
    } else {
        None
    };

    let status = cmd
        .status()
        .with_context(|| format!("{} {program}", tr("failed to start", "起動に失敗しました:")))?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}
