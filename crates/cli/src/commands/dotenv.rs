use std::io::{IsTerminal, Write};

use anyhow::Context;
use envfish_core::VariableKind;
use envfish_core::dotenv::{Suggestion, classify, parse};

use crate::commands::Ctx;
use crate::i18n::tr;
use crate::output;

pub async fn import(
    ctx: &Ctx,
    file: &str,
    yes: bool,
    dry_run: bool,
    gitignore: bool,
    delete: bool,
) -> anyhow::Result<()> {
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
    if gitignore {
        report_gitignore(ctx, std::path::Path::new(file));
    }
    if delete {
        let known = known_names(ctx, &env.id).await?;
        let path = std::path::Path::new(file);
        if yes || confirm_delete(path)? {
            report_remove(ctx, envfish_core::dotenv::remove_if_covered(path, &known)?);
        } else if !ctx.json {
            println!("{}", tr("Kept the file.", "ファイルは残しました。"));
        }
    } else if !ctx.json {
        println!();
        println!(
            "{}",
            tr(
                "Delete the file with `envfish clean` (or `import --delete`) — EnvFish now holds these values.",
                "ファイルは `envfish clean` (または `import --delete`) で削除できます。値は EnvFish が保持しています。"
            )
        );
    }
    Ok(())
}

/// Add `file` to the enclosing repository's .gitignore and say what happened.
pub fn report_gitignore(ctx: &Ctx, file: &std::path::Path) {
    let abs = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    let Some(dir) = abs.parent() else { return };
    let Some(root) = envfish_core::dotenv::git_root(dir) else {
        if !ctx.json {
            println!(
                "{}",
                tr(
                    "(not inside a git repository; .gitignore untouched)",
                    "(git リポジトリ外のため .gitignore は変更していません)"
                )
            );
        }
        return;
    };
    let rel = abs
        .strip_prefix(&root)
        .unwrap_or(&abs)
        .to_string_lossy()
        .replace('\\', "/");
    let pattern = if rel.contains('/') { format!("/{rel}") } else { rel };
    match envfish_core::dotenv::ensure_gitignored(&root, &[&pattern]) {
        Ok(r) if !ctx.json => {
            if !r.added.is_empty() {
                println!(
                    "{} {} → {}",
                    tr("Added to .gitignore:", ".gitignore に追記しました:"),
                    r.added.join(", "),
                    r.path
                );
            } else {
                println!(
                    "{} {}",
                    tr(
                        ".gitignore already covers",
                        ".gitignore は既に対象を含んでいます:"
                    ),
                    r.already.join(", ")
                );
            }
        }
        Ok(_) => {}
        Err(e) if !ctx.json => println!(
            "{} {e}",
            tr(
                "could not update .gitignore:",
                ".gitignore を更新できませんでした:"
            )
        ),
        Err(_) => {}
    }
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

/// `envfish export-env [FILE]`: materialise the environment as a real dotenv file.
/// Human-initiated escape hatch for tools that only read files. Guard rails: the
/// file is created 0600, an existing file needs --force, and a path tracked by git
/// is refused outright.
pub async fn export_env(ctx: &Ctx, file: &str, force: bool, gitignore: bool) -> anyhow::Result<()> {
    let project = ctx.current_project().await?;
    let env = ctx.current_environment().await?;
    let path = std::path::Path::new(file);
    if path.exists() && !force {
        anyhow::bail!(
            "{} {file} ({})",
            tr("file exists:", "ファイルが既にあります:"),
            tr("use --force to overwrite", "上書きするには --force")
        );
    }
    if git_tracks(path) {
        anyhow::bail!(
            "{} {file}. {}",
            tr("git tracks", "git が追跡しています:"),
            tr(
                "Add it to .gitignore (git rm --cached) before exporting secrets into it",
                "Secret を書く前に .gitignore に追加してください (git rm --cached)"
            )
        );
    }
    let vars = ctx.app.list_variables(&env.id).await?;
    let process_env = ctx.app.resolve_process_env(&env.id).await?;
    let mut text = format!(
        "# Generated by EnvFish for {} / {} — contains secrets, keep out of git.\n",
        project.name, env.name
    );
    let mut cmd = std::process::Command::new("true");
    process_env.apply_to(&mut cmd);
    let injected: std::collections::HashMap<String, String> = cmd
        .get_envs()
        .filter_map(|(k, v)| {
            Some((
                k.to_string_lossy().into_owned(),
                v?.to_string_lossy().into_owned(),
            ))
        })
        .collect();
    for v in &vars {
        if let Some(value) = injected.get(&v.name) {
            text.push_str(&format!("{}={}\n", v.name, quote(value)));
        }
    }
    drop(process_env);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    super::cred::write_private(path, &text)?;
    if !ctx.json {
        println!(
            "{} {file} ({} {}, 0600)",
            tr("wrote", "出力しました:"),
            vars.len(),
            tr("variables", "変数")
        );
    }
    if gitignore {
        report_gitignore(ctx, path);
    }
    Ok(())
}

fn quote(value: &str) -> String {
    if value.is_empty()
        || value
            .chars()
            .any(|c| c.is_whitespace() || c == '#' || c == '"' || c == '\'' || c == '$')
    {
        format!(
            "\"{}\"",
            value
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
        )
    } else {
        value.to_string()
    }
}

fn git_tracks(path: &std::path::Path) -> bool {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    std::process::Command::new("git")
        .args(["ls-files", "--error-unmatch", "--", &name])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

async fn known_names(ctx: &Ctx, environment_id: &str) -> anyhow::Result<std::collections::HashSet<String>> {
    Ok(ctx
        .app
        .list_variables(environment_id)
        .await?
        .into_iter()
        .map(|v| v.name)
        .collect())
}

/// Ask before deleting. Non-interactive sessions answer "no" unless --yes was given.
fn confirm_delete(path: &std::path::Path) -> anyhow::Result<bool> {
    if !std::io::stdin().is_terminal() {
        return Ok(false);
    }
    eprint!("{} {} [y/N]: ", tr("Delete", "削除しますか:"), path.display());
    std::io::stderr().flush().ok();
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}

fn report_remove(ctx: &Ctx, outcome: envfish_core::dotenv::RemoveOutcome) {
    use envfish_core::dotenv::RemoveOutcome::*;
    if ctx.json {
        let _ = output::print_json(&outcome);
        return;
    }
    match outcome {
        Removed { path, variables } => {
            println!(
                "{} {path} ({} {})",
                tr("Deleted", "削除しました:"),
                variables,
                tr("variables", "変数")
            )
        }
        Kept { path, uncovered } => println!(
            "{} {path} — {} {}",
            tr("Kept", "残しました:"),
            tr("not yet stored in EnvFish:", "EnvFish に未登録:"),
            uncovered.join(", ")
        ),
        NotDotenv { path } => println!(
            "{} {path}",
            tr("Skipped (not a .env file):", "対象外 (.env ではありません):")
        ),
    }
}

/// `envfish clean [DIR]`: remove dotenv files whose variables are all stored,
/// asking for each file unless --yes.
pub async fn clean(ctx: &Ctx, dir: Option<&str>, dry_run: bool, yes: bool) -> anyhow::Result<()> {
    let project = ctx.current_project().await?;
    let env = ctx.current_environment().await?;
    let dir = match dir {
        Some(d) => std::path::PathBuf::from(d),
        None => project
            .local_path
            .clone()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from(".")),
    };
    let files = envfish_core::dotenv::dotenv_files_in(&dir);
    if files.is_empty() {
        if !ctx.json {
            println!(
                "{} {}",
                tr("No .env files in", ".env ファイルはありません:"),
                dir.display()
            );
        }
        return Ok(());
    }
    let known = known_names(ctx, &env.id).await?;
    if !ctx.json {
        println!("{} / {} · {}", project.name, env.name, dir.display());
    }
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let uncovered = envfish_core::dotenv::uncovered_names(&text, &known);
        if !uncovered.is_empty() {
            println!(
                "{} {} — {} {}",
                tr("keep", "残す:"),
                file.display(),
                tr("not yet stored:", "未登録:"),
                uncovered.join(", ")
            );
            continue;
        }
        if dry_run {
            println!("{} {}", tr("would delete", "削除対象:"), file.display());
            continue;
        }
        if yes || confirm_delete(&file)? {
            report_remove(ctx, envfish_core::dotenv::remove_if_covered(&file, &known)?);
        } else if !ctx.json {
            println!("{} {}", tr("kept", "残しました:"), file.display());
        }
    }
    Ok(())
}
