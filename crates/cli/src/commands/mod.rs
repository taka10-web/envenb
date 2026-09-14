mod agent;
mod ai;
mod config;
mod connection;
mod cred;
mod dotenv;
mod env;
mod mcp;
mod project;
mod run;
mod scan;
mod status;
pub(crate) mod var;
mod vault;

use anyhow::Context;
use envfish_core::{CliState, EnvFish, Environment, Project};

use crate::cli::{Cli, Command, EnvArgs, ProjectCommand};
use crate::fish;
use crate::i18n::tr;

/// Shared per-invocation context.
pub struct Ctx {
    pub app: EnvFish,
    pub state: CliState,
    pub json: bool,
}

impl Ctx {
    pub fn save_state(&self) -> anyhow::Result<()> {
        self.state.save(&self.app.paths().cli_state()).context(tr(
            "failed to save CLI state",
            "CLI の状態ファイルを保存できませんでした",
        ))
    }

    /// The selected project, or an error that names the next command to run.
    pub async fn current_project(&self) -> anyhow::Result<Project> {
        match self.state.current_project_id.as_deref() {
            Some(id) => Ok(self.app.get_project(id).await?),
            None => Err(self.no_project_hint().await),
        }
    }

    /// The selected environment, or an error that names the next command to run.
    pub async fn current_environment(&self) -> anyhow::Result<Environment> {
        match self.state.current_environment_id.as_deref() {
            Some(id) => Ok(self.app.get_environment(id).await?),
            None => Err(self.no_environment_hint().await),
        }
    }

    /// "Nothing is selected" is the most common first-run error, so the message
    /// lists the actual projects instead of the generic instruction.
    async fn no_project_hint(&self) -> anyhow::Error {
        let projects = self.app.list_projects().await.unwrap_or_default();
        if projects.is_empty() {
            return anyhow::anyhow!(
                "{}\n  envfish project add <name> --path .",
                tr(
                    "no projects yet. Register one first:",
                    "プロジェクトがまだありません。まず登録してください:"
                )
            );
        }
        let names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).take(5).collect();
        anyhow::anyhow!(
            "{}\n  envfish use {}\n  ({} {})",
            tr(
                "no project selected. Select one:",
                "プロジェクトが未選択です。選択してください:"
            ),
            names[0],
            tr("available:", "登録済み:"),
            names.join(", ")
        )
    }

    async fn no_environment_hint(&self) -> anyhow::Error {
        let Ok(project) = self.current_project().await else {
            return self.no_project_hint().await;
        };
        let envs = self.app.list_environments(&project.id).await.unwrap_or_default();
        if envs.is_empty() {
            return anyhow::anyhow!(
                "{} {}\n  envfish env development --create",
                tr("no environments in", "環境がありません:"),
                project.name
            );
        }
        let names: Vec<&str> = envs.iter().map(|e| e.name.as_str()).take(5).collect();
        anyhow::anyhow!(
            "{}\n  envfish env {}\n  ({} {})",
            tr(
                "no environment selected. Select one:",
                "環境が未選択です。選択してください:"
            ),
            names[0],
            tr("available:", "登録済み:"),
            names.join(", ")
        )
    }
}

pub async fn run(args: Cli, core: envfish_core::Result<EnvFish>) -> anyhow::Result<()> {
    let command = args.command.expect("subcommand presence is checked in main");

    let app = core.context(tr(
        "failed to open EnvFish data directory",
        "EnvFish のデータディレクトリを開けませんでした",
    ))?;
    let state = CliState::load(&app.paths().cli_state()).context(tr(
        "failed to load CLI state",
        "CLI の状態ファイルを読み込めませんでした",
    ))?;
    let mut ctx = Ctx {
        app,
        state,
        json: args.json,
    };

    // The splash marks "milestone" moments only (see design §15.1):
    // status, project registration, project / environment switch.
    let milestone = matches!(
        command,
        Command::Status
            | Command::Use { .. }
            | Command::Project {
                command: ProjectCommand::Add { .. }
            }
            | Command::Env(EnvArgs {
                environment: Some(_),
                ..
            })
    );
    if milestone && fish::should_animate(args.no_animation, args.json) {
        fish::splash();
    }

    match command {
        Command::Project { command } => project::run(&mut ctx, command).await,
        Command::Status => status::run(&ctx).await,
        Command::Use { project } => project::use_project(&mut ctx, &project).await,
        Command::Env(env_args) => env::run(&mut ctx, env_args).await,
        Command::Var { command } => var::run(&ctx, command).await,
        Command::Run {
            command,
            with_credentials,
        } => run::run(&ctx, command, with_credentials).await,
        Command::Cred { command } => cred::run(&ctx, command).await,
        Command::Ssh { name, args } => cred::ssh(&ctx, &name, args).await,
        Command::Import {
            file,
            yes,
            dry_run,
            no_gitignore,
            delete,
        } => dotenv::import(&ctx, &file, yes, dry_run, !no_gitignore, delete).await,
        Command::Clean { dir, dry_run, yes } => dotenv::clean(&ctx, dir.as_deref(), dry_run, yes).await,
        Command::ExportExample { file } => dotenv::export_example(&ctx, &file).await,
        Command::ExportEnv {
            file,
            force,
            no_gitignore,
        } => dotenv::export_env(&ctx, &file, force, !no_gitignore).await,
        Command::Connection { command } => connection::run(&ctx, command).await,
        Command::Ai { command } => ai::run(&ctx, command).await,
        Command::Activity { limit } => ai::activity(&ctx, limit).await,
        Command::Mcp { client, kind } => mcp::run(ctx, &client, &kind).await,
        Command::Agent { ping } => agent::run(ctx, ping).await,
        Command::Scan {
            dir,
            all_environments,
        } => scan::run(&ctx, &dir, all_environments).await,
        Command::Config { command } => config::run(&ctx, command).await,
        Command::Vault { command } => vault::run(&ctx, command).await,
    }
}
