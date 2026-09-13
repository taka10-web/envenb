use std::io::{IsTerminal, Write};

use anyhow::Context;
use envfish_core::VariableKind;
use envfish_core::dotenv::{Suggestion, classify, parse};

use crate::commands::Ctx;
use crate::i18n::tr;
use crate::output;

pub async fn import(ctx: &Ctx, file: &str, yes: bool, dry_run: bool) -> anyhow::Result<()> {
    let project = ctx.current_project().await?;
    let env = ctx.current_environment().await?;
    let text = std::fs::read_to_string(file)
        .with_context(|| format!("{} {file}", tr("cannot read", "読み取れません:")))?;
    let (entries, invalid) = parse(&text);
    if entries.is_empty() {
        anyhow::bail!(
            "{}",
            tr("no variables found in the file", "ファイルに変数がありません")
        );
    }

    let mut plan: Vec<(String, String, VariableKind)> = Vec::new();
    let interactive = !yes && std::io::stdin().is_terminal() && !ctx.json;
    if !ctx.json {
        println!(
            "{} {} → {} / {}",
            tr("Import", "取り込み:"),
            file,
            project.name,
            env.name
        );
        if !invalid.is_empty() {
            println!(
                "{} {:?}",
                tr("skipped invalid lines:", "不正な行をスキップ:"),
                invalid
            );
        }
        println!();
    }
    for e in &entries {
        let suggestion = classify(&e.name, &e.value);
        let default_kind = match suggestion {
            Suggestion::Public => VariableKind::Public,
            Suggestion::Secret | Suggestion::Review => VariableKind::Secret,
        };
        let kind = if interactive {
            ask_kind(&e.name, suggestion, default_kind)?
        } else {
            Some(default_kind)
        };
        if let Some(k) = kind {
            plan.push((e.name.clone(), e.value.clone(), k));
        }
    }

    if ctx.json || !interactive {
        // Non-interactive: show the plan as a table (no values for secrets).
        let rows: Vec<Vec<String>> = plan
            .iter()
            .map(|(n, v, k)| {
                vec![
                    n.clone(),
                    k.to_string(),
                    match k {
                        VariableKind::Public => v.clone(),
                        VariableKind::Secret => "••••••••".into(),
                    },
                ]
            })
            .collect();
        if !ctx.json {
            output::print_table(
                &[tr("NAME", "名前"), tr("KIND", "種別"), tr("VALUE", "値")],
                &rows,
            );
        }
    }

    if dry_run {
        if ctx.json {
            let view: Vec<_> = plan
                .iter()
                .map(|(n, _, k)| serde_json::json!({"name": n, "kind": k}))
                .collect();
            return output::print_json(&view);
        }
        println!("{}", tr("dry run: nothing stored", "dry-run: 保存していません"));
        return Ok(());
    }

    let report = ctx.app.import_variables(&env.id, plan).await?;
    if ctx.json {
        return output::print_json(&report);
    }
    println!(
        "{} PUBLIC {} / SECRET {}{}",
        tr("Imported:", "取り込みました:"),
        report.public_added,
        report.secret_added,
        if report.skipped.is_empty() {
            String::new()
        } else {
            format!(" / {} {}", tr("skipped", "スキップ"), report.skipped.len())
        }
    );
    for s in &report.skipped {
        println!("  - {s}");
    }
    println!();
    println!(
        "{}",
        tr(
            "Next: add the .env file to .gitignore and consider deleting it — EnvFish now holds these values.",
            "次に: .env を .gitignore に追加し、削除を検討してください。値は EnvFish が保持しています。"
        )
    );
    Ok(())
}

/// Ask the user to confirm the kind. Returns `None` to skip the variable.
fn ask_kind(
    name: &str,
    suggestion: Suggestion,
    default: VariableKind,
) -> anyhow::Result<Option<VariableKind>> {
    let hint = match suggestion {
        Suggestion::Public => tr("suggested: PUBLIC", "提案: PUBLIC"),
        Suggestion::Secret => tr("suggested: SECRET", "提案: SECRET"),
        Suggestion::Review => tr("suggested: SECRET (please review)", "提案: SECRET (要確認)"),
    };
    let default_letter = match default {
        VariableKind::Public => "P",
        VariableKind::Secret => "S",
    };
    eprint!(
        "{name}  [{hint}]  {} [{default_letter}]: ",
        tr("(P)ublic / (S)ecret / s(k)ip", "(P)ublic / (S)ecret / s(k)ip")
    );
    std::io::stderr().flush().ok();
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(match answer.trim().to_ascii_lowercase().as_str() {
        "" => Some(default),
        "p" | "public" => Some(VariableKind::Public),
        "s" | "secret" => Some(VariableKind::Secret),
        "k" | "skip" => None,
        _ => Some(default),
    })
}

pub async fn export_example(ctx: &Ctx, file: &str) -> anyhow::Result<()> {
    let env = ctx.current_environment().await?;
    let text = ctx.app.render_env_example(&env.id).await?;
    if file == "-" {
        print!("{text}");
        return Ok(());
    }
    std::fs::write(file, text)
        .with_context(|| format!("{} {file}", tr("cannot write", "書き込めません:")))?;
    if !ctx.json {
        println!("{} {file}", tr("wrote", "出力しました:"));
    }
    Ok(())
}
