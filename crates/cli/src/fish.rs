//! The EnvFish goldfish splash — two pixel-art goldfish.
//!
//! One fish is plain red, one is *nishiki* (brocade: red / white / black with a
//! gold eye) and one is a black *demekin* (telescope goldfish with bulging eyes).
//! The nishiki description continues: (red / white / black
//! with a gold eye). Each sprite is drawn with half-block characters so a text
//! cell carries two vertical pixels, giving a crisp "dot" look in any terminal.
//!
//! Rules:
//! - never when stdout is not a TTY (pipes, redirects)
//! - never when `CI` or `ENVFISH_NO_ANIMATION` is set, or `TERM=dumb`
//! - never with `--no-animation` or `--json`
//! - honours `NO_COLOR` (monochrome dots)
//! - draws only inside lines it reserves and restores the cursor, so it cannot
//!   corrupt regular command output
//! - contains no data from the vault, ever

use std::io::{IsTerminal, Write};
use std::time::Duration;

use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::{cursor, execute, queue, terminal};

const FRAME_DELAY: Duration = Duration::from_millis(60);
const FRAMES: usize = 13;
const LANE_WIDTH: u16 = 40;

/// Sprite legend: `.` transparent, `R` red, `W` white, `K` black, `G` gold, `D` dark body.
/// 8 rows (4 text lines) × 11 columns, facing right, fan tail on the left.
pub const SPRITE_W: u16 = 11;
pub const SPRITE_ROWS: usize = 8;
const SPRITE_LINES: u16 = (SPRITE_ROWS / 2) as u16;

const RED_FISH: [&str; SPRITE_ROWS] = [
    "R.....RRRR.",
    "RR...RRRRRR",
    ".RR.RRRRRKR",
    "..RRRRRRRRR",
    ".RR.RRRRRR.",
    "RR...RRRR..",
    "R.....RR...",
    "...........",
];

const NISHIKI_FISH: [&str; SPRITE_ROWS] = [
    "W.....RRWW.",
    "WR...RWWWWR",
    ".RW.WWRRWGW",
    "..WRWRRWWWW",
    ".WR.WWKKWW.",
    "RW...WWRW..",
    "W.....WW...",
    "...........",
];

/// Black demekin: dark body, oversized protruding eye (white with a gold centre).
const DEMEKIN_FISH: [&str; SPRITE_ROWS] = [
    "D.....DDDD.",
    "DD...DDDWWD",
    ".DD.DDDDWGD",
    "..DDDDDDDDD",
    ".DD.DDDDDD.",
    "DD...DDDD..",
    "D.....DD...",
    "...........",
];

#[derive(Clone, Copy)]
pub enum Fish {
    Red,
    Nishiki,
    Demekin,
}

impl Fish {
    fn sprite(self) -> &'static [&'static str; SPRITE_ROWS] {
        match self {
            Fish::Red => &RED_FISH,
            Fish::Nishiki => &NISHIKI_FISH,
            Fish::Demekin => &DEMEKIN_FISH,
        }
    }
}

fn pixel_color(c: u8) -> Option<Color> {
    match c {
        b'R' => Some(Color::Red),
        b'W' => Some(Color::White),
        b'K' => Some(Color::Black),
        b'G' => Some(Color::Yellow),
        // Plain black would vanish on dark terminals; dark grey reads as black.
        b'D' => Some(Color::DarkGrey),
        _ => None,
    }
}

/// Decide whether the splash may be shown at all.
pub fn should_animate(no_animation_flag: bool, json: bool) -> bool {
    if no_animation_flag || json {
        return false;
    }
    if std::env::var_os("ENVFISH_NO_ANIMATION").is_some() || std::env::var_os("CI").is_some() {
        return false;
    }
    if std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false) {
        return false;
    }
    std::io::stdout().is_terminal()
}

fn colors_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none()
}

/// Play the splash, then leave the two fish resting on screen above a blank line.
/// Any terminal error aborts the animation silently; the command output still runs.
pub fn splash() {
    if let Err(err) = run() {
        tracing::debug!(error = %err, "goldfish splash skipped");
        let _ = execute!(std::io::stdout(), cursor::Show, ResetColor);
    }
}

