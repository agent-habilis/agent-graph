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
