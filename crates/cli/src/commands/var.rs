use std::io::{IsTerminal, Read, Write};

use anyhow::Context;
use envenb_core::{SecretValue, VariableKind};

use crate::cli::VarCommand;
use crate::commands::Ctx;
use crate::i18n::tr;
use crate::output;

pub async fn run(ctx: &Ctx, command: VarCommand) -> anyhow::Result<()> {
    let project = ctx.current_project().await?;
    let env = ctx.current_environment().await?;

    match command {
        VarCommand::List => {
            let vars = ctx.app.list_variables(&env.id).await?;
            if ctx.json {
                return output::print_json(&vars);
            }
            println!("{} / {}", project.name, env.name);
            let rows: Vec<Vec<String>> = vars
                .iter()
                .map(|v| {
                    vec![
                        v.name.clone(),
                        v.kind.to_string(),
                        match v.kind {
                            VariableKind::Public => v.value.clone().unwrap_or_default(),
                            VariableKind::Secret => "••••••••".to_string(),
                        },
                    ]
                })
                .collect();
            output::print_table(
                &[tr("NAME", "名前"), tr("KIND", "種別"), tr("VALUE", "値")],
                &rows,
            );
            Ok(())
        }
        VarCommand::Set { name, value } => {
            let v = ctx.app.set_public_variable(&env.id, &name, &value).await?;
            if ctx.json {
                return output::print_json(&v);
            }
            println!(
                "{} {} ({} / {})",
                tr("Set PUBLIC", "PUBLIC を設定しました:"),
                v.name,
                project.name,
                env.name
            );
            Ok(())
        }
        VarCommand::SetSecret { name } => {
            let secret = read_secret_from_stdin(&name)?;
            let v = ctx.app.set_secret_variable(&env.id, &name, secret).await?;
            if ctx.json {
                return output::print_json(&v);
            }
            println!(
                "{} {} ({} / {})",
                tr("Stored SECRET (encrypted)", "SECRET を暗号化して保存しました:"),
                v.name,
                project.name,
                env.name
            );
            Ok(())
        }
        VarCommand::Kind { name, kind } => {
            let kind: VariableKind = kind.parse().map_err(anyhow::Error::msg)?;
            let v = ctx.app.change_variable_kind(&env.id, &name, kind).await?;
            if ctx.json {
                return output::print_json(&v);
            }
            println!(
                "{} {} → {}",
                tr("Changed kind:", "種別を変更しました:"),
                v.name,
                v.kind
            );
            Ok(())
        }
        VarCommand::Copy { name } => {
            // Same footing as `cred copy`: a human at a real terminal, and the
            // value goes to the clipboard rather than to the scrollback.
            crate::human::require_human("envenb var copy")?;
            if !std::io::stdout().is_terminal() {
                anyhow::bail!(
                    "{}",
                    tr(
                        "var copy is only available from an interactive terminal",
                        "var copy は対話端末からのみ実行できます"
                    )
                );
            }
            let vars = ctx.app.list_variables(&env.id).await?;
            let Some(v) = vars.iter().find(|v| v.name == name) else {
                anyhow::bail!(
                    "{} {name}",
                    tr("no such variable:", "そのような変数はありません:")
                );
            };
            if v.kind != VariableKind::Secret {
                anyhow::bail!(
                    "{} {name}",
                    tr(
                        "not a SECRET variable; its value is already visible in `var list`:",
                        "SECRET ではありません。値は `var list` に表示されています:"
                    )
                );
            }
            let ttl = envenb_core::clipboard::DEFAULT_TTL;
            ctx.app
                .with_secret(&env.id, &name, |v| {
                    envenb_core::clipboard::copy_then_clear(v, ttl)
                })
                .await?
                .map_err(anyhow::Error::msg)?;
            eprintln!(
                "{} {name} {} {}s",
                tr("Copied", "コピーしました:"),
                tr(
                    "to the clipboard; it will be cleared in",
                    "クリップボードは次の秒数後に消去されます:"
                ),
                ttl.as_secs()
            );
            // Stay alive so the clearing thread can run.
            std::thread::sleep(ttl + std::time::Duration::from_millis(200));
            Ok(())
        }
        VarCommand::Remove { name } => {
            ctx.app.delete_variable(&env.id, &name).await?;
            if !ctx.json {
                println!("{} {name}", tr("Removed", "削除しました:"));
            }
            Ok(())
        }
    }
}

/// Read a secret from stdin. On a TTY the echo is disabled while typing so the value
/// does not land in the terminal scrollback; from a pipe the whole input is taken.
fn read_secret_from_stdin(name: &str) -> anyhow::Result<SecretValue> {
    let mut stdin = std::io::stdin();
    let raw = if stdin.is_terminal() {
        eprint!(
            "{} {name} {}",
            tr("Enter value for", "値を入力してください:"),
            tr("(input hidden):", "(入力は表示されません):")
        );
        std::io::stderr().flush().ok();
        read_line_no_echo()?
    } else {
        let mut buf = String::new();
        stdin.read_to_string(&mut buf).context(tr(
            "failed to read secret from stdin",
            "stdin から Secret を読み取れませんでした",
        ))?;
        buf
    };
    let trimmed = raw.trim_end_matches(['\r', '\n']).to_string();
    zeroize_string(raw);
    if trimmed.is_empty() {
        anyhow::bail!("{}", tr("secret value must not be empty", "Secret の値が空です"));
    }
    Ok(SecretValue::new(trimmed))
}

pub(crate) fn read_line_no_echo() -> anyhow::Result<String> {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, read};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

    enable_raw_mode().context(tr(
        "failed to switch terminal to raw mode",
        "端末を raw モードに切り替えられませんでした",
    ))?;
    let result = (|| -> anyhow::Result<String> {
        let mut buf = String::new();
        loop {
            if let Event::Key(KeyEvent { code, modifiers, .. }) = read()? {
                match code {
                    KeyCode::Enter => break,
                    KeyCode::Backspace => {
                        buf.pop();
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        anyhow::bail!("{}", tr("cancelled", "キャンセルしました"));
                    }
                    KeyCode::Char(c) => buf.push(c),
                    _ => {}
                }
            }
        }
        Ok(buf)
    })();
    disable_raw_mode().ok();
    eprintln!();
    result
}

fn zeroize_string(mut s: String) {
    // Best-effort scrub of the intermediate buffer.
    unsafe { s.as_bytes_mut() }.fill(0);
}
