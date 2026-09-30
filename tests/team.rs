use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const TEMPLATE: &str = r#"---
type: TeamTemplate
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
  outside[["user or other team"]]
  outside external@<--> lead
```
"#;

const LEAD: &str = "---\ntype: Role\ndescription: l\nmodel: opus\n---\n";
const WORKER: &str = "---\ntype: Role\ndescription: w\ncount: 2\nmodel: sonnet\n---\n";

struct Team {
    _tmp: TempDir,
    root: PathBuf,
    home: PathBuf,
    hash: String,
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn write_template(dir: &Path, template: &str) {
    write(&dir.join("index.md"), template);
    write(&dir.join("roles/lead/index.md"), LEAD);
    write(&dir.join("roles/worker/index.md"), WORKER);
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

/// A project with the template `squad` in `.agent-graph/`, and an empty home.
fn team_with(template: &str) -> Team {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let root = base.join("project");
    let home = base.join("home");
    fs::create_dir_all(&home).unwrap();
    write_template(&root.join(".agent-graph/squad"), template);
    let out = run_in(&root, &home, &["template", "hash", "squad"]);
    let hash = stdout(&out).trim().to_string();
    Team {
        _tmp: tmp,
        root,
        home,
        hash,
    }
}

fn team() -> Team {
    team_with(TEMPLATE)
}

const ANN: &str = r#""ann":{"model":"claude-opus-5-5","harness":"Claude Code","status":"busy","team":"squad@main","template":"squad","hash":"HASH","role":"lead"}"#;
const BOB: &str = r#""bob":{"model":"claude-sonnet-5","status":"busy","team":"squad@main","template":"squad","hash":"HASH","role":"worker","invited_by":"ann"}"#;
const CY: &str = r#""cy":{"model":"GPT-6","status":"idle","team":"squad@main","template":"squad","hash":"HASH","role":"worker","invited_by":"ann"}"#;
const DEE: &str = r#""dee":{"model":"claude-opus-5-5","status":"idle"}"#;

const ROSTER: &str = r#"{"ok":true,"peer_count":4,"peers":[
  {"nickname":"bob","quiet":false,"last_seen_secs_ago":3},
  {"nickname":"cy","quiet":true,"last_seen_secs_ago":90},
  {"nickname":"dee","quiet":false,"last_seen_secs_ago":1}
]}"#;

/// The output of `agent-gossip meta get` with these entries, and the hash of
/// the squad template for `HASH`.
fn meta_of(team: &Team, entries: &[&str]) -> String {
    format!(
        r#"{{"ok":true,"document":{{"peers":{{{}}}}},"absent":[]}}"#,
        entries.join(",")
    )
    .replace("HASH", &team.hash)
}

fn meta(team: &Team) -> String {
    meta_of(team, &[ANN, BOB, CY, DEE])
}

/// `meta` with the entry of `dee` replaced by `entry`.
fn meta_with_dee(team: &Team, entry: &str) -> String {
    meta_of(team, &[ANN, BOB, CY, &format!(r#""dee":{entry}"#)])
}

/// Write the gossip files and run `team topology` with them, and with `args` after
/// them.
fn run_topology(team: &Team, meta: &str, roster: &str, args: &[&str]) -> Output {
    let dir = team.root.join("gossip");
    write(&dir.join("meta.json"), meta);
    write(&dir.join("peers.json"), roster);
    let files = [
        "--meta",
        dir.join("meta.json").to_str().unwrap(),
        "--peers",
        dir.join("peers.json").to_str().unwrap(),
    ]
    .map(str::to_string);
    let mut all: Vec<&str> = vec!["team", "topology"];
    all.extend(files.iter().map(String::as_str));
    all.extend(args);
    run_in(&team.root, &team.home, &all)
}

/// `run_topology` with `--mermaid`.
fn topology(team: &Team, meta: &str, roster: &str, args: &[&str]) -> Output {
    let mut all = vec!["--mermaid"];
    all.extend(args);
    run_topology(team, meta, roster, &all)
}

#[test]
fn team_topology_draws_each_member_in_a_team_box_with_its_handoffs() {
    let team = team();

    let out = topology(&team, &meta(&team), ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let short = &team.hash[..6];
    assert_eq!(
        stdout(&out),
        format!(
            "flowchart TD\n\
             \x20 outside[user or other team]\n\
             \x20 subgraph \"squad@main · 3 members · {short}\"\n\
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
fn team_topology_output_does_not_depend_on_the_key_order_of_the_input() {
    let team = team();
    let reordered = r#"{"absent":[],"document":{"peers":{
      "dee":{"status":"idle","model":"claude-opus-5-5"},
      "cy":{"invited_by":"ann","role":"worker","hash":"HASH","template":"squad","team":"squad@main","status":"idle","model":"GPT-6"},
      "bob":{"invited_by":"ann","role":"worker","hash":"HASH","template":"squad","team":"squad@main","status":"busy","model":"claude-sonnet-5"},
      "ann":{"role":"lead","hash":"HASH","template":"squad","team":"squad@main","status":"busy","harness":"Claude Code","model":"claude-opus-5-5"}
    }},"ok":true}"#
        .replace("HASH", &team.hash);
    let roster = r#"{"peers":[
      {"last_seen_secs_ago":1,"quiet":false,"nickname":"dee"},
      {"last_seen_secs_ago":90,"quiet":true,"nickname":"cy"},
      {"last_seen_secs_ago":3,"quiet":false,"nickname":"bob"}
    ],"peer_count":4,"ok":true}"#;

    let first = topology(&team, &meta(&team), ROSTER, &["--me", "ann"]);
    let second = topology(&team, &reordered, roster, &["--me", "ann"]);

    assert_eq!(stdout(&first), stdout(&second));
    assert_eq!(stderr(&first), stderr(&second));
}

#[test]
fn team_topology_does_not_draw_open_places_and_warns() {
    let team = team();
    let meta = meta_of(
        &team,
        &[ANN, BOB, r#""cy":{"model":"GPT-6","status":"idle"}"#, DEE],
    );

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("squad@main · 2 members ·"), "{text}");
    assert!(!text.contains("__cy"), "{text}");
    assert_eq!(stderr(&out), "warning: worker · 1 open place\n");
}

#[test]
fn team_topology_does_not_warn_on_more_members_than_count() {
    let team = team();
    let meta = meta_with_dee(
        &team,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","team":"squad@main","template":"squad","hash":"{}","role":"worker","invited_by":"bob"}}"#,
            team.hash
        ),
    );

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("squad@main · 4 members ·"), "{text}");
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
fn team_topology_does_not_count_an_absent_member() {
    let team = team();
    let meta = meta(&team).replace(r#""absent":[]"#, r#""absent":["cy"]"#);
    assert!(meta.contains(r#""absent":["cy"]"#));

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains("__cy"), "{text}");
    assert!(text.contains("squad@main · 2 members ·"), "{text}");
    assert_eq!(stderr(&out), "warning: worker · 1 open place\n");
}

#[test]
fn team_topology_warns_on_a_gone_member() {
    let team = team();
    let roster = ROSTER.replace(
        r#"{"nickname":"bob","quiet":false,"last_seen_secs_ago":3},"#,
        "",
    );

    let out = topology(&team, &meta(&team), &roster, &["--me", "ann"]);

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
fn team_topology_warns_on_a_partial_entry_and_does_not_draw_it() {
    let team = team();
    let meta = meta_with_dee(
        &team,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","team":"squad@main","template":"squad","hash":"{}","invited_by":"ann"}}"#,
            team.hash
        ),
    );

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stdout(&out).contains("__dee"), "{}", stdout(&out));
    assert!(
        stderr(&out).contains("warning: squad@main · dee · meta has no role\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn team_topology_warns_on_a_member_in_a_role_that_the_template_does_not_have() {
    let team = team();
    let meta = meta_with_dee(
        &team,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","team":"squad@main","template":"squad","hash":"{}","role":"ghost","invited_by":"ann"}}"#,
            team.hash
        ),
    );

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: ghost · dee · not in the template\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn team_topology_warns_on_two_members_of_the_lead_role() {
    let team = team();
    let meta = meta_with_dee(
        &team,
        &format!(
            r#"{{"model":"claude-opus-5-5","status":"busy","team":"squad@main","template":"squad","hash":"{}","role":"lead","invited_by":"ann"}}"#,
            team.hash
        ),
    );

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: lead · 2 members in the lead role\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn team_topology_warns_on_two_founders() {
    let team = team();
    let meta = meta_of(
        &team,
        &[ANN, &BOB.replace(r#","invited_by":"ann""#, ""), CY, DEE],
    );

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: squad@main · 2 founders · ann bob\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn team_topology_uses_the_hash_of_me_and_warns_on_another_hash() {
    let team = team();
    let meta = meta_with_dee(
        &team,
        r#"{"model":"claude-sonnet-5","status":"busy","team":"squad@main","template":"squad","hash":"0000","role":"worker","invited_by":"cy"}"#,
    );

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stdout(&out).contains("__dee"), "{}", stdout(&out));
    assert!(
        stderr(&out).contains("warning: squad@main · 1 member with another hash · dee\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn team_topology_fails_and_lists_hashes_when_not_a_member_and_hashes_differ() {
    let team = team();
    let meta = meta_of(&team, &[ANN, &BOB.replace("HASH", "000000ff"), CY, DEE]);

    let out = topology(&team, &meta, ROSTER, &["squad@main", "--me", "dee"]);

    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("team `squad@main` has 2 hashes"), "{err}");
    assert!(err.contains("000000"), "{err}");
    assert!(err.contains(&team.hash[..6]), "{err}");
}

#[test]
fn team_topology_ignores_an_old_entry_without_team() {
    let team = team();
    let meta = meta_with_dee(
        &team,
        r#"{"model":"claude-sonnet-5","status":"busy","graph":"squad@main","node":"worker"}"#,
    );

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stdout(&out).contains("__dee"), "{}", stdout(&out));
    assert!(!stderr(&out).contains("dee"), "{}", stderr(&out));
}

#[test]
fn team_topology_finds_the_template_by_hash_for_a_custom_instance_name() {
    let team = team();
    let meta = meta(&team).replace("squad@main", "demo");

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("demo__lead__ann -->|assign a part| demo__worker__bob"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn team_topology_removes_line_breaks_and_brackets_from_gossip_values() {
    let team = team();
    let meta = meta(&team)
        .replace(
            r#""model":"claude-sonnet-5""#,
            r#""model":"claude-sonnet-5\n  end\n  hacked[injected]""#,
        )
        .replace("squad@main", "squad@feat/a\\\"b");

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert_eq!(
        text.lines().filter(|line| line.trim() == "end").count(),
        1,
        "{text}"
    );
    assert!(!text.contains("hacked["), "{text}");
    assert!(
        text.contains("subgraph \"squad@feat/ab · 3 members ·"),
        "{text}"
    );
}

#[test]
fn team_topology_uses_the_handoff_name_for_an_empty_label() {
    let team = team_with(&TEMPLATE.replace(r#"|"part result"|"#, "||"));

    let out = topology(&team, &meta(&team), ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("squad_main__worker__bob -->|result| squad_main__lead__ann"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn team_topology_takes_the_instance_argument() {
    let team = team();
    let meta = meta(&team).replace("squad@main", "squad@my-repo");

    let out = topology(&team, &meta, ROSTER, &["squad@my-repo", "--me", "dee"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("squad_my_repo__lead__ann(lead · ann · claude-opus-5-5 · busy"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn team_topology_fails_when_the_team_has_no_members() {
    let team = team();

    let out = topology(&team, &meta(&team), ROSTER, &["nope", "--me", "ann"]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("team `nope` has no members"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn team_topology_fails_when_me_is_not_in_a_team() {
    let team = team();

    let out = topology(&team, &meta(&team), ROSTER, &["--me", "dee"]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("`dee` is not in a team"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn team_topology_fails_when_me_is_not_in_the_meta() {
    let team = team();

    let out = topology(&team, &meta(&team), ROSTER, &["--me", "typo"]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("`typo` is not in the gossip meta"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn team_topology_fails_when_a_gossip_command_failed() {
    let team = team();

    let out = topology(
        &team,
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
fn team_topology_finds_a_received_template_by_hash_in_home() {
    let team = team();
    let received = team.home.join(".agent-graph").join(&team.hash);
    fs::create_dir_all(received.parent().unwrap()).unwrap();
    fs::rename(team.root.join(".agent-graph/squad"), &received).unwrap();

    let out = topology(&team, &meta(&team), ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("squad_main__lead__ann -->|assign a part| squad_main__worker__bob")
    );
}

#[test]
fn team_topology_prefers_a_received_template_with_the_hash_over_a_changed_local_template() {
    let team = team();
    let local = team.root.join(".agent-graph/squad");
    let received = team.home.join(".agent-graph").join(&team.hash);
    write_template(&received, TEMPLATE);
    write(&local.join("roles/worker/index.md"), &format!("{WORKER}\n"));

    let out = topology(&team, &meta(&team), ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
}

#[test]
fn team_topology_says_when_the_local_template_has_another_hash() {
    let team = team();
    write(
        &team.root.join(".agent-graph/squad/roles/worker/index.md"),
        &format!("{WORKER}\n"),
    );

    let out = topology(&team, &meta(&team), ROSTER, &["--me", "ann"]);

    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains("template `squad` has changed"), "{err}");
    assert!(err.contains(&team.hash[..6]), "{err}");
}

#[test]
fn team_topology_fails_when_no_template_has_the_hash() {
    let team = team();
    fs::remove_dir_all(team.root.join(".agent-graph")).unwrap();
    let meta = meta(&team).replace(&team.hash, "0000");

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("template 0000 not found"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn team_topology_draws_the_team_as_terminal_text_by_default() {
    let team = team();

    let out = run_topology(&team, &meta(&team), ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains("flowchart"), "{text}");
    assert!(
        text.contains(&format!("squad@main · 3 members · {}", &team.hash[..6])),
        "{text}"
    );
    assert!(text.ends_with('\n'), "{text:?}");
}

#[test]
fn team_topology_warns_on_an_invited_by_that_is_not_a_member() {
    let team = team();
    let meta = meta_with_dee(
        &team,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","team":"squad@main","template":"squad","hash":"{}","role":"worker","invited_by":"zed"}}"#,
            team.hash
        ),
    );

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("warning: worker · dee · invited_by zed is not a member\n"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn team_topology_draws_the_team_when_me_has_a_partial_entry() {
    let team = team();
    let meta = meta_with_dee(
        &team,
        r#"{"model":"claude-sonnet-5","status":"busy","team":"squad@main","template":"squad","role":"worker","invited_by":"ann"}"#,
    );

    let out = topology(&team, &meta, ROSTER, &["--me", "dee"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("squad@main · 3 members ·"),
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
fn team_topology_fails_when_no_entry_of_the_team_has_a_hash() {
    let team = team();
    let meta =
        meta_of(&team, &[ANN, BOB, CY, DEE]).replace(&format!(r#""hash":"{}","#, team.hash), "");

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("team `squad@main` has no hash in the gossip meta"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn team_topology_does_not_count_an_absent_second_founder() {
    let team = team();
    let meta = meta_of(
        &team,
        &[ANN, &BOB.replace(r#","invited_by":"ann""#, ""), CY, DEE],
    )
    .replace(r#""absent":[]"#, r#""absent":["bob"]"#);

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stderr(&out).contains("founders"), "{}", stderr(&out));
}

#[test]
fn team_topology_finds_a_local_template_by_hash_under_another_folder_name() {
    let team = team();
    let meta = meta(&team).replace(r#""template":"squad""#, r#""template":"renamed""#);

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("squad_main__lead__ann -->|assign a part| squad_main__worker__bob")
    );
}

#[test]
fn team_topology_warns_on_a_member_that_did_not_verify_the_template() {
    let team = team();
    let meta = meta_with_dee(
        &team,
        &format!(
            r#"{{"model":"claude-sonnet-5","status":"busy","team":"squad@main","template":"squad","hash":"{}","role":"worker","invited_by":"ann","verified":false}}"#,
            team.hash
        ),
    );

    let out = topology(&team, &meta, ROSTER, &["--me", "ann"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("squad_main__worker__dee[worker · dee · claude-sonnet-5 · busy ⚠]"),
        "{}",
        stdout(&out)
    );
    assert!(
        stderr(&out).contains("warning: worker · dee · template not verified\n"),
        "{}",
        stderr(&out)
    );
}
