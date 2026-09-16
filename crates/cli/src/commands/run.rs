use std::process::{Command, Stdio};

use anyhow::Context;

use crate::commands::Ctx;
use crate::i18n::tr;

/// `envenb run <cmd...>`: start a command with the environment's PUBLIC
/// variables, and nothing else.
///
/// Secrets are deliberately absent. A value handed to a child process can be
/// read back by any code running in it — `process.env.API_KEY`, `os.environ`,
/// a stray `console.log(process.env)` — which is exactly the exposure EnvEnb
/// exists to remove. Applications reach providers through the local proxy
/// instead (`envenb session`), so the credential never leaves the daemon.
pub async fn run(ctx: &Ctx, argv: Vec<String>) -> anyhow::Result<()> {
    let project = ctx.current_project().await?;
    let env = ctx.current_environment().await?;
    let (program, rest) = argv.split_first().context("empty command")?;

    let public = ctx.app.resolve_public_env(&env.id).await?;
    let secret_count = ctx
        .app
        .list_variables(&env.id)
        .await?
        .iter()
        .filter(|v| v.kind == envenb_core::VariableKind::Secret)
        .count();

    if !ctx.json {
        eprintln!(
            "  EnvEnb · {} / {} · {} {}",
            project.name,
            env.name,
            public.len(),
            tr("public variables", "PUBLIC 変数")
        );
        if secret_count > 0 {
            eprintln!(
                "  {} {secret_count} {}",
                tr("Secrets are not injected:", "Secret は注入されません:"),
                tr(
                    "kept in the vault. Use `envenb session` and call through the proxy.",
                    "Vault に残ります。`envenb session` と Proxy 経由で利用してください。"
                )
            );
        }
    }

    let mut cmd = Command::new(program);
    cmd.args(rest)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .env("ENVENB_PROJECT", &project.name)
        .env("ENVENB_ENVIRONMENT", &env.name);
    for (name, value) in public {
        cmd.env(name, value);
    }

    let status = cmd
        .status()
        .with_context(|| format!("{} {program}", tr("failed to start", "起動に失敗しました:")))?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
