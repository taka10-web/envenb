use envfish_core::{ConnectionKind, NewConnection};

use crate::cli::ConnectionCommand;
use crate::commands::Ctx;
use crate::i18n::tr;
use crate::output;

pub async fn run(ctx: &Ctx, command: ConnectionCommand) -> anyhow::Result<()> {
    let project = ctx.current_project().await?;
    let env = ctx.current_environment().await?;
    match command {
        ConnectionCommand::List => {
            let conns = ctx.app.list_connections(&env.id).await?;
            if ctx.json {
                return output::print_json(&conns);
            }
            println!("{} / {}", project.name, env.name);
            let rows: Vec<Vec<String>> = conns
                .iter()
                .map(|c| {
                    vec![
                        c.name.clone(),
                        c.kind.to_string(),
                        c.base_url.clone(),
                        c.auth_secret.clone().unwrap_or_else(|| "-".into()),
                        c.auth_style.clone(),
                        c.id[..8].to_string(),
                    ]
                })
                .collect();
            output::print_table(
                &[
                    tr("NAME", "名前"),
                    tr("KIND", "種別"),
                    "URL",
                    tr("CREDENTIAL", "認証情報 (SECRET 名)"),
                    tr("AUTH", "方式"),
                    "ID",
                ],
                &rows,
            );
            Ok(())
        }
        ConnectionCommand::Add {
            name,
            kind,
            url,
            secret,
            auth,
            meta,
        } => {
            let kind: ConnectionKind = kind.parse().map_err(anyhow::Error::msg)?;
            let mut metadata = serde_json::Map::new();
            for pair in meta {
                let (k, v) = pair.split_once('=').ok_or_else(|| {
                    anyhow::anyhow!(
                        "{} {pair}",
                        tr("--meta expects KEY=VALUE, got", "--meta は KEY=VALUE 形式です:")
                    )
                })?;
                metadata.insert(
                    k.trim().to_string(),
                    serde_json::Value::String(v.trim().to_string()),
                );
            }
            let metadata = if metadata.is_empty() {
                None
            } else {
                Some(serde_json::Value::Object(metadata))
            };
            let conn = ctx
                .app
                .create_connection(NewConnection {
                    environment_id: env.id.clone(),
                    kind,
                    name,
                    base_url: url,
                    auth_secret: secret,
                    auth_style: auth,
                    metadata,
                })
                .await?;
            if ctx.json {
                return output::print_json(&conn);
            }
            println!(
                "{} {} ({} → {})",
                tr("Added connection", "接続を追加しました:"),
                conn.name,
                conn.kind,
                conn.base_url
            );
            Ok(())
        }
        ConnectionCommand::Remove { connection } => {
            let c = ctx.app.resolve_connection(&env.id, &connection).await?;
            ctx.app.delete_connection(&c.id).await?;
            if !ctx.json {
                println!("{} {}", tr("Removed connection", "接続を削除しました:"), c.name);
            }
            Ok(())
        }
    }
}
