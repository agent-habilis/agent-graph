//! Rewrites the slash commands in a rendered skill. `build.rs` and the tests share it.

use std::fmt::Write as _;

/// Keeps a slash command bare: the renderer drops the marker and leaves the
/// `/<skill>` that follows it as it is. It is for a text that a peer of any
/// agent reads.
const BARE: &str = "<!-- bare -->";

/// Drops each `BARE` marker. With a `namespace`, it also writes `/<skill>` as
/// `/<namespace>:<skill>` for each skill in `names`. A slash command stands
/// after a space, a quote (double or single), a backtick, a bracket, or at the start of a line. A
/// path such as `../shared/graph-meta.md` is not one.
pub(crate) fn namespace_commands(text: &str, names: &[String], namespace: Option<&str>) -> String {
    let Some(namespace) = namespace else {
        return text.replace(BARE, "");
    };
    let is_name_char = |ch: char| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_';
    let mut out = String::with_capacity(text.len());
    let mut before: Option<char> = None;
    let mut rest = text;
    let mut keep_bare = false;
    while let Some(next) = rest.chars().next() {
        if let Some(after_marker) = rest.strip_prefix(BARE) {
            keep_bare = true;
            rest = after_marker;
            continue;
        }
        if next == '/'
            && !keep_bare
            && before.is_none_or(|prev| matches!(prev, ' ' | '"' | '\'' | '`' | '(' | '[' | '\n'))
        {
            let word = &rest[1..];
            let len = word
                .find(|ch: char| !is_name_char(ch))
                .unwrap_or(word.len());
            if names.iter().any(|name| name == &word[..len]) {
                write!(out, "/{namespace}:{}", &word[..len]).expect("write to String");
                before = word[..len].chars().next_back();
                rest = &word[len..];
                continue;
            }
        }
        keep_bare = false;
        out.push(next);
        before = Some(next);
        rest = &rest[next.len_utf8()..];
    }
    out
}
