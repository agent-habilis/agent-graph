//! The renderer that names each slash command of an agy skill with the plugin
//! namespace. The build script uses the same file.

#[path = "../build_support/namespace.rs"]
mod namespace;

fn names() -> Vec<String> {
    vec!["graph-up".to_string(), "graph-leave".to_string()]
}

fn agy(text: &str) -> String {
    namespace::namespace_commands(text, &names(), Some("agent-graph"))
}

#[test]
fn a_slash_command_after_a_single_quote_gets_the_namespace() {
    assert_eq!(
        agy("run '/graph-leave' now"),
        "run '/agent-graph:graph-leave' now"
    );
}

#[test]
fn a_slash_command_after_each_other_boundary_gets_the_namespace() {
    for before in [" ", "\"", "`", "(", "["] {
        assert_eq!(
            agy(&format!("run{before}/graph-up now")),
            format!("run{before}/agent-graph:graph-up now"),
            "before {before:?}"
        );
    }
    assert_eq!(agy("/graph-up"), "/agent-graph:graph-up");
}

#[test]
fn a_path_or_a_longer_name_is_not_a_slash_command() {
    assert_eq!(agy("../shared/graph-up.md"), "../shared/graph-up.md");
    assert_eq!(agy("see /graph-upgrade"), "see /graph-upgrade");
}

#[test]
fn the_bare_marker_keeps_a_command_bare_in_each_render() {
    assert_eq!(agy("<!-- bare -->/graph-leave"), "/graph-leave");
    assert_eq!(
        namespace::namespace_commands("<!-- bare -->/graph-leave", &names(), None),
        "/graph-leave"
    );
}

#[test]
fn the_agy_render_writes_pid_auto_where_a_command_has_pid_ppid() {
    assert_eq!(
        agy("agent-graph graph node-down --pid \"$PPID\"\n"),
        "agent-graph graph node-down --pid auto\n"
    );
    assert_eq!(
        agy("agent-graph node up <node> --name \"$NAME\" --pid \"$PPID\""),
        "agent-graph node up <node> --name \"$NAME\" --pid auto"
    );
}

#[test]
fn the_agy_render_leaves_other_uses_of_ppid_and_an_escaped_one_in_a_brief() {
    for text in [
        "agent-gossip session --session-pid \\\"\\$PPID\\\"",
        "run agent-graph graph node-up x --pid \\\"\\$PPID\\\" now",
        "`$PPID` is the agent process",
    ] {
        assert_eq!(agy(text), text);
    }
}

#[test]
fn the_agy_render_writes_the_conversation_where_a_command_has_session_pid() {
    assert_eq!(
        agy("agent-gossip session --session-pid \"$PPID\"\n"),
        "agent-gossip session --session $CONVERSATION\n"
    );
}

#[test]
fn the_agy_render_writes_a_placeholder_inside_a_single_quoted_text_without_a_quote_juggle() {
    assert_eq!(
        agy("--merge '{\"a\":\"'\"$INSTANCE\"'\",\"b\":'\"$JOINED_AT\"',\"c\":1}'"),
        "--merge '{\"a\":\"$INSTANCE\",\"b\":$JOINED_AT,\"c\":1}'"
    );
    for text in ["'\"$x y\"'", "'\"$\"'", "a '\"$A", "'\"$A\" '"] {
        assert_eq!(agy(text), text, "{text}");
    }
}

#[test]
fn the_default_render_keeps_the_quote_juggle() {
    let text = "--merge '{\"a\":\"'\"$INSTANCE\"'\"}'";
    assert_eq!(namespace::namespace_commands(text, &names(), None), text);
}

#[test]
fn the_default_render_keeps_session_pid_ppid() {
    let text = "agent-gossip session --session-pid \"$PPID\"";
    assert_eq!(namespace::namespace_commands(text, &names(), None), text);
}

#[test]
fn the_default_render_keeps_pid_ppid() {
    let text = "agent-graph graph node-down --pid \"$PPID\"";
    assert_eq!(namespace::namespace_commands(text, &names(), None), text);
}
