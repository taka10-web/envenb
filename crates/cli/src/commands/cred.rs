use std::io::{IsTerminal, Read, Write};
use std::process::{Command, Stdio};

use anyhow::Context;
use envenb_core::{CredentialFieldInput, CredentialKind, NewCredential, SecretValue};

use crate::cli::CredCommand;
use crate::commands::Ctx;
use crate::i18n::tr;
use crate::output;

pub async fn run(ctx: &Ctx, command: CredCommand) -> anyhow::Result<()> {
    let project = ctx.current_project().await?;
    let env = ctx.current_environment().await?;
    match command {
        CredCommand::List => {
            let creds = ctx.app.list_credentials(&env.id).await?;
            if ctx.json {
                return output::print_json(&creds);
            }
            println!("{} / {}", project.name, env.name);
            let rows: Vec<Vec<String>> = creds
                .iter()
                .map(|c| {
                    let summary = c
                        .fields
                        .iter()
                        .filter(|f| !f.secret)
                        .map(|f| format!("{}={}", f.field, f.value.clone().unwrap_or_default()))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let secrets = c
                        .fields
                        .iter()
                        .filter(|f| f.secret)
                        .map(|f| f.field.as_str())
                        .collect::<Vec<_>>()
                        .join(",");
                    vec![
                        c.name.clone(),
                        c.kind.to_string(),
                        summary,
                        secrets,
                        c.note.clone().unwrap_or_default(),
                    ]
                })
                .collect();
            output::print_table(
                &[
                    tr("NAME", "名前"),
                    tr("KIND", "種別"),
                    tr("DETAILS", "詳細"),
                    tr("SECRET FIELDS", "Secret フィールド"),
                    tr("NOTE", "メモ"),
                ],
                &rows,
            );
            Ok(())
        }
        CredCommand::Add {
            name,
            kind,
            fields,
            note,
        } => {
            let kind: CredentialKind = kind.parse().map_err(anyhow::Error::msg)?;
            let mut inputs = parse_fields(&fields)?;
            // Prompt for required fields not supplied (hidden input for secrets) when interactive.
            if std::io::stdin().is_terminal() && !ctx.json {
                for spec in kind.fields() {
                    if inputs.iter().any(|i| i.field == spec.name) {
                        continue;
                    }
                    if !spec.required && !spec.secret {
                        continue;
                    }
                    if spec.multiline {
                        eprintln!(
                            "{} {} {}",
                            tr("Field", "フィールド"),
                            spec.name,
                            tr(
                                "is multi-line: pass it with --field NAME=@path",
                                "は複数行です。--field 名=@パス で指定してください"
                            )
                        );
                        continue;
                    }
                    let label = format!(
                        "{}{}",
                        spec.name,
                        if spec.required {
                            ""
                        } else {
                            tr(" (optional)", " (任意)")
                        }
                    );
                    let value = if spec.secret {
                        prompt_hidden(&label)?
                    } else {
                        prompt_plain(&label)?
                    };
                    if !value.is_empty() {
                        inputs.push(CredentialFieldInput {
                            field: spec.name.to_string(),
                            value: SecretValue::new(value),
                        });
                    }
                }
            }
            let c = ctx
                .app
                .create_credential(NewCredential {
                    environment_id: env.id.clone(),
                    kind,
                    name,
                    note,
                    fields: inputs,
                })
                .await?;
            if ctx.json {
                return output::print_json(&c);
            }
            println!(
                "{} {} ({})",
                tr("Stored credential", "資格情報を保存しました:"),
                c.name,
                c.kind
            );
            Ok(())
        }
        CredCommand::Show { name } => {
            let c = ctx.app.resolve_credential(&env.id, &name).await?;
            if ctx.json {
                return output::print_json(&c);
            }
            println!("{} [{}]", c.name, c.kind);
            if let Some(n) = &c.note {
                println!("  {} {n}", tr("note:", "メモ:"));
            }
            for f in &c.fields {
                if f.secret {
                    println!(
                        "  {:<14} ••••••••  ({})",
                        f.field,
                        tr("copy: envenb cred copy", "コピー: envenb cred copy")
                    );
                } else {
                    println!("  {:<14} {}", f.field, f.value.clone().unwrap_or_default());
                }
            }
            Ok(())
        }
        CredCommand::Set { name, fields } => {
            let c = ctx.app.resolve_credential(&env.id, &name).await?;
            let inputs = parse_fields(&fields)?;
            let updated = ctx.app.update_credential_fields(&c.id, inputs, None).await?;
            if ctx.json {
                return output::print_json(&updated);
            }
            println!("{} {}", tr("Updated", "更新しました:"), updated.name);
            Ok(())
        }
        CredCommand::Copy { name, field } => {
            crate::human::require_human("envenb cred copy")?;
            if !std::io::stdout().is_terminal() {
                anyhow::bail!(
                    "{}",
                    tr(
                        "cred copy is only available from an interactive terminal",
                        "cred copy は対話端末からのみ実行できます"
                    )
                );
            }
            let c = ctx.app.resolve_credential(&env.id, &name).await?;
            let ttl = envenb_core::clipboard::DEFAULT_TTL;
            if field == "totp" {
                let code = ctx.app.credential_totp(&c.id).await?;
                envenb_core::clipboard::copy_then_clear(&code, ttl).map_err(anyhow::Error::msg)?;
            } else {
                if !c.fields.iter().any(|f| f.field == field && f.secret) {
                    anyhow::bail!(
                        "{} {field}",
                        tr(
                            "no such secret field:",
                            "そのような Secret フィールドはありません:"
                        )
                    );
                }
                ctx.app
                    .with_credential_field(&c.id, &field, |v| envenb_core::clipboard::copy_then_clear(v, ttl))
                    .await?
                    .map_err(anyhow::Error::msg)?;
            }
            // Keep the process alive so the clearing thread can do its job.
            eprintln!(
                "{} {}.{} {} {}s",
                tr("Copied", "コピーしました:"),
                c.name,
                field,
                tr(
                    "to the clipboard; it will be cleared in",
                    "クリップボードは次の秒数後に消去されます:"
                ),
                ttl.as_secs()
            );
            std::thread::sleep(ttl + std::time::Duration::from_millis(200));
            Ok(())
        }
        CredCommand::Remove { name } => {
            let c = ctx.app.resolve_credential(&env.id, &name).await?;
            ctx.app.delete_credential(&c.id).await?;
            if !ctx.json {
                println!("{} {}", tr("Removed", "削除しました:"), c.name);
            }
            Ok(())
        }
    }
}

