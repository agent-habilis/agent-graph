// Cargo-style status output: a right-aligned (12-col) bold verb + message,
// written through `anstream`, which strips color when stderr is not a
// terminal so piped output stays clean.

use anstyle::{AnsiColor, Style};

/// A green status line: a right-aligned-12 bold-green `verb` then `msg`.
pub(crate) fn status(verb: &str, msg: &str) {
    let style = Style::new().fg_color(Some(AnsiColor::Green.into())).bold();
    anstream::eprintln!("{style}{verb:>12}{style:#} {msg}");
}

/// A cargo-style `error: {msg}` line (bold-red `error`).
pub(crate) fn error(msg: &str) {
    let style = Style::new().fg_color(Some(AnsiColor::Red.into())).bold();
    anstream::eprintln!("{style}error{style:#}: {msg}");
}
