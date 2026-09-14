//! The EnvEnb splash — a pixel-art *maiko* who dances while the vault opens.
//!
//! The name is a pun: `env` + 演舞 (*enbu*, a performed dance). The sprite was
//! traced down from reference artwork: a maiko in a black kimono with a gold
//! obi, holding a fan aloft. Eight frames bend the whole body around her feet,
//! so she sways rather than slides, and cherry petals fall independently.
//!
//! Each sprite is drawn with half-block characters, so one text cell carries two
//! vertical pixels and the dots stay crisp in any terminal.
//!
//! Rules:
//! - never when stdout is not a TTY (pipes, redirects)
//! - never when `CI` or `ENVENB_NO_ANIMATION` is set, or `TERM=dumb`
//! - never with `--no-animation` or `--json`
//! - honours `NO_COLOR` (monochrome dots)
//! - draws only inside lines it reserves and restores the cursor, so it cannot
//!   corrupt regular command output
//! - contains no data from the vault, ever

use std::io::{IsTerminal, Write};
use std::time::Duration;

use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::{cursor, execute, queue, terminal};

const FRAME_DELAY: Duration = Duration::from_millis(150);
/// Two full turns of the dance: long enough to read, short enough to ignore.
const FRAMES_SHOWN: usize = 16;

/// Sprite legend: `.` transparent, then one letter per palette entry.
pub const SPRITE_W: u16 = 24;
pub const SPRITE_ROWS: usize = 36;
pub const FRAME_COUNT: usize = 8;

