use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const GRAPH: &str = r#"---
type: Graph
description: d
scope: branch
---

```mermaid
flowchart LR
  subgraph squad
    lead(("lead")):::lead
    worker(("worker"))
    lead assign@-->|"assign a part"| worker
    worker result@-->|"part result"| lead
  end
  outside[["user or other graph"]]
  outside external@<--> lead
```
"#;

const LEAD: &str = "---\ntype: Node\ndescription: l\nmodel: opus\n---\n";
const WORKER: &str = "---\ntype: Node\ndescription: w\ncount: 2\nmodel: sonnet\n---\n";

struct Graph {
    _tmp: TempDir,
    root: PathBuf,
    home: PathBuf,
    hash: String,
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn write_graph(dir: &Path, graph: &str) {
    write(&dir.join("index.md"), graph);
    write(&dir.join("nodes/lead/index.md"), LEAD);
    write(&dir.join("nodes/worker/index.md"), WORKER);
}

fn run_in(cwd: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agent-graph"))
        .args(args)
        .current_dir(cwd)
        .env("HOME", home)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

/// A project with the graph `squad` in `.agent-graph/`, and an empty home.
fn graph_with(graph: &str) -> Graph {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let root = base.join("project");
    let home = base.join("home");
    fs::create_dir_all(&home).unwrap();
    write_graph(&root.join(".agent-graph/squad"), graph);
    let out = run_in(&root, &home, &["graph", "hash", "squad"]);
    let hash = stdout(&out).trim().to_string();
    Graph {
        _tmp: tmp,
        root,
        home,
        hash,
    }
}

fn graph() -> Graph {
    graph_with(GRAPH)
}

const ANN: &str = r#""ann":{"model":"claude-opus-5-5","harness":"Claude Code","status":"busy","instance":"squad@main","graph":"squad","hash":"HASH","node":"lead"}"#;
const BOB: &str = r#""bob":{"model":"claude-sonnet-5","status":"busy","instance":"squad@main","graph":"squad","hash":"HASH","node":"worker","invited_by":"ann"}"#;
const CY: &str = r#""cy":{"model":"GPT-6","status":"idle","instance":"squad@main","graph":"squad","hash":"HASH","node":"worker","invited_by":"ann"}"#;
const DEE: &str = r#""dee":{"model":"claude-opus-5-5","status":"idle"}"#;

const ROSTER: &str = r#"{"ok":true,"peer_count":4,"peers":[
  {"nickname":"bob","quiet":false,"last_seen_secs_ago":3},
  {"nickname":"cy","quiet":true,"last_seen_secs_ago":90},
  {"nickname":"dee","quiet":false,"last_seen_secs_ago":1}
]}"#;

/// The output of `agent-gossip meta get` with these entries, and the hash of
/// the squad graph for `HASH`.
fn meta_of(graph: &Graph, entries: &[&str]) -> String {
    format!(
        r#"{{"ok":true,"document":{{"peers":{{{}}}}},"absent":[]}}"#,
        entries.join(",")
    )
    .replace("HASH", &graph.hash)
}

fn meta(graph: &Graph) -> String {
    meta_of(graph, &[ANN, BOB, CY, DEE])
}

