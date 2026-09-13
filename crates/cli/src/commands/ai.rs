use envfish_core::{Action, ApprovalStatus, Decision, PermissionScope};

use crate::cli::AiCommand;
use crate::commands::Ctx;
use crate::i18n::tr;
use crate::output;

pub async fn run(ctx: &Ctx, command: AiCommand) -> anyhow::Result<()> {
    match command {
        AiCommand::Clients => {
            let clients = ctx.app.list_ai_clients().await?;
            if ctx.json {
                return output::print_json(&clients);
            }
            let rows: Vec<Vec<String>> = clients
                .iter()
                .map(|c| {
                    vec![
                        c.name.clone(),
                        c.kind.clone(),
                        c.last_seen_at
                            .map(|t| t.to_rfc3339())
                            .unwrap_or_else(|| "-".into()),
                        c.id[..8].to_string(),
                    ]
                })
                .collect();
            output::print_table(
                &[
                    tr("NAME", "名前"),
                    tr("KIND", "種別"),
                    tr("LAST SEEN", "最終接続"),
                    "ID",
                ],
                &rows,
            );
            Ok(())
        }
        AiCommand::Register { name, kind } => {
            let c = ctx.app.register_ai_client(&name, &kind).await?;
            if ctx.json {
                return output::print_json(&c);
            }
            println!(
                "{} {} ({})",
                tr("Registered client", "クライアントを登録しました:"),
                c.name,
                c.id
            );
            Ok(())
        }
        AiCommand::Permit {
            action,
            decision,
            client,
            connection,
            all_environments,
        } => {
            let project = ctx.current_project().await?;
            let action: Action = action.parse().map_err(anyhow::Error::msg)?;
            let decision: Decision = decision.parse().map_err(anyhow::Error::msg)?;
            let env = if all_environments {
                None
            } else {
                Some(ctx.current_environment().await?)
            };
            let client_id = match client {
                Some(c) => Some(ctx.app.resolve_ai_client(&c).await?.id),
                None => None,
            };
            let connection_id = match (&connection, &env) {
                (Some(c), Some(e)) => Some(ctx.app.resolve_connection(&e.id, c).await?.id),
                (Some(_), None) => anyhow::bail!(
                    "{}",
                    tr(
                        "--connection requires a selected environment (omit --all-environments)",
                        "--connection には環境の選択が必要です (--all-environments と併用できません)"
                    )
                ),
                (None, _) => None,
            };
            let rule = ctx
                .app
                .set_permission(
                    PermissionScope {
                        client_id,
                        project_id: Some(project.id.clone()),
                        environment_id: env.as_ref().map(|e| e.id.clone()),
                        connection_id,
                    },
                    action,
                    decision,
                )
                .await?;
            if ctx.json {
                return output::print_json(&rule);
            }
            println!(
                "{} {} → {} ({})",
                tr("Rule set:", "ルールを設定しました:"),
                action,
                decision,
                rule.id
            );
            Ok(())
        }
        AiCommand::Rules => {
            let rules = ctx.app.list_permissions().await?;
            if ctx.json {
                return output::print_json(&rules);
            }
            let short = |v: &Option<String>| {
                v.as_deref()
                    .map(|s| s.chars().take(8).collect())
                    .unwrap_or_else(|| "*".to_string())
            };
            let rows: Vec<Vec<String>> = rules
                .iter()
                .map(|r| {
                    vec![
                        r.id[..8].to_string(),
                        short(&r.client_id),
                        short(&r.project_id),
                        short(&r.environment_id),
                        short(&r.connection_id),
                        r.action.to_string(),
                        r.decision.to_string(),
                    ]
                })
                .collect();
            output::print_table(
                &[
                    "ID",
                    tr("CLIENT", "クライアント"),
                    tr("PROJECT", "PJ"),
                    tr("ENV", "環境"),
                    tr("CONN", "接続"),
                    tr("ACTION", "操作"),
                    tr("DECISION", "判定"),
                ],
                &rows,
            );
            Ok(())
        }
        AiCommand::Unpermit { rule_id } => {
            let rules = ctx.app.list_permissions().await?;
            let matched: Vec<_> = rules.iter().filter(|r| r.id.starts_with(&rule_id)).collect();
            match matched.as_slice() {
                [one] => {
                    ctx.app.delete_permission(&one.id).await?;
                    if !ctx.json {
                        println!("{} {}", tr("Removed rule", "ルールを削除しました:"), one.id);
                    }
                    Ok(())
                }
                [] => anyhow::bail!("{} {rule_id}", tr("rule not found:", "ルールが見つかりません:")),
                _ => anyhow::bail!(
                    "{} {rule_id}",
                    tr(
                        "ambiguous rule id prefix:",
                        "ルール id の先頭が一意ではありません:"
                    )
                ),
            }
        }
        AiCommand::Check { client, connection } => {
            let project = ctx.current_project().await?;
            let env = ctx.current_environment().await?;
            let client_id = match client {
                Some(c) => Some(ctx.app.resolve_ai_client(&c).await?.id),
                None => None,
            };
            let connection_id = match connection {
                Some(c) => Some(ctx.app.resolve_connection(&env.id, &c).await?.id),
                None => None,
            };
            let scope = PermissionScope {
                client_id,
                project_id: Some(project.id.clone()),
                environment_id: Some(env.id.clone()),
                connection_id,
            };
            let mut rows = Vec::new();
            let mut json = serde_json::Map::new();
            for action in Action::ALL {
                let d = ctx.app.decide(&scope, action).await?;
                rows.push(vec![action.to_string(), d.to_string()]);
                json.insert(action.to_string(), serde_json::Value::String(d.to_string()));
            }
            if ctx.json {
                return output::print_json(&json);
            }
            println!("{} / {}", project.name, env.name);
            output::print_table(&[tr("ACTION", "操作"), tr("DECISION", "判定")], &rows);
            Ok(())
        }
        AiCommand::Approvals { all } => {
            let status = if all { None } else { Some(ApprovalStatus::Pending) };
            let approvals = ctx.app.list_approvals(status, 100).await?;
            if ctx.json {
                return output::print_json(&approvals);
            }
            let rows: Vec<Vec<String>> = approvals
                .iter()
                .map(|a| {
                    vec![
                        a.id[..8].to_string(),
                        a.client_name.clone(),
                        a.action.to_string(),
                        a.summary.clone(),
                        a.status.as_str().to_string(),
                        a.created_at.format("%H:%M:%S").to_string(),
                    ]
                })
                .collect();
            output::print_table(
                &[
                    "ID",
                    tr("CLIENT", "クライアント"),
                    tr("ACTION", "操作"),
                    tr("REQUEST", "要求"),
                    tr("STATUS", "状態"),
                    tr("TIME", "時刻"),
                ],
                &rows,
            );
            if !approvals.is_empty() && !all {
                println!();
                println!(
                    "{}",
                    tr(
                        "envfish ai approve <ID> | envfish ai deny <ID>",
                        "envfish ai approve <ID> | envfish ai deny <ID>"
                    )
                );
            }
            Ok(())
        }
        AiCommand::Approve { id } => {
            let a = ctx.app.resolve_approval(&id, true).await?;
            if ctx.json {
                return output::print_json(&a);
            }
            println!(
                "{} {} ({} {})",
                tr("Approved:", "承認しました:"),
                a.summary,
                tr("for", "対象:"),
                a.client_name
            );
            Ok(())
        }
        AiCommand::Deny { id } => {
            let a = ctx.app.resolve_approval(&id, false).await?;
            if ctx.json {
                return output::print_json(&a);
            }
            println!(
                "{} {} ({} {})",
                tr("Denied:", "拒否しました:"),
                a.summary,
                tr("for", "対象:"),
                a.client_name
            );
            Ok(())
        }
    }
}