const FRAMES: [[&str; SPRITE_ROWS]; FRAME_COUNT] = [
    [
        "...............KEK......",
        "..............KKKE......",
        "..............KKKgK.....",
        ".............KKCKKK.....",
        "......KPBE...KESSKK.....",
        ".....EPCEPB...ESSKK.....",
        "....EPECSEPE...CCK......",
        "....EEPPPBEE..KCCC......",
        ".....EgKKgE.KdVSCVd.....",
        ".......EEK.dVVVSvVVd....",
        "....EdddddKVVVbvVVVV....",
        ".....KdVVVVddVbVVdVVd...",
        "......dVVVVddKVVdKddV...",
        "......dVVVVdKRRRggKVVd..",
        "......KVVVddKPPCCYdVVd..",
        "......dVVdVdKgYgGYddVVd.",
        "......dVVVVdKKYGGgdVVVd.",
        "......dVVVddKKdgGgVVdVd.",
        "......KVVVVdKdVddKbvdVV.",
        ".......VVVdKKdEVVKvPVVVd",
        ".......VVdKK.dEgVKbPPVVd",
        ".......ddK...dPvgKKPPVK.",
        ".............dvPgdVPbK..",
        ".............VvEEdVVK...",
        ".............VvEEdVd....",
        ".............VVGGdK.....",
        ".............dVVGdK.....",
        "..............VdVVd.....",
        "..............VdEEB.....",
        "..............ddVBP.....",
        "..............dvEEPB....",
        "..............dBdEBK....",
        ".............dVvEEVVK...",
        "............KddVPgEdVV..",
        "...........KddKEPVbgVvd.",
        ".............KKBEEddKK..",
    ],
    [
        "...............KKKE.....",
        "...............KKKgK....",
        "..............KKCKKK....",
        "........KPBE............",
        ".......EPCEPB.KESSKK....",
        "......EPECSEPE.ESSKK....",
        "......EEPPPBEE..CCK.....",
        ".......EgKKgE..KCCC.....",
        ".........EEK.KdVSCVd....",
        "............dVVVSvVVd...",
        ".....EdddddKVVVbvVVVV...",
        "......KdVVVVddVbVVdVVd..",
        ".......dVVVVddKVVdKddV..",
        ".......dVVVVdKRRRggKVVd.",
        ".......KVVVddKPPCCYdVVd.",
        ".......dVVdVdKgYgGYddVVd",
        ".......dVVVVdKKYGGgdVVVd",
        "......dVVVddKKdgGgVVdVd.",
        "......KVVVVdKdVddKbvdVV.",
        ".......VVVdKKdEVVKvPVVVd",
        ".......VVdKK.dEgVKbPPVVd",
        ".......ddK...dPvgKKPPVK.",
        ".............dvPgdVPbK..",
        ".............VvEEdVVK...",
        ".............VvEEdVd....",
        ".............VVGGdK.....",
        ".............dVVGdK.....",
        "..............VdVVd.....",
        "..............VdEEB.....",
        "..............ddVBP.....",
        "..............dvEEPB....",
        "..............dBdEBK....",
        ".............dVvEEVVK...",
        "............KddVPgEdVV..",
        "...........KddKEPVbgVvd.",
        ".............KKBEEddKK..",
    ],
    [
        "................KKKE....",
        "................KKKgK...",
        "...............KKCKKK...",
        "...............KESSKK...",
        ".......KPBE.....ESSKK...",
        "......EPCEPB....CCK.....",
        ".....EPECSEPE..KCCC.....",
        ".....EEPPPBEEKdVSCVd....",
        "......EgKKgE............",
        "........EEK.dVVVSvVVd...",
        ".....EdddddKVVVbvVVVV...",
        "......KdVVVVddVbVVdVVd..",
        ".......dVVVVddKVVdKddV..",
        ".......dVVVVdKRRRggKVVd.",
        ".......KVVVddKPPCCYdVVd.",
        ".......dVVdVdKgYgGYddVVd",
        ".......dVVVVdKKYGGgdVVVd",
        ".......dVVVddKKdgGgVVdVd",
        ".......KVVVVdKdVddKbvdVV",
        "........VVVdKKdEVVKvPVVV",
        "........VVdKK.dEgVKbPPVV",
        ".......ddK...dPvgKKPPVK.",
        ".............dvPgdVPbK..",
        ".............VvEEdVVK...",
        ".............VvEEdVd....",
        ".............VVGGdK.....",
        ".............dVVGdK.....",
        "..............VdVVd.....",
        "..............VdEEB.....",
        "..............ddVBP.....",
        "..............dvEEPB....",
        "..............dBdEBK....",
        ".............dVvEEVVK...",
        "............KddVPgEdVV..",
        "...........KddKEPVbgVvd.",
        ".............KKBEEddKK..",
    ],
    [
        "...............KKKE.....",
        "...............KKKgK....",
        "..............KKCKKK....",
        "........................",
        "..............KESSKK....",
        "...............ESSKK....",
        ".....KPBE.......CCK.....",
        "....EPCEPB.....KCCC.....",
        "...EPECSEPE..KdVSCVd....",
        "...EEPPPBEE.dVVVSvVVd...",
        "....EgKKgEdKVVVbvVVVV...",
        "......EEKVVVddVbVVdVVd..",
        ".......dVVVVddKVVdKddV..",
        ".......dVVVVdKRRRggKVVd.",
        ".......KVVVddKPPCCYdVVd.",
        ".......dVVdVdKgYgGYddVVd",
        ".......dVVVVdKKYGGgdVVVd",
        "......dVVVddKKdgGgVVdVd.",
        "......KVVVVdKdVddKbvdVV.",
        ".......VVVdKKdEVVKvPVVVd",
        ".......VVdKK.dEgVKbPPVVd",
        ".......ddK...dPvgKKPPVK.",
        ".............dvPgdVPbK..",
        ".............VvEEdVVK...",
        ".............VvEEdVd....",
        ".............VVGGdK.....",
        ".............dVVGdK.....",
        "..............VdVVd.....",
        "..............VdEEB.....",
        "..............ddVBP.....",
        "..............dvEEPB....",
        "..............dBdEBK....",
        ".............dVvEEVVK...",
        "............KddVPgEdVV..",
        "...........KddKEPVbgVvd.",
        ".............KKBEEddKK..",
    ],
    [
        "...............KEK......",
        "..............KKKE......",
        "..............KKKgK.....",
        ".............KKCKKK.....",
        ".............KESSKK.....",
        "..............ESSKK.....",
        "...............CCK......",
        "..............KCCC......",
        "..KPBE......KdVSCVd.....",
        ".EPCEPB....dVVVSvVVd....",
        "EPECSEPEddKVVVbvVVVV....",
        "EEPPPBEEVVVddVbVVdVVd...",
        ".EgKKgEVVVVddKVVdKddV...",
        "...EEKdVVVVdKRRRggKVVd..",
        "......KVVVddKPPCCYdVVd..",
        "......dVVdVdKgYgGYddVVd.",
        "......dVVVVdKKYGGgdVVVd.",
        "......dVVVddKKdgGgVVdVd.",
        "......KVVVVdKdVddKbvdVV.",
        ".......VVVdKKdEVVKvPVVVd",
        ".......VVdKK.dEgVKbPPVVd",
        ".......ddK...dPvgKKPPVK.",
        ".............dvPgdVPbK..",
        ".............VvEEdVVK...",
        ".............VvEEdVd....",
        ".............VVGGdK.....",
        ".............dVVGdK.....",
        "..............VdVVd.....",
        "..............VdEEB.....",
        "..............ddVBP.....",
        "..............dvEEPB....",
        "..............dBdEBK....",
        ".............dVvEEVVK...",
        "............KddVPgEdVV..",
        "...........KddKEPVbgVvd.",
        ".............KKBEEddKK..",
    ],
    [
        ".............KKKE.......",
        ".............KKKgK......",
        "............KKCKKK......",
        "........................",
        "............KESSKK......",
        ".............ESSKK......",
        "..............CCK.......",
        ".............KCCC.......",
        "KPBE.......KdVSCVd......",
        "PCEPB.....dVVVSvVVd.....",
        "ECSEPEdddKVVVbvVVVV.....",
        "PPPBEEVVVVddVbVVdVVd....",
        "gKKgEdVVVVddKVVdKddV....",
        ".EEK.dVVVVdKRRRggKVVd...",
        ".....KVVVddKPPCCYdVVd...",
        ".....dVVdVdKgYgGYddVVd..",
        ".....dVVVVdKKYGGgdVVVd..",
        "......dVVVddKKdgGgVVdVd.",
        "......KVVVVdKdVddKbvdVV.",
        ".......VVVdKKdEVVKvPVVVd",
        ".......VVdKK.dEgVKbPPVVd",
        ".......ddK...dPvgKKPPVK.",
        ".............dvPgdVPbK..",
        ".............VvEEdVVK...",
        ".............VvEEdVd....",
        ".............VVGGdK.....",
        ".............dVVGdK.....",
        "..............VdVVd.....",
        "..............VdEEB.....",
        "..............ddVBP.....",
        "..............dvEEPB....",
        "..............dBdEBK....",
        ".............dVvEEVVK...",
        "............KddVPgEdVV..",
        "...........KddKEPVbgVvd.",
        ".............KKBEEddKK..",
    ],
    [
        "............KKKE........",
        "............KKKgK.......",
        "...........KKCKKK.......",
        "...........KESSKK.......",
        "............ESSKK.......",
        "..............CCK.......",
        ".............KCCC.......",
        ".KPBE......KdVSCVd......",
        "EPCEPB..................",
        "PECSEPE...dVVVSvVVd.....",
        "EPPPBEEddKVVVbvVVVV.....",
        "EgKKgEVVVVddVbVVdVVd....",
        "..EEKdVVVVddKVVdKddV....",
        ".....dVVVVdKRRRggKVVd...",
        ".....KVVVddKPPCCYdVVd...",
        ".....dVVdVdKgYgGYddVVd..",
        ".....dVVVVdKKYGGgdVVVd..",
        ".....dVVVddKKdgGgVVdVd..",
        ".....KVVVVdKdVddKbvdVV..",
        "......VVVdKKdEVVKvPVVVd.",
        "......VVdKK.dEgVKbPPVVd.",
        ".......ddK...dPvgKKPPVK.",
        ".............dvPgdVPbK..",
        ".............VvEEdVVK...",
        ".............VvEEdVd....",
        ".............VVGGdK.....",
        ".............dVVGdK.....",
        "..............VdVVd.....",
        "..............VdEEB.....",
        "..............ddVBP.....",
        "..............dvEEPB....",
        "..............dBdEBK....",
        ".............dVvEEVVK...",
        "............KddVPgEdVV..",
        "...........KddKEPVbgVvd.",
        ".............KKBEEddKK..",
    ],
    [
        ".............KKKE.......",
        ".............KKKgK......",
        "............KKCKKK......",
        "........................",
        "............KESSKK......",
        "...KPBE......ESSKK......",
        "..EPCEPB......CCK.......",
        ".EPECSEPE....KCCC.......",
        ".EEPPPBEE..KdVSCVd......",
        "..EgKKgE..dVVVSvVVd.....",
        "...EEEKddKVVVbvVVVV.....",
        "....KdVVVVddVbVVdVVd....",
        ".....dVVVVddKVVdKddV....",
        ".....dVVVVdKRRRggKVVd...",
        ".....KVVVddKPPCCYdVVd...",
        ".....dVVdVdKgYgGYddVVd..",
        ".....dVVVVdKKYGGgdVVVd..",
        "......dVVVddKKdgGgVVdVd.",
        "......KVVVVdKdVddKbvdVV.",
        ".......VVVdKKdEVVKvPVVVd",
        ".......VVdKK.dEgVKbPPVVd",
        ".......ddK...dPvgKKPPVK.",
        ".............dvPgdVPbK..",
        ".............VvEEdVVK...",
        ".............VvEEdVd....",
        ".............VVGGdK.....",
        ".............dVVGdK.....",
        "..............VdVVd.....",
        "..............VdEEB.....",
        "..............ddVBP.....",
        "..............dvEEPB....",
        "..............dBdEBK....",
        ".............dVvEEVVK...",
        "............KddVPgEdVV..",
        "...........KddKEPVbgVvd.",
        ".............KKBEEddKK..",
    ],
];

