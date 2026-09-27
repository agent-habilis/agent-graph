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

const ANN: &str = r#""ann":{"model":"claude-opus-5-5","harness":"Claude Code","status":"busy","pod":"pod@main","graph":"pod","hash":"HASH","node":"lead"}"#;
const BOB: &str = r#""bob":{"model":"claude-sonnet-5","status":"busy","pod":"pod@main","graph":"pod","hash":"HASH","node":"worker","invited_by":"ann"}"#;
const CY: &str = r#""cy":{"model":"GPT-6","status":"idle","pod":"pod@main","graph":"pod","hash":"HASH","node":"worker","invited_by":"ann"}"#;
const DEE: &str = r#""dee":{"model":"claude-opus-5-5","status":"idle"}"#;

const ROSTER: &str = r#"{"ok":true,"peer_count":4,"peers":[
  {"nickname":"bob","quiet":false,"last_seen_secs_ago":3},
  {"nickname":"cy","quiet":true,"last_seen_secs_ago":90},
  {"nickname":"dee","quiet":false,"last_seen_secs_ago":1}
]}"#;

/// The output of `agent-gossip meta get` with these entries, and the hash of
/// the pod graph for `HASH`.
fn meta_of(pod: &Pod, entries: &[&str]) -> String {
    format!(
        r#"{{"ok":true,"document":{{"peers":{{{}}}}},"absent":[]}}"#,
        entries.join(",")
    )
    .replace("HASH", &pod.hash)
}

fn meta(pod: &Pod) -> String {
    meta_of(pod, &[ANN, BOB, CY, DEE])
}

