use std::process::{Command, Stdio};

use anyhow::Context;

use crate::commands::Ctx;
use crate::i18n::tr;

/// `envfish run <cmd...>`: decrypt the current environment and inject it into the
/// child process only. This process's own environment is left untouched, and the
/// values never reach stdout/stderr.
pub async fn run(ctx: &Ctx, argv: Vec<String>) -> anyhow::Result<()> {
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

    let status = cmd
        .status()
        .with_context(|| format!("{} {program}", tr("failed to start", "起動に失敗しました:")))?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