fn pixel_color(c: u8) -> Option<Color> {
    match c {
        b'B' => Some(Color::Rgb { r: 90, g: 86, b: 102 }),
        b'C' => Some(Color::Rgb {
            r: 216,
            g: 212,
            b: 204,
        }),
        b'E' => Some(Color::Rgb { r: 70, g: 128, b: 70 }),
        b'G' => Some(Color::Rgb {
            r: 232,
            g: 178,
            b: 74,
        }),
        b'K' => Some(Color::Rgb { r: 20, g: 18, b: 28 }),
        b'P' => Some(Color::Rgb {
            r: 185,
            g: 180,
            b: 174,
        }),
        b'R' => Some(Color::Rgb { r: 210, g: 64, b: 47 }),
        b'S' => Some(Color::Rgb {
            r: 250,
            g: 228,
            b: 205,
        }),
        b'V' => Some(Color::Rgb { r: 43, g: 41, b: 50 }),
        b'Y' => Some(Color::Rgb {
            r: 246,
            g: 214,
            b: 96,
        }),
        b'b' => Some(Color::Rgb { r: 52, g: 49, b: 64 }),
        b'd' => Some(Color::Rgb { r: 23, g: 22, b: 28 }),
        b'g' => Some(Color::Rgb {
            r: 196,
            g: 132,
            b: 44,
        }),
        b'v' => Some(Color::Rgb { r: 75, g: 72, b: 86 }),
        _ => None,
    }
}