/// `meta` with the entry of `dee` replaced by `entry`.
fn meta_with_dee(graph: &Graph, entry: &str) -> String {
    meta_of(graph, &[ANN, BOB, CY, &format!(r#""dee":{entry}"#)])
}

/// Write the gossip files and run `graph topology` with them, and with `args` after
/// them.
fn run_topology(graph: &Graph, meta: &str, roster: &str, args: &[&str]) -> Output {
    let dir = graph.root.join("gossip");
    write(&dir.join("meta.json"), meta);
    write(&dir.join("peers.json"), roster);
    let files = [
        "--meta",
        dir.join("meta.json").to_str().unwrap(),
        "--peers",
        dir.join("peers.json").to_str().unwrap(),
    ]
    .map(str::to_string);
    let mut all: Vec<&str> = vec!["graph", "topology"];
    all.extend(files.iter().map(String::as_str));
    all.extend(args);
    run_in(&graph.root, &graph.home, &all)
}

/// `run_topology` with `--mermaid`.
fn topology(graph: &Graph, meta: &str, roster: &str, args: &[&str]) -> Output {
    let mut all = vec!["--mermaid"];
    all.extend(args);
    run_topology(graph, meta, roster, &all)
}

#[test]
fn graph_topology_draws_each_peer_in_a_graph_box_with_its_edges() {
    let graph = graph();

    let out = topology(&graph, &meta(&graph), ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let short = &graph.hash[..6];
    assert_eq!(
        stdout(&out),
        format!(
            "flowchart TD\n\
             \x20 outside[user or other graph]\n\
             \x20 subgraph \"squad@main · 3 peers · {short}\"\n\
             \x20   squad_main__lead__ann(lead · ann ← you · claude-opus-5-5 · busy)\n\
             \x20   squad_main__worker__bob[worker · bob · claude-sonnet-5 · busy]\n\
             \x20   squad_main__worker__cy[worker · cy · GPT-6 · idle ⚠]\n\
             \x20 end\n\
             \x20 squad_main__lead__ann -->|assign a part| squad_main__worker__bob\n\
             \x20 squad_main__lead__ann -->|assign a part| squad_main__worker__cy\n\
             \x20 squad_main__worker__bob -->|part result| squad_main__lead__ann\n\
             \x20 squad_main__worker__cy -->|part result| squad_main__lead__ann\n\
             \x20 outside ---|external| squad_main__lead__ann\n"
        )
    );
    assert_eq!(
        stderr(&out),
        "warning: worker · cy · model GPT-6 does not match sonnet\n\
         warning: worker · cy · quiet\n"
    );
}

#[test]
fn graph_topology_output_does_not_depend_on_the_key_order_of_the_input() {
    let graph = graph();
    let reordered = r#"{"absent":[],"document":{"peers":{
      "dee":{"status":"idle","model":"claude-opus-5-5"},
      "cy":{"invited_by":"ann","node":"worker","hash":"HASH","graph":"squad","instance":"squad@main","status":"idle","model":"GPT-6"},
      "bob":{"invited_by":"ann","node":"worker","hash":"HASH","graph":"squad","instance":"squad@main","status":"busy","model":"claude-sonnet-5"},
      "ann":{"node":"lead","hash":"HASH","graph":"squad","instance":"squad@main","status":"busy","harness":"Claude Code","model":"claude-opus-5-5"}
    }},"ok":true}"#
        .replace("HASH", &graph.hash);
    let roster = r#"{"peers":[
      {"last_seen_secs_ago":1,"quiet":false,"nickname":"dee"},
      {"last_seen_secs_ago":90,"quiet":true,"nickname":"cy"},
      {"last_seen_secs_ago":3,"quiet":false,"nickname":"bob"}
    ],"peer_count":4,"ok":true}"#;

    let first = topology(&graph, &meta(&graph), ROSTER, &["--me", "ann"]);
    let second = topology(&graph, &reordered, roster, &["--me", "ann"]);

    assert_eq!(stdout(&first), stdout(&second));
    assert_eq!(stderr(&first), stderr(&second));
}

#[test]
fn graph_topology_does_not_draw_open_places_and_warns() {
    let graph = graph();
    let meta = meta_of(
        &graph,
        &[ANN, BOB, r#""cy":{"model":"GPT-6","status":"idle"}"#, DEE],
    );

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("squad@main · 2 peers ·"), "{text}");
    assert!(!text.contains("__cy"), "{text}");
    assert_eq!(stderr(&out), "warning: worker · 1 open place\n");
}

