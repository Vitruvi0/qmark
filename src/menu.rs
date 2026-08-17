//! Interactive picker drawn on `/dev/tty`.
//!
//! All UI goes to the tty; stdout stays clean so the shell widget can capture
//! the chosen entry with `$(qmark suggest --interactive ...)`.

use std::fs::{File, OpenOptions};
use std::io::{IsTerminal, Write};

use anyhow::Result;
use crossterm::{
    cursor::{MoveToColumn, MoveUp},
    event::{Event, KeyCode, KeyEventKind, KeyModifiers, read},
    queue,
    style::{Attribute, SetAttribute},
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};

use crate::suggest::Entry;

/// Rows shown at once; longer lists scroll.
const VISIBLE: usize = 12;

/// Truncate `s` to at most `width` chars (ellipsis when cut). Every menu line
/// must fit the terminal width: a wrapped line breaks the fixed-height repaint
/// (MoveUp counts logical rows, the terminal counts physical ones).
fn fit(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        return s.to_string();
    }
    let mut out: String = s.chars().take(width.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// The tty to draw on, or `None` when there is no interactive terminal
/// (piped stderr covers tests and non-interactive shells).
pub fn tty() -> Option<File> {
    if !std::io::stderr().is_terminal() {
        return None;
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .ok()
}

/// Run the picker. `Some(insert)` on Enter, `None` on Esc/q/Ctrl-C.
/// The menu stays on screen afterwards, doubling as the printed list.
pub fn pick(tty: &mut File, title: &str, entries: &[Entry]) -> Result<Option<String>> {
    enable_raw_mode()?;
    let result = pick_inner(tty, title, entries);
    disable_raw_mode()?;
    result
}

fn pick_inner(tty: &mut File, title: &str, entries: &[Entry]) -> Result<Option<String>> {
    let rows = entries.len().min(VISIBLE);
    // chars, not bytes: `{:<width$}` pads by char count.
    let width = entries
        .iter()
        .map(|e| e.display.chars().count())
        .max()
        .unwrap_or(0);
    let mut selected = 0usize;
    let mut offset = 0usize;
    let mut first = true;

    let choice = loop {
        if selected < offset {
            offset = selected;
        } else if selected >= offset + rows {
            offset = selected - rows + 1;
        }

        if !first {
            queue!(tty, MoveUp((rows + 1) as u16))?;
        }
        first = false;

        let cols = crossterm::terminal::size()
            .map(|(c, _)| c as usize)
            .unwrap_or(80);

        queue!(tty, MoveToColumn(0), Clear(ClearType::UntilNewLine))?;
        let header = format!(
            "── qmark ── {title} ── ↑↓ move · ⏎ insert · Esc close · {}/{}",
            selected + 1,
            entries.len()
        );
        write!(tty, "{}\r\n", fit(&header, cols))?;
        for (i, e) in entries.iter().enumerate().skip(offset).take(rows) {
            queue!(tty, MoveToColumn(0), Clear(ClearType::UntilNewLine))?;
            if i == selected {
                queue!(tty, SetAttribute(Attribute::Reverse))?;
            }
            let row = format!("  {:<width$}  {}", e.display, e.desc);
            write!(tty, "{}", fit(&row, cols))?;
            queue!(tty, SetAttribute(Attribute::Reset))?;
            write!(tty, "\r\n")?;
        }
        tty.flush()?;

        let Event::Key(key) = read()? else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Up | KeyCode::BackTab => {
                selected = selected.checked_sub(1).unwrap_or(entries.len() - 1);
            }
            KeyCode::Down | KeyCode::Tab => {
                selected = (selected + 1) % entries.len();
            }
            KeyCode::Enter => break Some(entries[selected].insert.clone()),
            KeyCode::Esc | KeyCode::Char('q') => break None,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => break None,
            _ => {}
        }
    };
    tty.flush()?;
    Ok(choice)
}

#[cfg(test)]
mod tests {
    use super::fit;

    #[test]
    fn fit_leaves_short_lines_alone() {
        assert_eq!(fit("abc", 10), "abc");
    }

    #[test]
    fn fit_truncates_with_ellipsis_at_width() {
        let out = fit("aaaaaaaaaa", 5);
        assert_eq!(out, "aaaa…");
        assert_eq!(out.chars().count(), 5);
    }

    #[test]
    fn fit_counts_chars_not_bytes() {
        assert_eq!(fit("── qmark ──", 20), "── qmark ──");
    }
}