/// Falling cherry petals, drawn over the dancer. Deterministic so the splash
/// looks the same every run; the drift comes from the phase, not randomness.
const PETAL_COUNT: usize = 6;
const PETAL_COLOR: Color = Color::Rgb {
    r: 244,
    g: 190,
    b: 208,
};

fn petal_at(index: usize, frame: usize) -> (u16, u16) {
    // Each petal has its own column, fall speed and sway, spread by the index.
    let column = (index * 7 + 3) % SPRITE_W as usize;
    let drift = ((frame as f32 * 0.5 + index as f32 * 1.7).sin() * 1.6) as i32;
    let x = (column as i32 + drift).clamp(0, SPRITE_W as i32 - 1) as u16;
    let speed = 1 + index % 2;
    let y = ((index * 5 + frame * speed) % (SPRITE_ROWS + 4)) as u16;
    (x, y)
}

/// Decide whether the splash may be shown at all.
pub fn should_animate(no_animation_flag: bool, json: bool) -> bool {
    if no_animation_flag || json {
        return false;
    }
    if envenb_core::env_compat::is_set("NO_ANIMATION") || std::env::var_os("CI").is_some() {
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

/// Play the splash, then leave the final pose on screen above a blank line.
/// Any terminal error aborts the animation silently; the command output still runs.
pub fn splash() {
    if let Err(err) = run() {
        tracing::debug!(error = %err, "maiko splash skipped");
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
    if cols < SPRITE_W + 4 {
        return Ok(()); // too narrow to draw without wrapping
    }
    let color = colors_enabled();
    let lines = (SPRITE_ROWS / 2) as u16;

    // Reserve the lines up front so MoveUp never scrolls into earlier output.
    execute!(out, Print("\n".repeat(lines as usize)), cursor::Hide)?;

    for frame in 0..FRAMES_SHOWN {
        queue!(out, cursor::MoveUp(lines), cursor::MoveToColumn(0))?;
        draw_frame(&mut out, frame, color)?;
        out.flush()?;
        std::thread::sleep(FRAME_DELAY);
    }

    execute!(out, ResetColor, cursor::Show, Print("\n"))?;
    Ok(())
}

/// Draw one frame. Each text line holds two sprite rows via half-block characters.
fn draw_frame(out: &mut impl Write, frame: usize, color: bool) -> std::io::Result<()> {
    let sprite = &FRAMES[frame % FRAME_COUNT];
    let petals: Vec<(u16, u16)> = (0..PETAL_COUNT).map(|i| petal_at(i, frame)).collect();

    for line in 0..SPRITE_ROWS / 2 {
        let top = sprite[line * 2].as_bytes();
        let bottom = sprite[line * 2 + 1].as_bytes();
        queue!(
            out,
            terminal::Clear(terminal::ClearType::CurrentLine),
            Print("  ")
        )?;
        for col in 0..SPRITE_W as usize {
            let mut t = pixel_color(top[col]);
            let mut b = pixel_color(bottom[col]);
            // Petals fall behind nothing: they only fill empty cells.
            if t.is_none() && petals.contains(&(col as u16, (line * 2) as u16)) {
                t = Some(PETAL_COLOR);
            }
            if b.is_none() && petals.contains(&(col as u16, (line * 2 + 1) as u16)) {
                b = Some(PETAL_COLOR);
            }
            draw_cell(out, t, b, color)?;
        }
        queue!(out, ResetColor, Print("\n"))?;
    }
    Ok(())
}

/// One text cell = two vertical pixels. `▀` shows the top pixel in the foreground
/// and the bottom pixel in the background colour; `▄` when only the bottom is set.
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
    fn every_frame_is_rectangular_and_uses_known_pixels() {
        for frame in FRAMES.iter() {
            assert_eq!(frame.len(), SPRITE_ROWS);
            for row in frame.iter() {
                assert_eq!(row.len() as u16, SPRITE_W, "row: {row}");
                for b in row.bytes() {
                    assert!(
                        b == b'.' || pixel_color(b).is_some(),
                        "unknown pixel {}",
                        b as char
                    );
                }
            }
        }
    }

    #[test]
    fn the_dancer_actually_moves() {
        // Consecutive frames must differ, or the dance is a still image.
        for pair in FRAMES.windows(2) {
            assert_ne!(pair[0].concat(), pair[1].concat());
        }
    }

    #[test]
    fn petals_stay_inside_the_sprite() {
        for frame in 0..FRAME_COUNT * 3 {
            for i in 0..PETAL_COUNT {
                let (x, y) = petal_at(i, frame);
                assert!(x < SPRITE_W, "petal x {x} out of bounds");
                assert!((y as usize) < SPRITE_ROWS + 4, "petal y {y} out of bounds");
            }
        }
    }

    #[test]
    fn an_even_number_of_rows_so_half_blocks_pair_up() {
        assert_eq!(SPRITE_ROWS % 2, 0);
    }
}
