use crate::cli::ProjectCommand;
use crate::commands::Ctx;
use crate::i18n::tr;
use crate::output;

pub async fn run(ctx: &mut Ctx, command: ProjectCommand) -> anyhow::Result<()> {
    match command {
        ProjectCommand::List => {
            let projects = ctx.app.list_projects().await?;
            if ctx.json {
                return output::print_json(&projects);
            }
            let current = ctx.state.current_project_id.as_deref();
            let rows: Vec<Vec<String>> = projects
                .iter()
                .map(|p| {
                    vec![
                        if Some(p.id.as_str()) == current {
                            "*".into()
                        } else {
                            String::new()
                        },
                        p.name.clone(),
                        p.local_path.clone().unwrap_or_default(),
                        p.id.clone(),
                    ]
                })
                .collect();
            output::print_table(&["", tr("NAME", "名前"), tr("PATH", "パス"), "ID"], &rows);
            Ok(())
        }
        ProjectCommand::Add { name, path } => {
            let project = ctx.app.create_project(&name, path.as_deref()).await?;
            if ctx.json {
                return output::print_json(&project);
            }
            println!(
                "{} {} ({})",
                tr("Registered project", "プロジェクトを登録しました:"),
                project.name,
                project.id
            );
            if ctx.state.current_project_id.is_none() {
                ctx.state.current_project_id = Some(project.id.clone());
                ctx.save_state()?;
                println!(
                    "{}",
                    tr(
                        "Selected as the current project.",
                        "現在のプロジェクトとして選択しました。"
                    )
                );
            }
            Ok(())
        }
        ProjectCommand::Remove { project } => {
            let p = ctx.app.resolve_project(&project).await?;
            ctx.app.delete_project(&p.id).await?;
            if ctx.state.current_project_id.as_deref() == Some(p.id.as_str()) {
                ctx.state.current_project_id = None;
                ctx.state.current_environment_id = None;
                ctx.save_state()?;
            }
            if !ctx.json {
                println!(
                    "{} {}",
                    tr("Removed project", "プロジェクトを削除しました:"),
                    p.name
                );
            }
            Ok(())
        }
    }
}

pub async fn use_project(ctx: &mut Ctx, project: &str) -> anyhow::Result<()> {
    let p = ctx.app.resolve_project(project).await?;
    let changed = ctx.state.current_project_id.as_deref() != Some(p.id.as_str());
    ctx.state.current_project_id = Some(p.id.clone());
    if changed {
        ctx.state.current_environment_id = None;
    }
    ctx.save_state()?;
    if ctx.json {
        return output::print_json(&p);
    }
    println!("{} {}", tr("Project:", "プロジェクト:"), p.name);
    if changed {
        println!(
            "{} {}",
            tr("Environment:", "環境:"),
            tr(
                "(none) — select one with `envenb env <name>`",
                "(未選択) — `envenb env <名前>` で選択してください"
            )
        );
    }
    Ok(())
}
