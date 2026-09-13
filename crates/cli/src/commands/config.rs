use crate::cli::ConfigCommand;
use crate::commands::Ctx;
use crate::i18n::tr;
use crate::output;

pub async fn run(ctx: &Ctx, command: ConfigCommand) -> anyhow::Result<()> {
    let settings = match command {
        ConfigCommand::Show => ctx.app.settings().await?,
        ConfigCommand::Language { language } => ctx.app.set_language(&language).await?,
        ConfigCommand::Theme { theme } => ctx.app.set_theme(&theme).await?,
    };
    if ctx.json {
        return output::print_json(&settings);
    }
    println!("{} {}", tr("language:", "言語:"), settings.language);
    println!("{} {}", tr("theme:", "テーマ:"), settings.theme);
    println!("{} {}", tr("key backend:", "鍵の保存先:"), settings.key_backend);
    Ok(())
}
