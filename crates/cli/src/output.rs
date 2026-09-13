//! Table / JSON rendering helpers. Only ever receives non-secret data.

use serde::Serialize;

use crate::i18n::{display_width, pad_right};

pub fn print_json<T: Serialize>(value: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

/// Minimal fixed-width table without external dependencies.
pub fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        println!("{}", crate::i18n::tr("(none)", "(なし)"));
        return;
    }
    let mut widths: Vec<usize> = headers.iter().map(|h| display_width(h)).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(display_width(cell));
        }
    }
    let fmt_row = |cells: &[String]| {
        cells
            .iter()
            .enumerate()
            .map(|(i, c)| pad_right(c, widths[i]))
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end()
            .to_string()
    };
    let head: Vec<String> = headers.iter().map(|h| h.to_string()).collect();
    println!("{}", fmt_row(&head));
    println!(
        "{}",
        widths
            .iter()
            .map(|w| "-".repeat(*w))
            .collect::<Vec<_>>()
            .join("  ")
    );
    for row in rows {
        println!("{}", fmt_row(row));
    }
}