#[test]
fn graph_topology_does_not_warn_on_more_peers_than_count() {
    let graph = graph();
    let meta = meta_with_dee(
        &graph,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","instance":"squad@main","graph":"squad","hash":"{}","node":"worker","invited_by":"bob"}}"#,
            graph.hash
        ),
    );

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("squad@main · 4 peers ·"), "{text}");
    assert!(
        text.contains("squad_main__worker__dee[worker · dee ·"),
        "{text}"
    );
    assert_eq!(
        stderr(&out),
        "warning: worker · cy · model GPT-6 does not match sonnet\n\
         warning: worker · cy · quiet\n"
    );
}

#[test]
fn graph_topology_does_not_count_an_absent_peer() {
    let graph = graph();
    let meta = meta(&graph).replace(r#""absent":[]"#, r#""absent":["cy"]"#);
    assert!(meta.contains(r#""absent":["cy"]"#));

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains("__cy"), "{text}");
    assert!(text.contains("squad@main · 2 peers ·"), "{text}");
    assert_eq!(stderr(&out), "warning: worker · 1 open place\n");
}

#[test]
fn graph_topology_warns_on_a_gone_peer() {
    let graph = graph();
    let roster = ROSTER.replace(
        r#"{"nickname":"bob","quiet":false,"last_seen_secs_ago":3},"#,
        "",
    );

    let out = topology(&graph, &meta(&graph), &roster, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("squad_main__worker__bob[worker · bob · claude-sonnet-5 · busy ⚠]")
    );
    assert_eq!(
        stderr(&out),
        "warning: worker · bob · gone from the roster\n\
         warning: worker · cy · model GPT-6 does not match sonnet\n\
         warning: worker · cy · quiet\n"
    );
}

#[test]
fn graph_topology_warns_on_a_partial_entry_and_does_not_draw_it() {
    let graph = graph();
    let meta = meta_with_dee(
        &graph,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","instance":"squad@main","graph":"squad","hash":"{}","invited_by":"ann"}}"#,
            graph.hash
        ),
    );

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stdout(&out).contains("__dee"), "{}", stdout(&out));
    assert!(
        stderr(&out).contains("warning: squad@main · dee · meta has no node\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn graph_topology_warns_on_a_peer_in_a_node_that_the_graph_does_not_have() {
    let graph = graph();
    let meta = meta_with_dee(
        &graph,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","instance":"squad@main","graph":"squad","hash":"{}","node":"ghost","invited_by":"ann"}}"#,
            graph.hash
        ),
    );

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: ghost · dee · not in the graph\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn graph_topology_warns_on_two_peers_of_the_lead_node() {
    let graph = graph();
    let meta = meta_with_dee(
        &graph,
        &format!(
            r#"{{"model":"claude-opus-5-5","status":"busy","instance":"squad@main","graph":"squad","hash":"{}","node":"lead","invited_by":"ann"}}"#,
            graph.hash
        ),
    );

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: lead · 2 peers in the lead node\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn graph_topology_warns_on_two_founders() {
    let graph = graph();
    let meta = meta_of(
        &graph,
        &[ANN, &BOB.replace(r#","invited_by":"ann""#, ""), CY, DEE],
    );

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: squad@main · 2 founders · ann bob\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn graph_topology_uses_the_hash_of_me_and_warns_on_another_hash() {
    let graph = graph();
    let meta = meta_with_dee(
        &graph,
        r#"{"model":"claude-sonnet-5","status":"busy","instance":"squad@main","graph":"squad","hash":"0000","node":"worker","invited_by":"cy"}"#,
    );

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stdout(&out).contains("__dee"), "{}", stdout(&out));
    assert!(
        stderr(&out).contains("warning: squad@main · 1 peer with another hash · dee\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn graph_topology_fails_and_lists_hashes_when_not_a_peer_and_hashes_differ() {
    let graph = graph();
    let meta = meta_of(&graph, &[ANN, &BOB.replace("HASH", "000000ff"), CY, DEE]);

    let out = topology(&graph, &meta, ROSTER, &["squad@main", "--me", "dee"]);

    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("graph `squad@main` has 2 hashes"), "{err}");
    assert!(err.contains("000000"), "{err}");
    assert!(err.contains(&graph.hash[..6]), "{err}");
}

#[test]
fn graph_topology_ignores_an_old_entry_without_graph() {
    let graph = graph();
    let meta = meta_with_dee(
        &graph,
        r#"{"model":"claude-sonnet-5","status":"busy","graph":"squad@main","node":"worker"}"#,
    );

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stdout(&out).contains("__dee"), "{}", stdout(&out));
    assert!(!stderr(&out).contains("dee"), "{}", stderr(&out));
}

#[test]
fn graph_topology_finds_the_graph_by_hash_for_a_custom_instance_name() {
    let graph = graph();
    let meta = meta(&graph).replace("squad@main", "demo");

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("demo__lead__ann -->|assign a part| demo__worker__bob"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn graph_topology_removes_line_breaks_and_brackets_from_gossip_values() {
    let graph = graph();
    let meta = meta(&graph)
        .replace(
            r#""model":"claude-sonnet-5""#,
            r#""model":"claude-sonnet-5\n  end\n  hacked[injected]""#,
        )
        .replace("squad@main", "squad@feat/a\\\"b");

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert_eq!(
        text.lines().filter(|line| line.trim() == "end").count(),
        1,
        "{text}"
    );
    assert!(!text.contains("hacked["), "{text}");
    assert!(
        text.contains("subgraph \"squad@feat/ab · 3 peers ·"),
        "{text}"
    );
}

#[test]
fn graph_topology_uses_the_edge_name_for_an_empty_label() {
    let graph = graph_with(&GRAPH.replace(r#"|"part result"|"#, "||"));

    let out = topology(&graph, &meta(&graph), ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("squad_main__worker__bob -->|result| squad_main__lead__ann"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn graph_topology_takes_the_instance_argument() {
    let graph = graph();
    let meta = meta(&graph).replace("squad@main", "squad@my-repo");

    let out = topology(&graph, &meta, ROSTER, &["squad@my-repo", "--me", "dee"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("squad_my_repo__lead__ann(lead · ann · claude-opus-5-5 · busy"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn graph_topology_fails_when_the_graph_has_no_peers() {
    let graph = graph();

    let out = topology(&graph, &meta(&graph), ROSTER, &["nope", "--me", "ann"]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("graph `nope` has no peers"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn graph_topology_fails_when_me_is_not_in_a_graph() {
    let graph = graph();

    let out = topology(&graph, &meta(&graph), ROSTER, &["--me", "dee"]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("`dee` is not in a graph"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn graph_topology_fails_when_me_is_not_in_the_meta() {
    let graph = graph();

    let out = topology(&graph, &meta(&graph), ROSTER, &["--me", "typo"]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("`typo` is not in the gossip meta"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn graph_topology_fails_when_a_gossip_command_failed() {
    let graph = graph();

    let out = topology(
        &graph,
        r#"{"ok":false,"error":"no session"}"#,
        ROSTER,
        &["--me", "ann"],
    );

    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("meta.json"), "{err}");
    assert!(err.contains("no session"), "{err}");
}

#[test]
fn graph_topology_finds_a_received_graph_by_hash_in_home() {
    let graph = graph();
    let received = graph.home.join(".agent-graph").join(&graph.hash);
    fs::create_dir_all(received.parent().unwrap()).unwrap();
    fs::rename(graph.root.join(".agent-graph/squad"), &received).unwrap();

    let out = topology(&graph, &meta(&graph), ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("squad_main__lead__ann -->|assign a part| squad_main__worker__bob")
    );
}

#[test]
fn graph_topology_prefers_a_received_graph_with_the_hash_over_a_changed_local_graph() {
    let graph = graph();
    let local = graph.root.join(".agent-graph/squad");
    let received = graph.home.join(".agent-graph").join(&graph.hash);
    write_graph(&received, GRAPH);
    write(&local.join("nodes/worker/index.md"), &format!("{WORKER}\n"));

    let out = topology(&graph, &meta(&graph), ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
}

#[test]
fn graph_topology_says_when_the_local_graph_has_another_hash() {
    let graph = graph();
    write(
        &graph.root.join(".agent-graph/squad/nodes/worker/index.md"),
        &format!("{WORKER}\n"),
    );

    let out = topology(&graph, &meta(&graph), ROSTER, &["--me", "ann"]);

    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("graph `squad` has changed"), "{err}");
    assert!(err.contains(&graph.hash[..6]), "{err}");
}

#[test]
fn graph_topology_fails_when_no_graph_has_the_hash() {
    let graph = graph();
    fs::remove_dir_all(graph.root.join(".agent-graph")).unwrap();
    let meta = meta(&graph).replace(&graph.hash, "0000");

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("graph 0000 not found"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn graph_topology_draws_the_graph_as_terminal_text_by_default() {
    let graph = graph();

    let out = run_topology(&graph, &meta(&graph), ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains("flowchart"), "{text}");
    assert!(
        text.contains(&format!("squad@main · 3 peers · {}", &graph.hash[..6])),
        "{text}"
    );
    assert!(text.ends_with('\n'), "{text:?}");
}

#[test]
fn graph_topology_warns_on_an_invited_by_that_is_not_a_peer() {
    let graph = graph();
    let meta = meta_with_dee(
        &graph,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","instance":"squad@main","graph":"squad","hash":"{}","node":"worker","invited_by":"zed"}}"#,
            graph.hash
        ),
    );

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: worker · dee · invited_by zed is not a peer\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn graph_topology_draws_the_graph_when_me_has_a_partial_entry() {
    let graph = graph();
    let meta = meta_with_dee(
        &graph,
        r#"{"model":"claude-sonnet-5","status":"busy","instance":"squad@main","graph":"squad","node":"worker","invited_by":"ann"}"#,
    );

    let out = topology(&graph, &meta, ROSTER, &["--me", "dee"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("squad@main · 3 peers ·"),
        "{}",
        stdout(&out)
    );
    assert!(
        stderr(&out).contains("warning: squad@main · dee · meta has no hash\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn graph_topology_fails_when_no_entry_of_the_graph_has_a_hash() {
    let graph = graph();
    let meta =
        meta_of(&graph, &[ANN, BOB, CY, DEE]).replace(&format!(r#""hash":"{}","#, graph.hash), "");

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("graph `squad@main` has no hash in the gossip meta"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn graph_topology_does_not_count_an_absent_second_founder() {
    let graph = graph();
    let meta = meta_of(
        &graph,
        &[ANN, &BOB.replace(r#","invited_by":"ann""#, ""), CY, DEE],
    )
    .replace(r#""absent":[]"#, r#""absent":["bob"]"#);

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stderr(&out).contains("founders"), "{}", stderr(&out));
}

#[test]
fn graph_topology_finds_a_local_graph_by_hash_under_another_folder_name() {
    let graph = graph();
    let meta = meta(&graph).replace(r#""graph":"squad""#, r#""graph":"renamed""#);

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("squad_main__lead__ann -->|assign a part| squad_main__worker__bob")
    );
}

#[test]
fn graph_topology_warns_on_a_peer_that_did_not_verify_the_graph() {
    let graph = graph();
    let meta = meta_with_dee(
        &graph,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","instance":"squad@main","graph":"squad","hash":"{}","node":"worker","invited_by":"ann","verified":false}}"#,
            graph.hash
        ),
    );

    let out = topology(&graph, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("squad_main__worker__dee[worker · dee · claude-sonnet-5 · busy ⚠]"),
        "{}",
        stdout(&out)
    );
    assert!(
        stderr(&out).contains("warning: worker · dee · graph not verified\n"),
        "{}",
        stderr(&out)
    );
}
