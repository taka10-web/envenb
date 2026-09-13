use crate::cli::EnvArgs;
use crate::commands::Ctx;
use crate::i18n::tr;
use crate::output;

pub async fn run(ctx: &mut Ctx, args: EnvArgs) -> anyhow::Result<()> {
    let project = ctx.current_project().await?;

    let Some(target) = args.environment else {
        let envs = ctx.app.list_environments(&project.id).await?;
        if ctx.json {
            return output::print_json(&envs);
        }
        let current = ctx.state.current_environment_id.as_deref();
        let rows: Vec<Vec<String>> = envs
            .iter()
            .map(|e| {
                vec![
                    if Some(e.id.as_str()) == current {
                        "*".into()
                    } else {
                        String::new()
                    },
                    e.name.clone(),
                    e.id.clone(),
                ]
            })
            .collect();
        println!("{} {}", tr("Project:", "プロジェクト:"), project.name);
        output::print_table(&["", tr("ENVIRONMENT", "環境"), "ID"], &rows);
        return Ok(());
    };

    let env = match ctx.app.resolve_environment(&project.id, &target).await {
        Ok(e) => e,
        Err(envfish_core::CoreError::EnvironmentNotFound(_)) if args.create => {
            let e = ctx.app.create_environment(&project.id, &target).await?;
            if !ctx.json {
                println!("{} {}", tr("Created environment", "環境を作成しました:"), e.name);
            }
            e
        }
        Err(err) => return Err(err.into()),
    };

    ctx.state.current_environment_id = Some(env.id.clone());
    ctx.save_state()?;
    if ctx.json {
        return output::print_json(&env);
    }
    println!("{} {}", tr("Project:", "プロジェクト:"), project.name);
    println!("{} {}", tr("Environment:", "環境:"), env.name);
    Ok(())
}
