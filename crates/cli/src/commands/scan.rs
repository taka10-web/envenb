//! `envfish scan`: look for stored secret values inside a working tree.
//!
//! Every SECRET of the selected environment(s) is decrypted in memory, and each
//! text file under `dir` (skipping VCS / dependency / build folders and the
//! EnvFish data dir) is searched for the exact value. Hits are reported as
//! `file:line  VARIABLE_NAME`; the value itself is never printed. Tracked `.env`
//! files (`git ls-files`) are reported as well.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::commands::Ctx;
use crate::i18n::tr;
use crate::output;

const SKIP_DIRS: [&str; 9] = [
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".turbo",
    "vendor",
    ".venv",
];
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Serialize)]
struct Hit {
    file: String,
    line: usize,
    variable: String,
    environment: String,
}

#[derive(Serialize)]
struct Report {
    scanned_files: usize,
    hits: Vec<Hit>,
    tracked_env_files: Vec<String>,
}

pub async fn run(ctx: &Ctx, dir: &str, all_environments: bool) -> anyhow::Result<()> {
    let project = ctx.current_project().await?;
    let root = std::fs::canonicalize(dir)?;
    let envs = if all_environments {
        ctx.app.list_environments(&project.id).await?
    } else {
        vec![ctx.current_environment().await?]
    };

    // (environment name, variable name, plaintext) — kept only for the scan.
    let mut needles: Vec<(String, String, envfish_core::SecretValue)> = Vec::new();
    for env in &envs {
        for v in ctx.app.list_variables(&env.id).await? {
            if v.kind == envfish_core::VariableKind::Secret {
                let value = ctx.app.with_secret(&env.id, &v.name, |s| s.to_string()).await?;
                if value.len() >= 6 {
                    needles.push((
                        env.name.clone(),
                        v.name.clone(),
                        envfish_core::SecretValue::new(value),
                    ));
                }
            }
        }
    }

    let data_dir = ctx.app.paths().root().to_path_buf();
    let mut files = Vec::new();
    collect_files(&root, &data_dir, &mut files);

    let mut hits = Vec::new();
    for file in &files {
        let Ok(bytes) = std::fs::read(file) else { continue };
        let Ok(text) = std::str::from_utf8(&bytes) else {
            continue;
        };
        for (line_no, line) in text.lines().enumerate() {
            for (env_name, var_name, value) in &needles {
                if line.contains(value.expose()) {
                    hits.push(Hit {
                        file: display_path(file, &root),
                        line: line_no + 1,
                        variable: var_name.clone(),
                        environment: env_name.clone(),
                    });
                }
            }
        }
    }
    drop(needles);

    let tracked_env_files = tracked_env_files(&root);
    let report = Report {
        scanned_files: files.len(),
        hits,
        tracked_env_files,
    };

    if ctx.json {
        output::print_json(&report)?;
    } else {
        println!(
            "{} {} {}",
            tr("Scanned", "検査しました:"),
            report.scanned_files,
            tr("files", "ファイル")
        );
        if report.hits.is_empty() {
            println!(
                "{}",
                tr(
                    "No secret values found in the tree.",
                    "ツリー内に Secret の値は見つかりませんでした。"
                )
            );
        } else {
            println!();
            let rows: Vec<Vec<String>> = report
                .hits
                .iter()
                .map(|h| {
                    vec![
                        format!("{}:{}", h.file, h.line),
                        h.variable.clone(),
                        h.environment.clone(),
                    ]
                })
                .collect();
            output::print_table(
                &[tr("LOCATION", "場所"), tr("VARIABLE", "変数"), tr("ENV", "環境")],
                &rows,
            );
        }
        if !report.tracked_env_files.is_empty() {
            println!();
            println!(
                "{}",
                tr(
                    "These .env files are tracked by git — add them to .gitignore:",
                    "次の .env ファイルは git に追跡されています。.gitignore に追加してください:"
                )
            );
            for f in &report.tracked_env_files {
                println!("  - {f}");
            }
        }
    }
    if !report.hits.is_empty() || !report.tracked_env_files.is_empty() {
        std::process::exit(2);
    }
    Ok(())
}

fn collect_files(dir: &Path, data_dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path == data_dir {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && SKIP_DIRS.contains(&name)
            {
                continue;
            }
            collect_files(&path, data_dir, out);
        } else if meta.is_file() && meta.len() <= MAX_FILE_BYTES {
            out.push(path);
        }
    }
}

fn tracked_env_files(root: &Path) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["ls-files", "--", ".env", ".env.*", "**/.env", "**/.env.*"])
        .current_dir(root)
        .output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| !l.ends_with(".example") && !l.ends_with(".sample"))
            .map(String::from)
            .collect(),
        _ => Vec::new(),
    }
}

fn display_path(file: &Path, root: &Path) -> String {
    file.strip_prefix(root).unwrap_or(file).display().to_string()
}
