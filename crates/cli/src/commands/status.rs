use serde::Serialize;

use crate::commands::Ctx;
use crate::i18n::{pad_right, tr};
use crate::output;

#[derive(Serialize)]
struct StatusView {
    #[serde(flatten)]
    report: envfish_core::StatusReport,
    current_project: Option<String>,
    current_environment: Option<String>,
}

pub async fn run(ctx: &Ctx) -> anyhow::Result<()> {
    let report = ctx.app.status().await?;
    let project = ctx.current_project().await.ok();
    let environment = ctx.current_environment().await.ok();

    if ctx.json {
        return output::print_json(&StatusView {
            report,
            current_project: project.map(|p| p.name),
            current_environment: environment.map(|e| e.name),
        });
    }

    println!("  EnvFish");
    println!();
    let row = |label: &str, value: &str| println!("  {} {value}", pad_right(label, 14));
    row(tr("Data dir:", "データ:"), &report.data_dir);
    row(tr("Database:", "DB:"), &report.database_path);
    row(tr("Master key:", "マスターキー:"), &report.master_key_location);
    if report.master_key_location.starts_with("file") {
        row(
            "",
            tr(
                "⚠ key file readable by any process as you — `envfish vault key-backend keychain` is recommended",
                "⚠ 鍵ファイルは同一ユーザーの全プロセスから読めます。`envfish vault key-backend keychain` を推奨します",
            ),
        );
    }
    row(
        tr("Projects:", "プロジェクト:"),
        &report.project_count.to_string(),
    );
    row(
        tr("Secrets:", "Secret:"),
        &format!(
            "{} {}",
            report.secret_count,
            tr("(encrypted at rest)", "(暗号化保存)")
        ),
    );
    println!();
    row(
        tr("Project:", "選択中 PJ:"),
        &project.map(|p| p.name).unwrap_or_else(|| {
            tr(
                "(none) — `envfish use <project>`",
                "(未選択) — `envfish use <プロジェクト>`",
            )
            .into()
        }),
    );
    row(
        tr("Environment:", "選択中環境:"),
        &environment.map(|e| e.name).unwrap_or_else(|| {
            tr(
                "(none) — `envfish env <environment>`",
                "(未選択) — `envfish env <環境>`",
            )
            .into()
        }),
    );
    Ok(())
}
