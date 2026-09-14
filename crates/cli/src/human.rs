//! Human-only gate for commands that let plaintext leave the vault.
//!
//! The MCP surface never returns secrets, but an AI agent with shell access could
//! simply run `envenb run env`. These commands therefore require an interactive
//! terminal and refuse to run inside known agent sessions. A deliberate override
//! exists for scripts the human sets up themselves.

use std::io::IsTerminal;

use crate::i18n::tr;

pub const OVERRIDE_ENV: &str = "ENVENB_ALLOW_UNATTENDED";

/// Environment variables that coding agents set in the shells they spawn.
const AGENT_MARKERS: [&str; 6] = [
    "CLAUDECODE",
    "CLAUDE_CODE_ENTRYPOINT",
    "CODEX_SANDBOX",
    "CODEX_CI",
    "CURSOR_AGENT",
    "GEMINI_CLI",
];

pub fn agent_marker() -> Option<&'static str> {
    AGENT_MARKERS.into_iter().find(|m| std::env::var_os(m).is_some())
}

/// Refuse unless a human is plausibly at the keyboard.
pub fn require_human(what: &str) -> anyhow::Result<()> {
    // Agent markers win over the override: a script the human wrote does not run
    // inside an agent's shell, and an agent that strips the marker has to do so in
    // a command the human gets to review.
    if let Some(marker) = agent_marker() {
        anyhow::bail!(
            "{what}: {} ({marker}). {}",
            tr(
                "refused inside an AI agent session",
                "AI エージェントのセッション内では実行できません"
            ),
            tr(
                "Secrets are for humans and for `envenb run` started from a real terminal; agents use the MCP broker.",
                "Secret は人間と、実際の端末から起動した `envenb run` のためのものです。AI エージェントは MCP の Broker を使ってください。"
            )
        );
    }
    if envenb_core::env_compat::var("ALLOW_UNATTENDED").as_deref() == Some("1") {
        return Ok(());
    }
    if !std::io::stdin().is_terminal() {
        let hint = match crate::i18n::lang() {
            crate::i18n::Lang::En => format!("Set {OVERRIDE_ENV}=1 only for scripts you run yourself."),
            crate::i18n::Lang::Ja => {
                format!("自分で実行するスクリプトに限り {OVERRIDE_ENV}=1 で許可できます。")
            }
        };
        anyhow::bail!(
            "{what}: {} {hint}",
            tr(
                "requires an interactive terminal.",
                "対話端末からのみ実行できます。"
            )
        );
    }
    Ok(())
}
