use crate::cli::VaultCommand;
use crate::commands::Ctx;
use crate::i18n::tr;
use crate::output;

pub async fn run(ctx: &Ctx, command: VaultCommand) -> anyhow::Result<()> {
    match command {
        VaultCommand::Status => {
            let report = ctx.app.status().await?;
            let settings = ctx.app.settings().await?;
            if ctx.json {
                return output::print_json(&serde_json::json!({
                    "key_backend": settings.key_backend,
                    "master_key_location": report.master_key_location,
                    "secret_count": report.secret_count,
                }));
            }
            println!("{} {}", tr("key backend:", "鍵の保存先:"), settings.key_backend);
            println!(
                "{} {}",
                tr("master key:", "マスターキー:"),
                report.master_key_location
            );
            println!("{} {}", tr("secrets:", "Secret 数:"), report.secret_count);
            Ok(())
        }
        VaultCommand::KeyBackend { backend } => {
            let settings = ctx.app.switch_key_backend(&backend).await?;
            if ctx.json {
                return output::print_json(&settings);
            }
            println!(
                "{} {}",
                tr(
                    "master key is now stored in:",
                    "マスターキーの保存先を変更しました:"
                ),
                settings.key_backend
            );
            if settings.key_backend == "keychain" {
                println!(
                    "{}",
                    tr(
                        "The key file was removed. Existing secrets stay readable; the OS may prompt for access.",
                        "鍵ファイルは削除しました。既存の Secret はそのまま読めます。OS がアクセス許可を求める場合があります。"
                    )
                );
            }
            Ok(())
        }
    }
}
