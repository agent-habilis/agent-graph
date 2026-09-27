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
  subgraph pod
    lead(("lead")):::public
    worker(("worker")):::private
    lead assign@-->|"assign a part"| worker
    worker result@-->|"part result"| lead
  end
  outside[["user or other graph"]]
  outside external@<--> lead
```
"#;

const LEAD: &str = "---\ntype: Node\ndescription: l\nmodel: opus\n---\n";
const WORKER: &str = "---\ntype: Node\ndescription: w\ncount: 2\nmodel: sonnet\n---\n";

struct Pod {
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

/// A project with the graph `pod` in `.agent-graph/`, and an empty home.
fn pod_with(graph: &str) -> Pod {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let root = base.join("project");
    let home = base.join("home");
    fs::create_dir_all(&home).unwrap();
    write_graph(&root.join(".agent-graph/pod"), graph);
    let out = run_in(&root, &home, &["graph", "hash", "pod"]);
    let hash = stdout(&out).trim().to_string();
    Pod {
        _tmp: tmp,
        root,
        home,
        hash,
    }
}

fn pod() -> Pod {
    pod_with(GRAPH)
}

/// The output of `agent-gossip state get` for the pod `pod@main`, with these
/// `nodes`.
fn state_nodes(hash: &str, nodes: &str) -> String {
    format!(
        r#"{{"ok":true,"document":{{"graphs":{{"pod@main":{{"bootstrapper":"ann","hash":"{hash}","nodes":{{{nodes}}}}}}}}}}}"#
    )
}

fn state(hash: &str, workers: &str) -> String {
    state_nodes(
        hash,
        &format!(
            r#""lead":{{"node":"pod/lead","count":1,"peers":["ann"]}},"worker":{{"node":"pod/worker","count":2,"peers":[{workers}]}}"#
        ),
    )
}

const META: &str = r#"{"ok":true,"document":{"peers":{
  "ann":{"model":"claude-opus-5-5","harness":"Claude Code","status":"busy","graph":"pod@main","node":"lead"},
  "bob":{"model":"claude-sonnet-5","status":"busy","graph":"pod@main","node":"worker"},
  "cy":{"model":"GPT-6","status":"idle","graph":"pod@main","node":"worker"},
  "dee":{"model":"claude-opus-5-5","status":"idle"}
}},"absent":[]}"#;

const ROSTER: &str = r#"{"ok":true,"peer_count":4,"peers":[
  {"nickname":"bob","quiet":false,"last_seen_secs_ago":3},
  {"nickname":"cy","quiet":true,"last_seen_secs_ago":90},
  {"nickname":"dee","quiet":false,"last_seen_secs_ago":1}
]}"#;

/// Write the three gossip files and run `topology` with them, and with
/// `args` after them.
fn run_topology(pod: &Pod, state: &str, meta: &str, roster: &str, args: &[&str]) -> Output {
    let dir = pod.root.join("gossip");
    write(&dir.join("state.json"), state);
    write(&dir.join("meta.json"), meta);
    write(&dir.join("peers.json"), roster);
    let files = [
        "--state",
        dir.join("state.json").to_str().unwrap(),
        "--meta",
        dir.join("meta.json").to_str().unwrap(),
        "--peers",
        dir.join("peers.json").to_str().unwrap(),
    ]
    .map(str::to_string);
    let mut all: Vec<&str> = vec!["topology"];
    all.extend(files.iter().map(String::as_str));
    all.extend(args);
    run_in(&pod.root, &pod.home, &all)
}

/// `run_topology` with `--mermaid`.
fn topology(pod: &Pod, state: &str, meta: &str, roster: &str, args: &[&str]) -> Output {
    let mut all = vec!["--mermaid"];
    all.extend(args);
    run_topology(pod, state, meta, roster, &all)
}

#[test]
fn topology_draws_each_peer_in_a_pod_box_with_its_edges() {
    let pod = pod();

    let out = topology(
        &pod,
        &state(&pod.hash, r#""bob","cy""#),
        META,
        ROSTER,
        &["--me", "ann"],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    let short = &pod.hash[..6];
    assert_eq!(
        stdout(&out),
        format!(
            "flowchart TD\n\
             \x20 outside[user or other graph]\n\
             \x20 subgraph \"pod@main · 3/3 · {short}\"\n\
             \x20   pod_main__lead__ann(lead · ann ← you ★ · claude-opus-5-5 · busy)\n\
             \x20   pod_main__worker__bob[worker · bob · claude-sonnet-5 · busy]\n\
             \x20   pod_main__worker__cy[worker · cy · GPT-6 · idle ⚠]\n\
             \x20 end\n\
             \x20 pod_main__lead__ann -->|assign a part| pod_main__worker__bob\n\
             \x20 pod_main__lead__ann -->|assign a part| pod_main__worker__cy\n\
             \x20 pod_main__worker__bob -->|part result| pod_main__lead__ann\n\
             \x20 pod_main__worker__cy -->|part result| pod_main__lead__ann\n\
             \x20 outside ---|external| pod_main__lead__ann\n"
        )
    );
    assert_eq!(
        stderr(&out),
        "warning: worker · cy · model GPT-6 does not match sonnet\n\
         warning: worker · cy · quiet\n"
    );
}

#[test]
fn topology_output_does_not_depend_on_the_key_order_of_the_input() {
    let pod = pod();
    let state = state(&pod.hash, r#""bob","cy""#);
    let meta = r#"{"absent":[],"document":{"peers":{
      "dee":{"status":"idle","model":"claude-opus-5-5"},
      "cy":{"node":"worker","graph":"pod@main","status":"idle","model":"GPT-6"},
      "bob":{"node":"worker","graph":"pod@main","status":"busy","model":"claude-sonnet-5"},
      "ann":{"node":"lead","graph":"pod@main","status":"busy","harness":"Claude Code","model":"claude-opus-5-5"}
    }},"ok":true}"#;
    let roster = r#"{"peers":[
      {"last_seen_secs_ago":1,"quiet":false,"nickname":"dee"},
      {"last_seen_secs_ago":90,"quiet":true,"nickname":"cy"},
      {"last_seen_secs_ago":3,"quiet":false,"nickname":"bob"}
    ],"peer_count":4,"ok":true}"#;

    let first = topology(&pod, &state, META, ROSTER, &["--me", "ann"]);
    let second = topology(&pod, &state, meta, roster, &["--me", "ann"]);

    assert_eq!(stdout(&first), stdout(&second));
    assert_eq!(stderr(&first), stderr(&second));
}

#[test]
fn topology_does_not_draw_open_places_and_warns() {
    let pod = pod();

    let out = topology(
        &pod,
        &state(&pod.hash, r#""bob""#),
        META,
        ROSTER,
        &["--me", "ann"],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("pod@main · 2/3 ·"), "{text}");
    assert!(!text.contains("__cy"), "{text}");
    assert_eq!(
        stderr(&out),
        "warning: worker · 1 open place\n\
         warning: worker · cy · in the meta, not in the state\n"
    );
}

#[test]
fn topology_takes_the_count_from_the_node_file_when_the_state_has_no_entry() {
    let pod = pod();
    let state = state_nodes(
        &pod.hash,
        r#""lead":{"node":"pod/lead","count":1,"peers":["ann"]}"#,
    );

    let out = topology(&pod, &state, META, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("pod@main · 1/3 ·"),
        "{}",
        stdout(&out)
    );
    assert_eq!(
        stderr(&out),
        "warning: worker · 2 open places\n\
         warning: worker · bob · in the meta, not in the state\n\
         warning: worker · cy · in the meta, not in the state\n"
    );
}

#[test]
fn topology_warns_on_more_peers_than_places() {
    let pod = pod();
    let meta = META.replace(
        r#""dee":{"model":"claude-opus-5-5","status":"idle"}"#,
        r#""dee":{"model":"claude-sonnet-5","status":"idle","graph":"pod@main","node":"worker"}"#,
    );

    let out = topology(
        &pod,
        &state(&pod.hash, r#""bob","dee""#).replace(r#""count":2"#, r#""count":1"#),
        &meta,
        ROSTER,
        &["--me", "ann"],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("pod@main · 3/2 ·"),
        "{}",
        stdout(&out)
    );
    assert_eq!(
        stderr(&out),
        "warning: worker · 2 peers for 1 place\n\
         warning: worker · cy · in the meta, not in the state\n"
    );
}

#[test]
fn topology_warns_on_a_gone_peer_and_a_meta_conflict() {
    let pod = pod();
    let meta = META.replace(
        r#""bob":{"model":"claude-sonnet-5","status":"busy","graph":"pod@main","node":"worker"}"#,
        r#""bob":{"model":"claude-sonnet-5","status":"busy","graph":"pod@main","node":"lead"}"#,
    );
    let roster = ROSTER.replace(
        r#"{"nickname":"bob","quiet":false,"last_seen_secs_ago":3},"#,
        "",
    );

    let out = topology(
        &pod,
        &state(&pod.hash, r#""bob","cy""#),
        &meta,
        &roster,
        &["--me", "ann"],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("pod_main__worker__bob[worker · bob · claude-sonnet-5 · busy ⚠]")
    );
    assert_eq!(
        stderr(&out),
        "warning: worker · bob · gone from the roster\n\
         warning: worker · bob · meta node lead does not match worker\n\
         warning: worker · cy · model GPT-6 does not match sonnet\n\
         warning: worker · cy · quiet\n"
    );
}

#[test]
fn topology_draws_a_peer_in_two_nodes_two_times_and_warns() {
    let pod = pod();
    let state = state_nodes(
        &pod.hash,
        r#""lead":{"node":"pod/lead","count":1,"peers":["ann"]},"worker":{"node":"pod/worker","count":2,"peers":["ann","bob","bob"]}"#,
    );

    let out = topology(&pod, &state, META, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("    pod_main__lead__ann("), "{text}");
    assert!(text.contains("    pod_main__worker__ann["), "{text}");
    assert_eq!(
        text.matches("    pod_main__worker__bob[").count(),
        1,
        "{text}"
    );
    assert!(text.contains("pod@main · 3/3 ·"), "{text}");
    assert_eq!(
        stderr(&out),
        "warning: lead · ann · in 2 nodes\n\
         warning: worker · ann · in 2 nodes\n\
         warning: worker · ann · meta node lead does not match worker\n\
         warning: worker · ann · model claude-opus-5-5 does not match sonnet\n\
         warning: worker · bob · listed 2 times\n\
         warning: worker · cy · in the meta, not in the state\n"
    );
}

#[test]
fn topology_warns_on_a_meta_peer_that_the_state_does_not_list() {
    let pod = pod();
    let meta = META.replace(
        r#""dee":{"model":"claude-opus-5-5","status":"idle"}"#,
        r#""dee":{"model":"claude-sonnet-5","status":"idle","graph":"pod@main","node":"worker"}"#,
    );

    let out = topology(
        &pod,
        &state(&pod.hash, r#""bob""#),
        &meta,
        ROSTER,
        &["--me", "ann"],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stderr(&out),
        "warning: worker · 1 open place\n\
         warning: worker · cy · in the meta, not in the state\n\
         warning: worker · dee · in the meta, not in the state\n"
    );
}

#[test]
fn topology_warns_on_a_state_node_that_the_graph_does_not_have() {
    let pod = pod();
    let state = state(&pod.hash, r#""bob","cy""#).replace(
        r#""lead":{"#,
        r#""ghost":{"node":"pod/ghost","count":1,"peers":["eve"]},"lead":{"#,
    );

    let out = topology(&pod, &state, META, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: ghost · not in the graph\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_removes_line_breaks_and_brackets_from_gossip_values() {
    let pod = pod();
    let meta = META.replace(
        r#""model":"claude-sonnet-5""#,
        r#""model":"claude-sonnet-5\n  end\n  hacked[injected]""#,
    );
    let state = state(&pod.hash, r#""bob","cy""#).replace("pod@main", "pod@feat/a\\\"b");
    let meta = meta.replace("pod@main", "pod@feat/a\\\"b");

    let out = topology(&pod, &state, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert_eq!(
        text.lines().filter(|line| line.trim() == "end").count(),
        1,
        "{text}"
    );
    assert!(!text.contains("hacked["), "{text}");
    assert!(text.contains("subgraph \"pod@feat/ab · 3/3 ·"), "{text}");
}

#[test]
fn topology_uses_the_edge_name_for_an_empty_label() {
    let pod = pod_with(&GRAPH.replace(r#"|"part result"|"#, "||"));

    let out = topology(
        &pod,
        &state(&pod.hash, r#""bob","cy""#),
        META,
        ROSTER,
        &["--me", "ann"],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("pod_main__worker__bob -->|result| pod_main__lead__ann"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn topology_takes_the_instance_argument() {
    let pod = pod();
    let state = state(&pod.hash, r#""bob","cy""#).replace("pod@main", "pod@my-repo");
    let meta = META.replace("pod@main", "pod@my-repo");

    let out = topology(&pod, &state, &meta, ROSTER, &["pod@my-repo", "--me", "dee"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("pod_my_repo__lead__ann(lead · ann ★ ·"));
}

#[test]
fn topology_fails_when_me_is_not_in_a_pod() {
    let pod = pod();

    let out = topology(
        &pod,
        &state(&pod.hash, r#""bob","cy""#),
        META,
        ROSTER,
        &["--me", "dee"],
    );

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("`dee` is not in a pod"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_fails_when_me_is_not_in_the_meta() {
    let pod = pod();

    let out = topology(
        &pod,
        &state(&pod.hash, r#""bob","cy""#),
        META,
        ROSTER,
        &["--me", "typo"],
    );

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("`typo` is not in the gossip meta"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_fails_when_a_gossip_command_failed() {
    let pod = pod();

    let out = topology(
        &pod,
        r#"{"ok":false,"error":"no session"}"#,
        META,
        ROSTER,
        &["--me", "ann"],
    );

    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("state.json"), "{err}");
    assert!(err.contains("no session"), "{err}");
}

#[test]
fn topology_finds_a_received_graph_by_hash_in_home() {
    let pod = pod();
    let received = pod.home.join(".agent-graph").join(&pod.hash);
    fs::create_dir_all(received.parent().unwrap()).unwrap();
    fs::rename(pod.root.join(".agent-graph/pod"), &received).unwrap();

    let out = topology(
        &pod,
        &state(&pod.hash, r#""bob","cy""#),
        META,
        ROSTER,
        &["--me", "ann"],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("pod_main__lead__ann -->|assign a part| pod_main__worker__bob"));
}

#[test]
fn topology_prefers_a_received_graph_with_the_hash_over_a_changed_local_graph() {
    let pod = pod();
    let local = pod.root.join(".agent-graph/pod");
    let received = pod.home.join(".agent-graph").join(&pod.hash);
    write_graph(&received, GRAPH);
    write(&local.join("nodes/worker/index.md"), &format!("{WORKER}\n"));

    let out = topology(
        &pod,
        &state(&pod.hash, r#""bob","cy""#),
        META,
        ROSTER,
        &["--me", "ann"],
    );

    assert!(out.status.success(), "{}", stderr(&out));
}

#[test]
fn topology_says_when_the_local_graph_has_another_hash() {
    let pod = pod();
    write(
        &pod.root.join(".agent-graph/pod/nodes/worker/index.md"),
        &format!("{WORKER}\n"),
    );

    let out = topology(
        &pod,
        &state(&pod.hash, r#""bob","cy""#),
        META,
        ROSTER,
        &["--me", "ann"],
    );

    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("graph `pod` has changed"), "{err}");
    assert!(err.contains(&pod.hash[..6]), "{err}");
}

#[test]
fn topology_fails_when_no_graph_has_the_hash() {
    let pod = pod();
    fs::remove_dir_all(pod.root.join(".agent-graph")).unwrap();

    let out = topology(
        &pod,
        &state("0000", r#""bob","cy""#),
        META,
        ROSTER,
        &["--me", "ann"],
    );

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("graph 0000 not found"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_draws_the_pod_as_terminal_text_by_default() {
    let pod = pod();

    let out = run_topology(
        &pod,
        &state(&pod.hash, r#""bob","cy""#),
        META,
        ROSTER,
        &["--me", "ann"],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains("flowchart"), "{text}");
    assert!(
        text.contains(&format!("pod@main · 3/3 · {}", &pod.hash[..6])),
        "{text}"
    );
    assert!(text.ends_with('\n'), "{text:?}");
}