/// `meta` with the entry of `dee` replaced by `entry`.
fn meta_with_dee(pod: &Pod, entry: &str) -> String {
    meta_of(pod, &[ANN, BOB, CY, &format!(r#""dee":{entry}"#)])
}

/// Write the gossip files and run `topology` with them, and with `args` after
/// them.
fn run_topology(pod: &Pod, meta: &str, roster: &str, args: &[&str]) -> Output {
    let dir = pod.root.join("gossip");
    write(&dir.join("meta.json"), meta);
    write(&dir.join("peers.json"), roster);
    let files = [
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
fn topology(pod: &Pod, meta: &str, roster: &str, args: &[&str]) -> Output {
    let mut all = vec!["--mermaid"];
    all.extend(args);
    run_topology(pod, meta, roster, &all)
}

#[test]
fn topology_draws_each_member_in_a_pod_box_with_its_edges() {
    let pod = pod();

    let out = topology(&pod, &meta(&pod), ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let short = &pod.hash[..6];
    assert_eq!(
        stdout(&out),
        format!(
            "flowchart TD\n\
             \x20 outside[user or other graph]\n\
             \x20 subgraph \"pod@main · 3 peers · {short}\"\n\
             \x20   pod_main__lead__ann(lead · ann ← you · claude-opus-5-5 · busy)\n\
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
    let reordered = r#"{"absent":[],"document":{"peers":{
      "dee":{"status":"idle","model":"claude-opus-5-5"},
      "cy":{"invited_by":"ann","node":"worker","hash":"HASH","graph":"pod","pod":"pod@main","status":"idle","model":"GPT-6"},
      "bob":{"invited_by":"ann","node":"worker","hash":"HASH","graph":"pod","pod":"pod@main","status":"busy","model":"claude-sonnet-5"},
      "ann":{"node":"lead","hash":"HASH","graph":"pod","pod":"pod@main","status":"busy","harness":"Claude Code","model":"claude-opus-5-5"}
    }},"ok":true}"#
        .replace("HASH", &pod.hash);
    let roster = r#"{"peers":[
      {"last_seen_secs_ago":1,"quiet":false,"nickname":"dee"},
      {"last_seen_secs_ago":90,"quiet":true,"nickname":"cy"},
      {"last_seen_secs_ago":3,"quiet":false,"nickname":"bob"}
    ],"peer_count":4,"ok":true}"#;

    let first = topology(&pod, &meta(&pod), ROSTER, &["--me", "ann"]);
    let second = topology(&pod, &reordered, roster, &["--me", "ann"]);

    assert_eq!(stdout(&first), stdout(&second));
    assert_eq!(stderr(&first), stderr(&second));
}

#[test]
fn topology_does_not_draw_open_places_and_warns() {
    let pod = pod();
    let meta = meta_of(
        &pod,
        &[ANN, BOB, r#""cy":{"model":"GPT-6","status":"idle"}"#, DEE],
    );

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("pod@main · 2 peers ·"), "{text}");
    assert!(!text.contains("__cy"), "{text}");
    assert_eq!(stderr(&out), "warning: worker · 1 open place\n");
}

#[test]
fn topology_does_not_warn_on_more_peers_than_count() {
    let pod = pod();
    let meta = meta_with_dee(
        &pod,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","pod":"pod@main","graph":"pod","hash":"{}","node":"worker","invited_by":"bob"}}"#,
            pod.hash
        ),
    );

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("pod@main · 4 peers ·"), "{text}");
    assert!(
        text.contains("pod_main__worker__dee[worker · dee ·"),
        "{text}"
    );
    assert_eq!(
        stderr(&out),
        "warning: worker · cy · model GPT-6 does not match sonnet\n\
         warning: worker · cy · quiet\n"
    );
}

#[test]
fn topology_does_not_count_an_absent_peer() {
    let pod = pod();
    let meta = meta(&pod).replace(r#""absent":[]"#, r#""absent":["cy"]"#);
    assert!(meta.contains(r#""absent":["cy"]"#));

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains("__cy"), "{text}");
    assert!(text.contains("pod@main · 2 peers ·"), "{text}");
    assert_eq!(stderr(&out), "warning: worker · 1 open place\n");
}

#[test]
fn topology_warns_on_a_gone_peer() {
    let pod = pod();
    let roster = ROSTER.replace(
        r#"{"nickname":"bob","quiet":false,"last_seen_secs_ago":3},"#,
        "",
    );

    let out = topology(&pod, &meta(&pod), &roster, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("pod_main__worker__bob[worker · bob · claude-sonnet-5 · busy ⚠]")
    );
    assert_eq!(
        stderr(&out),
        "warning: worker · bob · gone from the roster\n\
         warning: worker · cy · model GPT-6 does not match sonnet\n\
         warning: worker · cy · quiet\n"
    );
}

#[test]
fn topology_warns_on_a_partial_entry_and_does_not_draw_it() {
    let pod = pod();
    let meta = meta_with_dee(
        &pod,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","pod":"pod@main","graph":"pod","hash":"{}","invited_by":"ann"}}"#,
            pod.hash
        ),
    );

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stdout(&out).contains("__dee"), "{}", stdout(&out));
    assert!(
        stderr(&out).contains("warning: pod@main · dee · meta has no node\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_warns_on_a_member_in_a_node_that_the_graph_does_not_have() {
    let pod = pod();
    let meta = meta_with_dee(
        &pod,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","pod":"pod@main","graph":"pod","hash":"{}","node":"ghost","invited_by":"ann"}}"#,
            pod.hash
        ),
    );

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: ghost · dee · not in the graph\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_warns_on_two_members_of_the_public_node() {
    let pod = pod();
    let meta = meta_with_dee(
        &pod,
        &format!(
            r#"{{"model":"claude-opus-5-5","status":"busy","pod":"pod@main","graph":"pod","hash":"{}","node":"lead","invited_by":"ann"}}"#,
            pod.hash
        ),
    );

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: lead · 2 peers in a public node\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_warns_on_two_members_without_invited_by() {
    let pod = pod();
    let meta = meta_of(
        &pod,
        &[ANN, &BOB.replace(r#","invited_by":"ann""#, ""), CY, DEE],
    );

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: pod@main · 2 peers without invited_by · ann bob\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_uses_the_hash_of_me_and_warns_on_another_hash() {
    let pod = pod();
    let meta = meta_with_dee(
        &pod,
        r#"{"model":"claude-sonnet-5","status":"busy","pod":"pod@main","graph":"pod","hash":"0000","node":"worker","invited_by":"cy"}"#,
    );

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stdout(&out).contains("__dee"), "{}", stdout(&out));
    assert!(
        stderr(&out).contains("warning: pod@main · 1 peer with another hash · dee\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_fails_and_lists_hashes_when_not_a_member_and_hashes_differ() {
    let pod = pod();
    let meta = meta_of(&pod, &[ANN, &BOB.replace("HASH", "000000ff"), CY, DEE]);

    let out = topology(&pod, &meta, ROSTER, &["pod@main", "--me", "dee"]);

    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("pod `pod@main` has 2 hashes"), "{err}");
    assert!(err.contains("000000"), "{err}");
    assert!(err.contains(&pod.hash[..6]), "{err}");
}

#[test]
fn topology_ignores_an_old_entry_without_pod() {
    let pod = pod();
    let meta = meta_with_dee(
        &pod,
        r#"{"model":"claude-sonnet-5","status":"busy","graph":"pod@main","node":"worker"}"#,
    );

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stdout(&out).contains("__dee"), "{}", stdout(&out));
    assert!(!stderr(&out).contains("dee"), "{}", stderr(&out));
}

#[test]
fn topology_finds_the_graph_by_hash_for_a_custom_instance_name() {
    let pod = pod();
    let meta = meta(&pod).replace("pod@main", "demo");

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("demo__lead__ann -->|assign a part| demo__worker__bob"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn topology_removes_line_breaks_and_brackets_from_gossip_values() {
    let pod = pod();
    let meta = meta(&pod)
        .replace(
            r#""model":"claude-sonnet-5""#,
            r#""model":"claude-sonnet-5\n  end\n  hacked[injected]""#,
        )
        .replace("pod@main", "pod@feat/a\\\"b");

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert_eq!(
        text.lines().filter(|line| line.trim() == "end").count(),
        1,
        "{text}"
    );
    assert!(!text.contains("hacked["), "{text}");
    assert!(
        text.contains("subgraph \"pod@feat/ab · 3 peers ·"),
        "{text}"
    );
}

#[test]
fn topology_uses_the_edge_name_for_an_empty_label() {
    let pod = pod_with(&GRAPH.replace(r#"|"part result"|"#, "||"));

    let out = topology(&pod, &meta(&pod), ROSTER, &["--me", "ann"]);

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
    let meta = meta(&pod).replace("pod@main", "pod@my-repo");

    let out = topology(&pod, &meta, ROSTER, &["pod@my-repo", "--me", "dee"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("pod_my_repo__lead__ann(lead · ann · claude-opus-5-5 · busy"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn topology_fails_when_the_pod_has_no_members() {
    let pod = pod();

    let out = topology(&pod, &meta(&pod), ROSTER, &["nope", "--me", "ann"]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("pod `nope` has no members"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_fails_when_me_is_not_in_a_pod() {
    let pod = pod();

    let out = topology(&pod, &meta(&pod), ROSTER, &["--me", "dee"]);

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

    let out = topology(&pod, &meta(&pod), ROSTER, &["--me", "typo"]);

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
        ROSTER,
        &["--me", "ann"],
    );

    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("meta.json"), "{err}");
    assert!(err.contains("no session"), "{err}");
}

#[test]
fn topology_finds_a_received_graph_by_hash_in_home() {
    let pod = pod();
    let received = pod.home.join(".agent-graph").join(&pod.hash);
    fs::create_dir_all(received.parent().unwrap()).unwrap();
    fs::rename(pod.root.join(".agent-graph/pod"), &received).unwrap();

    let out = topology(&pod, &meta(&pod), ROSTER, &["--me", "ann"]);

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

    let out = topology(&pod, &meta(&pod), ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
}

#[test]
fn topology_says_when_the_local_graph_has_another_hash() {
    let pod = pod();
    write(
        &pod.root.join(".agent-graph/pod/nodes/worker/index.md"),
        &format!("{WORKER}\n"),
    );

    let out = topology(&pod, &meta(&pod), ROSTER, &["--me", "ann"]);

    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("graph `pod` has changed"), "{err}");
    assert!(err.contains(&pod.hash[..6]), "{err}");
}

#[test]
fn topology_fails_when_no_graph_has_the_hash() {
    let pod = pod();
    fs::remove_dir_all(pod.root.join(".agent-graph")).unwrap();
    let meta = meta(&pod).replace(&pod.hash, "0000");

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

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

    let out = run_topology(&pod, &meta(&pod), ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains("flowchart"), "{text}");
    assert!(
        text.contains(&format!("pod@main · 3 peers · {}", &pod.hash[..6])),
        "{text}"
    );
    assert!(text.ends_with('\n'), "{text:?}");
}

#[test]
fn topology_warns_on_an_invited_by_that_is_not_a_member() {
    let pod = pod();
    let meta = meta_with_dee(
        &pod,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","pod":"pod@main","graph":"pod","hash":"{}","node":"worker","invited_by":"zed"}}"#,
            pod.hash
        ),
    );

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: worker · dee · invited_by zed is not a member\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_draws_the_pod_when_me_has_a_partial_entry() {
    let pod = pod();
    let meta = meta_with_dee(
        &pod,
        r#"{"model":"claude-sonnet-5","status":"busy","pod":"pod@main","graph":"pod","node":"worker","invited_by":"ann"}"#,
    );

    let out = topology(&pod, &meta, ROSTER, &["--me", "dee"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("pod@main · 3 peers ·"),
        "{}",
        stdout(&out)
    );
    assert!(
        stderr(&out).contains("warning: pod@main · dee · meta has no hash\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_fails_when_no_entry_of_the_pod_has_a_hash() {
    let pod = pod();
    let meta =
        meta_of(&pod, &[ANN, BOB, CY, DEE]).replace(&format!(r#""hash":"{}","#, pod.hash), "");

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("pod `pod@main` has no hash in the gossip meta"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_does_not_count_an_absent_second_starter() {
    let pod = pod();
    let meta = meta_of(
        &pod,
        &[ANN, &BOB.replace(r#","invited_by":"ann""#, ""), CY, DEE],
    )
    .replace(r#""absent":[]"#, r#""absent":["bob"]"#);

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        !stderr(&out).contains("without invited_by"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn topology_finds_a_local_graph_by_hash_under_another_folder_name() {
    let pod = pod();
    let meta = meta(&pod).replace(r#""graph":"pod""#, r#""graph":"renamed""#);

    let out = topology(&pod, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("pod_main__lead__ann -->|assign a part| pod_main__worker__bob"));
}
