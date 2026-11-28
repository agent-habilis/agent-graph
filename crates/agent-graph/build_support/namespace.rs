//! Rewrites the slash commands in a rendered skill. `build.rs` and the tests share it.

use std::fmt::Write as _;

/// Keeps a slash command bare: the renderer drops the marker and leaves the
/// `/<skill>` that follows it as it is. It is for a text that a peer of any
/// agent reads.
const BARE: &str = "<!-- bare -->";

/// The `--pid` argument of the other agents, and its `agy` form.
const PID_PPID: &str = "--pid \"$PPID\"";
const PID_AUTO: &str = "--pid auto";

/// The `--session-pid` argument of the other agents, and its `agy` form. `agy`
/// holds a conversation id, which `agent-gossip session --session` finds
/// without `$PPID`.
const SESSION_PPID: &str = "--session-pid \"$PPID\"";
const SESSION_CONVERSATION: &str = "--session $CONVERSATION";

/// Drops each `BARE` marker. With a `namespace`, it also writes `--pid "$PPID"` as
/// `--pid auto`, `--session-pid "$PPID"` as `--session $CONVERSATION`, and writes `/<skill>` as
/// `/<namespace>:<skill>` for each skill in `names`. A slash command stands
/// after a space, a quote (double or single), a backtick, a bracket, or at the start of a line. A
/// path such as `../shared/graph-meta.md` is not one.
pub(crate) fn namespace_commands(text: &str, names: &[String], namespace: Option<&str>) -> String {
    let Some(namespace) = namespace else {
        return text.replace(BARE, "");
    };
    // `agy` reads a shell-free command line: `--pid auto` finds the agent
    // without `$PPID`.
    let text = unjuggle(
        &text
            .replace(PID_PPID, PID_AUTO)
            .replace(SESSION_PPID, SESSION_CONVERSATION),
    );
    let text = text.as_str();
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

/// Writes each `'"$NAME"'` as `$NAME`. A shell closes a single-quoted text,
/// quotes the variable, and opens the text again. An agent that fills in the
/// placeholder writes the value inside the one text instead.
fn unjuggle(text: &str) -> String {
    const OPEN: &str = "'\"$";
    const CLOSE: &str = "\"'";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(OPEN) {
        let after = &rest[at + OPEN.len()..];
        let len = after
            .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .unwrap_or(after.len());
        let named = after
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_');
        if named && after[len..].starts_with(CLOSE) {
            out.push_str(&rest[..at]);
            out.push('$');
            out.push_str(&after[..len]);
            rest = &after[len + CLOSE.len()..];
        } else {
            out.push_str(&rest[..at + OPEN.len()]);
            rest = after;
        }
    }
    out.push_str(rest);
    out
}