fn run() -> std::io::Result<()> {
    let mut out = std::io::stdout();
    // Some pseudo-terminals report a zero size; fall back to a classic 80 columns.
    let cols = match terminal::size() {
        Ok((c, _)) if c > 0 => c,
        _ => 80,
    };
    let lane = LANE_WIDTH.min(cols.saturating_sub(SPRITE_W + 4)).max(SPRITE_W);
    let color = colors_enabled();
    let total_lines = SPRITE_LINES * 3;

    // Reserve the lines up front so MoveUp never scrolls into earlier output.
    execute!(out, Print("\n".repeat(total_lines as usize)), cursor::Hide)?;

    for frame in 0..FRAMES {
        let progress = frame as f32 / (FRAMES - 1) as f32;
        let travel = (lane - SPRITE_W) as f32;
        let red_x = (progress * travel) as u16;
        let nishiki_x = (progress * travel * 0.8) as u16 + 3;
        let demekin_x = (progress * travel * 0.65) as u16 + 1;

        queue!(out, cursor::MoveUp(total_lines), cursor::MoveToColumn(0))?;
        draw_fish_lane(&mut out, Fish::Red, red_x, lane, frame, color)?;
        draw_fish_lane(
            &mut out,
            Fish::Nishiki,
            nishiki_x.min(lane - SPRITE_W),
            lane,
            frame + 3,
            color,
        )?;
        draw_fish_lane(
            &mut out,
            Fish::Demekin,
            demekin_x.min(lane - SPRITE_W),
            lane,
            frame + 7,
            color,
        )?;
        out.flush()?;
        std::thread::sleep(FRAME_DELAY);
    }

    execute!(out, ResetColor, cursor::Show, Print("\n"))?;
    Ok(())
}

/// Draw one fish (4 text lines) at horizontal offset `x` inside a lane of `lane` cells,
/// with a few drifting ripples in the empty water.
fn draw_fish_lane(
    out: &mut impl Write,
    fish: Fish,
    x: u16,
    lane: u16,
    frame: usize,
    color: bool,
) -> std::io::Result<()> {
    let sprite = fish.sprite();
    for line in 0..SPRITE_LINES as usize {
        let top = sprite[line * 2].as_bytes();
        let bottom = sprite[line * 2 + 1].as_bytes();
        queue!(
            out,
            terminal::Clear(terminal::ClearType::CurrentLine),
            Print("  ")
        )?;
        for col in 0..lane {
            if col >= x && col < x + SPRITE_W {
                let i = (col - x) as usize;
                draw_cell(out, pixel_color(top[i]), pixel_color(bottom[i]), color)?;
            } else if line == 1 && (col as usize + frame).is_multiple_of(13) {
                if color {
                    queue!(out, SetForegroundColor(Color::DarkCyan), Print("~"), ResetColor)?;
                } else {
                    queue!(out, Print("~"))?;
                }
            } else {
                queue!(out, Print(" "))?;
            }
        }
        queue!(out, ResetColor, Print("\n"))?;
    }
    Ok(())
}

/// One text cell = two vertical pixels. `▀` shows the top pixel in the foreground and
/// the bottom pixel in the background colour; `▄` is used when only the bottom is set.
fn draw_cell(
    out: &mut impl Write,
    top: Option<Color>,
    bottom: Option<Color>,
    color: bool,
) -> std::io::Result<()> {
    match (top, bottom) {
        (None, None) => queue!(out, Print(" ")),
        (Some(t), None) => {
            if color {
                queue!(out, SetForegroundColor(t), Print("▀"), ResetColor)
            } else {
                queue!(out, Print("▀"))
            }
        }
        (None, Some(b)) => {
            if color {
                queue!(out, SetForegroundColor(b), Print("▄"), ResetColor)
            } else {
                queue!(out, Print("▄"))
            }
        }
        (Some(t), Some(b)) => {
            if color {
                queue!(
                    out,
                    SetForegroundColor(t),
                    SetBackgroundColor(b),
                    Print("▀"),
                    ResetColor
                )
            } else {
                queue!(out, Print("█"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_disable_animation() {
        assert!(!should_animate(true, false));
        assert!(!should_animate(false, true));
    }

    #[test]
    fn sprites_are_rectangular_and_use_known_pixels() {
        for sprite in [&RED_FISH, &NISHIKI_FISH, &DEMEKIN_FISH] {
            for row in sprite.iter() {
                assert_eq!(row.len() as u16, SPRITE_W);
                assert!(
                    row.bytes()
                        .all(|b| matches!(b, b'.' | b'R' | b'W' | b'K' | b'G' | b'D'))
                );
            }
        }
    }

    #[test]
    fn demekin_has_a_big_eye() {
        let joined = DEMEKIN_FISH.concat();
        assert_eq!(joined.matches('W').count(), 3);
        assert!(joined.contains('G') && joined.contains('D') && !joined.contains('R'));
    }

    #[test]
    fn nishiki_is_multicoloured() {
        let joined = NISHIKI_FISH.concat();
        assert!(joined.contains('R') && joined.contains('W') && joined.contains('K') && joined.contains('G'));
    }
}
