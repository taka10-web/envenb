//! CLI message locale.
//!
//! Resolution order: `ENVENB_LANG` (`ja` / `en`), then `LC_ALL`, `LC_MESSAGES`,
//! `LANG`. Anything starting with `ja` selects Japanese; everything else English.
//! `tr(en, ja)` returns the right static string, which lets clap's derive
//! attributes (`about = tr(..)`) and runtime `println!` share one mechanism.

use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Ja,
}

static LANG: OnceLock<Lang> = OnceLock::new();

pub fn lang() -> Lang {
    *LANG.get_or_init(|| detect(None))
}

/// Fix the language for this process. Precedence: `ENVENB_LANG` env var, then the
/// persisted setting (`ja` / `en`; `system` defers), then the OS locale variables.
pub fn init(setting: Option<&str>) {
    let _ = LANG.set(detect(setting));
}

fn detect(setting: Option<&str>) -> Lang {
    if let Some(v) = envenb_core::env_compat::var("LANG") {
        return match v.to_ascii_lowercase().as_str() {
            "ja" | "ja_jp" | "japanese" => Lang::Ja,
            _ => Lang::En,
        };
    }
    match setting.map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("ja") => return Lang::Ja,
        Some("en") => return Lang::En,
        _ => {}
    }
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(key)
            && !v.is_empty()
        {
            return if v.to_ascii_lowercase().starts_with("ja") {
                Lang::Ja
            } else {
                Lang::En
            };
        }
    }
    Lang::En
}

/// Pick the message for the active locale.
pub fn tr(en: &'static str, ja: &'static str) -> &'static str {
    match lang() {
        Lang::En => en,
        Lang::Ja => ja,
    }
}

/// Render an error for the active locale. Core errors carry identifiers only, so
/// translating them here never touches secret material.
pub fn describe_error(err: &anyhow::Error) -> String {
    use envenb_core::CoreError as E;
    if lang() == Lang::En {
        return format!("{err:#}");
    }
    if let Some(core) = err.downcast_ref::<E>() {
        return match core {
            E::ProjectNotFound(n) => format!("プロジェクトが見つかりません: {n}"),
            E::EnvironmentNotFound(n) => format!("環境が見つかりません: {n}"),
            E::VariableNotFound(n) => format!("変数が見つかりません: {n}"),
            E::AlreadyExists(what) => format!("同じ名前の {what} が既に存在します"),
            E::InvalidName(msg) => format!("名前が不正です: {msg}"),
            E::NoCurrentProject => {
                "プロジェクトが未選択です。先に `envenb use <プロジェクト>` を実行してください".into()
            }
            E::NoCurrentEnvironment => "環境が未選択です。先に `envenb env <環境>` を実行してください".into(),
            E::Vault(_) => format!("Vault エラー: {core}"),
            E::Database(_) => format!("データベースエラー: {core}"),
            other => other.to_string(),
        };
    }
    format!("{err:#}")
}

/// Terminal display width: East Asian wide characters take two cells.
pub fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            let cp = c as u32;
            let wide = (0x1100..=0x115F).contains(&cp)
                || (0x2E80..=0xA4CF).contains(&cp)
                || (0xAC00..=0xD7A3).contains(&cp)
                || (0xF900..=0xFAFF).contains(&cp)
                || (0xFE30..=0xFE4F).contains(&cp)
                || (0xFF00..=0xFF60).contains(&cp)
                || (0xFFE0..=0xFFE6).contains(&cp)
                || (0x20000..=0x3FFFD).contains(&cp);
            if wide { 2 } else { 1 }
        })
        .sum()
}

/// Left-align `s` to `width` display cells.
pub fn pad_right(s: &str, width: usize) -> String {
    let w = display_width(s);
    format!("{s}{}", " ".repeat(width.saturating_sub(w)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_counts_cjk_as_two() {
        assert_eq!(display_width("abc"), 3);
        assert_eq!(display_width("名前"), 4);
        assert_eq!(pad_right("名前", 6), "名前  ");
    }

    #[test]
    fn tr_returns_static_for_active_lang() {
        let s = tr("hello", "こんにちは");
        assert!(s == "hello" || s == "こんにちは");
    }
}