/// `--field NAME=VALUE` where VALUE may be `@path` or `-`.
fn parse_fields(raw: &[String]) -> anyhow::Result<Vec<CredentialFieldInput>> {
    let mut out = Vec::new();
    for pair in raw {
        let (k, v) = pair.split_once('=').ok_or_else(|| {
            anyhow::anyhow!(
                "{} {pair}",
                tr("--field expects NAME=VALUE, got", "--field は 名=値 の形式です:")
            )
        })?;
        let value = if v == "-" {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf.trim_end_matches(['\r', '\n']).to_string()
        } else if let Some(path) = v.strip_prefix('@') {
            std::fs::read_to_string(path)
                .with_context(|| format!("{} {path}", tr("cannot read", "読み取れません:")))?
        } else {
            v.to_string()
        };
        out.push(CredentialFieldInput {
            field: k.trim().to_string(),
            value: SecretValue::new(value),
        });
    }
    Ok(out)
}

fn prompt_plain(label: &str) -> anyhow::Result<String> {
    eprint!("{label}: ");
    std::io::stderr().flush().ok();
    let mut s = String::new();
    std::io::stdin().read_line(&mut s)?;
    Ok(s.trim().to_string())
}

fn prompt_hidden(label: &str) -> anyhow::Result<String> {
    eprint!("{label} {}: ", tr("(hidden)", "(非表示)"));
    std::io::stderr().flush().ok();
    crate::commands::var::read_line_no_echo()
}

/// `envenb ssh <name> [args...]`: write the key to a 0600 temp file, run ssh, delete the file.
pub async fn ssh(ctx: &Ctx, name: &str, extra: Vec<String>) -> anyhow::Result<()> {
    crate::human::require_human("envenb ssh")?;
    let env = ctx.current_environment().await?;
    let c = ctx.app.resolve_credential(&env.id, name).await?;
    if c.kind != CredentialKind::Ssh {
        anyhow::bail!(
            "{} {}",
            name,
            tr("is not an ssh credential", "は ssh 資格情報ではありません")
        );
    }
    let get = |f: &str| {
        c.fields
            .iter()
            .find(|x| x.field == f)
            .and_then(|x| x.value.clone())
    };
    let host = get("host").context("ssh credential has no host")?;
    let user = get("user").context("ssh credential has no user")?;
    let port = get("port");
    let has_key = c.fields.iter().any(|f| f.field == "private_key" && f.secret);

    let dir = tempfile::Builder::new().prefix("envenb-ssh-").tempdir()?;
    let mut cmd = Command::new("ssh");
    if let Some(p) = port {
        cmd.arg("-p").arg(p);
    }
    if has_key {
        let key_path = dir.path().join("id_envenb");
        ctx.app
            .with_credential_field(&c.id, "private_key", |key| write_private(&key_path, key))
            .await??;
        cmd.arg("-i").arg(&key_path).arg("-o").arg("IdentitiesOnly=yes");
    }
    cmd.arg(format!("{user}@{host}"));
    cmd.args(extra);
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if !ctx.json {
        eprintln!("  EnvEnb · ssh {user}@{host} ({})", c.name);
    }
    let status = cmd.status().context("failed to start ssh")?;
    drop(dir); // removes the key file
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

pub fn write_private(path: &std::path::Path, content: &str) -> anyhow::Result<()> {
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(content.as_bytes())?;
    if !content.ends_with('\n') {
        f.write_all(b"\n")?;
    }
    Ok(())
}