pub async fn activity(ctx: &Ctx, limit: i64) -> anyhow::Result<()> {
    let entries = ctx.app.list_audit(limit).await?;
    if ctx.json {
        return output::print_json(&entries);
    }
    let rows: Vec<Vec<String>> = entries
        .iter()
        .map(|e| {
            vec![
                e.created_at
                    .with_timezone(&chrono_local_offset())
                    .format("%m-%d %H:%M:%S")
                    .to_string(),
                e.client_name.clone(),
                format!(
                    "{}/{}",
                    e.project_name.clone().unwrap_or_else(|| "-".into()),
                    e.environment_name.clone().unwrap_or_else(|| "-".into())
                ),
                e.connection_name.clone().unwrap_or_else(|| "-".into()),
                e.action.to_string(),
                e.summary.clone(),
                e.decision.clone(),
            ]
        })
        .collect();
    output::print_table(
        &[
            tr("TIME", "時刻"),
            tr("CLIENT", "クライアント"),
            tr("PROJECT/ENV", "PJ/環境"),
            tr("CONN", "接続"),
            tr("ACTION", "操作"),
            tr("REQUEST", "要求"),
            tr("RESULT", "結果"),
        ],
        &rows,
    );
    Ok(())
}

fn chrono_local_offset() -> chrono::FixedOffset {
    use chrono::Offset;
    chrono::Local::now().offset().fix()
}
