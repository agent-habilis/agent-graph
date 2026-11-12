//! Run commands from the rendered skills in `bash`, with a stub
//! `agent-gossip` that records its arguments.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn rendered_skill(dir: &Path, skill: &str) -> String {
    let dest = dir.join("skills");
    let out = Command::new(env!("CARGO_BIN_EXE_agent-graph"))
        .args(["plug", "--path", dest.to_str().unwrap()])
        .env("HOME", dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    fs::read_to_string(dest.join(skill).join("SKILL.md")).unwrap()
}

/// The first `bash` block after `heading` that contains `marker`.
fn bash_block(text: &str, heading: &str, marker: &str) -> String {
    let section = &text[text.find(heading).unwrap()..];
    section
        .split("```bash\n")
        .skip(1)
        .map(|block| &block[..block.find("```").unwrap()])
        .find(|block| block.contains(marker))
        .unwrap()
        .to_string()
}

/// A folder with an `agent-gossip` that writes each argument on its own line
/// (NUL-separated) to `args`.
fn stub(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let script = bin.join("agent-gossip");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\nfor arg in \"$@\"; do printf '%s\\0' \"$arg\"; done > {}\n",
            dir.join("args").display()
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    }
    bin
}

#[test]
fn node_offer_brief_runs_in_bash_and_gives_the_invitee_valid_meta_json() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let skill = rendered_skill(dir, "graph-node-invite");
    let command = bash_block(&skill, "## Node offer", "a2a call")
        .replace("<id>", "worker")
        .replace("<node>", "worker");
    let bin = stub(dir);

    let out = Command::new("bash")
        .args(["-c", &command])
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("GOSSIP", "g1")
        .env("INVITER", "ann")
        .env("PEER", "bob")
        .env("INSTANCE", "demo")
        .env("GRAPH", "squad1")
        .env("HASH", "abc123")
        .env("CONTEXT_HASH", "c0ffee")
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let args = fs::read_to_string(dir.join("args")).unwrap();
    let args: Vec<&str> = args.trim_end_matches('\0').split('\0').collect();
    let text = args[args.iter().position(|arg| *arg == "--text").unwrap() + 1];
    assert!(args.contains(&"bob"), "{args:?}");
    let merge = text
        .split("--merge '")
        .nth(1)
        .and_then(|rest| rest.split('\'').next())
        .unwrap_or_else(|| panic!("no merge in the brief: {text}"))
        .replace("<leader>", "")
        .replace("<epoch seconds>", "1700000000");
    let json: Value =
        serde_json::from_str(&merge).unwrap_or_else(|error| panic!("{error}: {merge}"));
    assert!(
        json.get("peers").is_none(),
        "the merge is the own slice: {merge}"
    );
    let entry = &json;
    assert_eq!(entry["instance"], "demo");
    assert_eq!(entry["graph"], "squad1");
    assert_eq!(entry["hash"], "abc123");
    assert_eq!(entry["node"], "worker");
    assert_eq!(entry["invited_by"], "ann");
    assert_eq!(entry["joined_at"], 1_700_000_000);
    assert!(text.contains("--to ann "), "{text}");
    assert!(
        text.contains("\"$NICKNAME\""),
        "the invitee's own nickname stays a variable: {text}"
    );
    assert!(text.contains("Context hash: c0ffee."), "{text}");
    assert!(text.contains("send node abc123/worker"), "{text}");
    let unverified = text
        .split("--merge '")
        .skip(1)
        .filter_map(|rest| rest.split('\'').next())
        .find(|candidate| candidate.contains("verified"))
        .unwrap_or_else(|| panic!("no merge with verified in the brief: {text}"))
        .replace("<leader>", "")
        .replace("<epoch seconds>", "1700000000");
    let unverified_json: Value =
        serde_json::from_str(&unverified).unwrap_or_else(|error| panic!("{error}: {unverified}"));
    let unverified_entry = &unverified_json;
    assert_eq!(unverified_entry["verified"], false);
    assert_eq!(unverified_entry["instance"], "demo");
    assert_eq!(unverified_entry["node"], "worker");
    assert_eq!(unverified_entry["joined_at"], 1_700_000_000);
}

/// Run a bash block in `cwd` with the real `agent-graph` and a stub
/// `agent-gossip` on `PATH`, and return its stdout.
fn run_block(block: &str, cwd: &Path, bin: &Path, env: &[(&str, &str)]) -> String {
    let agent_graph = Path::new(env!("CARGO_BIN_EXE_agent-graph"));
    let out = Command::new("bash")
        .args(["-c", block])
        .current_dir(cwd)
        .env(
            "PATH",
            format!(
                "{}:{}:/usr/bin:/bin",
                bin.display(),
                agent_graph.parent().unwrap().display()
            ),
        )
        .envs(env.iter().copied())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn send_node_gives_the_node_context_that_matches_the_context_hash_of_the_offer() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let project = dir.join("project");
    fs::create_dir_all(&project).unwrap();
    let init = Command::new(env!("CARGO_BIN_EXE_agent-graph"))
        .args(["graph", "init"])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(init.status.success());
    let skill = rendered_skill(dir, "graph-node-invite");
    let bin = stub(dir);
    let hash = format!("skills-test-{}", std::process::id());
    let env = [
        ("GOSSIP", "g1"),
        ("NICKNAME", "ann"),
        ("SOURCE", "squad1"),
        ("HASH", hash.as_str()),
    ];

    let hash_block = bash_block(&skill, "## Node offer", "CONTEXT_HASH=")
        .replace("<node>", "worker")
        + "\necho \"$CONTEXT_HASH\"\n";
    let context_hash = run_block(&hash_block, &project, &bin, &env);
    let send_block = bash_block(&skill, "## Drive", "graph node-up")
        .replace("<node>", "worker")
        .replace("<task id>", "t1");
    run_block(&send_block, &project, &bin, &env);

    let sent = PathBuf::from(format!("/tmp/{hash}-worker.md"));
    let body = fs::read(&sent).unwrap();
    let digest = Command::new("shasum")
        .args(["-a", "256"])
        .arg(&sent)
        .output()
        .unwrap()
        .stdout;
    fs::remove_file(&sent).unwrap();
    let expected = Command::new(env!("CARGO_BIN_EXE_agent-graph"))
        .args(["graph", "node-up", "squad1/worker"])
        .current_dir(&project)
        .output()
        .unwrap()
        .stdout;
    assert_eq!(body, expected);
    let digest = String::from_utf8(digest).unwrap();
    assert_eq!(
        context_hash.trim(),
        digest.split_whitespace().next().unwrap()
    );
    let args = fs::read_to_string(dir.join("args")).unwrap();
    assert!(args.contains("artifact"), "{args}");
    assert!(args.contains(&sent.display().to_string()), "{args}");
}

#[test]
fn the_lead_peer_marks_its_meta_entry_with_leader_true() {
    let tmp = TempDir::new().unwrap();
    let graph_up = rendered_skill(tmp.path(), "graph-up");
    let invite = rendered_skill(tmp.path(), "graph-node-invite");

    assert!(graph_up.contains("\"leader\":true"), "graph-up");
    assert!(invite.contains("Lead: <yes|no>"), "the offer brief");
    assert!(invite.contains(r#"\"leader\":true"#), "the offer brief");
}

#[test]
fn every_meta_clear_of_a_graph_entry_also_clears_leader() {
    let tmp = TempDir::new().unwrap();
    for skill in ["graph-up", "graph-leave", "graph-down"] {
        let text = rendered_skill(tmp.path(), skill).replace('\\', "");
        let clears = text.matches(r#""instance":null"#).count();

        assert!(clears > 0, "{skill} has no meta clear");
        assert_eq!(
            text.matches(r#""leader":null"#).count(),
            clears,
            "{skill}: a clear leaves leader:true behind"
        );
    }
}

#[test]
fn the_offer_brief_puts_leader_in_the_merge_json_for_a_lead_offer() {
    let tmp = TempDir::new().unwrap();
    let skill = rendered_skill(tmp.path(), "graph-node-invite");
    let brief = bash_block(&skill, "## Node offer", "a2a call");

    assert!(brief.contains("<leader>"), "no leader slot in the merge");
    assert!(
        brief.contains(r#"replace <leader> with \"leader\":true, if Lead is yes"#),
        "the brief does not say how to fill the slot"
    );
}

#[test]
fn no_graph_skill_writes_or_reads_the_peers_shape_of_the_meta() {
    let tmp = TempDir::new().unwrap();
    for skill in [
        "graph-up",
        "graph-leave",
        "graph-down",
        "graph-node-invite",
        "graph-topology",
    ] {
        let text = rendered_skill(tmp.path(), skill).replace('\\', "");

        for old in [r#"{"peers":"#, "document.peers", "/peers/"] {
            assert!(!text.contains(old), "{skill} still has `{old}`");
        }
    }
}

#[test]
fn the_graph_skills_find_their_own_entry_by_the_session_key() {
    let tmp = TempDir::new().unwrap();
    let topology = rendered_skill(tmp.path(), "graph-topology");

    assert!(topology.contains(r#"--me "$PUBKEY""#), "{topology}");
    for skill in ["graph-up", "graph-leave", "graph-down", "graph-node-invite"] {
        let text = rendered_skill(tmp.path(), skill);

        assert!(text.contains("document[$PUBKEY]"), "{skill}");
    }
}

#[test]
fn the_graph_meta_reference_reads_the_nickname_from_the_entry() {
    let tmp = TempDir::new().unwrap();
    for skill in ["graph-up", "graph-down", "graph-node-invite"] {
        let text = rendered_skill(tmp.path(), skill);

        assert!(text.contains("`.nickname`"), "{skill}");
        assert!(
            text.contains("Never join an entry to a roster row"),
            "{skill}"
        );
    }
}

#[test]
fn the_graph_skills_stop_without_a_session_key_and_break_a_nickname_tie_by_key() {
    let tmp = TempDir::new().unwrap();
    let up = rendered_skill(tmp.path(), "graph-up").replace('\n', " ");

    assert!(up.contains("If it has no `pubkey`"), "no stop");
    assert!(up.contains("nicknames are equal, the smaller key"), "tie");
}

#[test]
fn the_graph_meta_reference_says_a_restart_gives_a_new_key() {
    let tmp = TempDir::new().unwrap();
    let text = rendered_skill(tmp.path(), "graph-down");

    assert!(
        text.contains("A restart of the daemon gives a new key"),
        "{text}"
    );
    assert!(text.contains("The new entry is empty"), "{text}");
}

#[test]
fn the_default_graphs_find_the_leader_by_key_and_address_it_by_nickname() {
    for graph in ["squad1", "squad2", "pair"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("defaults/graphs")
            .join(graph)
            .join("index.md");
        let text = fs::read_to_string(&path).unwrap();

        assert!(text.contains("whose key is not in `absent`"), "{graph}");
        assert!(text.contains("`.nickname` of the leader entry"), "{graph}");
    }
}

#[test]
fn the_lines_that_name_the_own_entry_or_the_nickname_field_fit_in_80_characters() {
    for skill in ["graph-up", "graph-down", "graph-node-invite"] {
        let text = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("skills")
                .join(skill)
                .join("SKILL.md"),
        )
        .unwrap();
        for line in text.lines() {
            let marked = line.contains("document[$PUBKEY]") || line.contains("`.nickname`");

            assert!(!marked || line.chars().count() <= 80, "{skill}: {line}");
        }
    }
}

#[test]
fn the_no_gossip_stop_comes_before_the_no_session_key_stop() {
    let tmp = TempDir::new().unwrap();
    for skill in ["graph-up", "graph-down", "graph-leave", "graph-topology"] {
        let text = rendered_skill(tmp.path(), skill);
        let no_gossip = text.find("graph · no gossip").unwrap();
        let no_key = text.find("graph · no session key").unwrap();

        assert!(
            no_gossip < no_key,
            "{skill}: a session with no gossip says no key"
        );
    }
}

#[test]
fn the_restart_list_names_every_field_of_a_graph_entry() {
    let tmp = TempDir::new().unwrap();
    let text = rendered_skill(tmp.path(), "graph-down").replace('\n', " ");
    let start = text.find("merges the whole entry again").unwrap();
    let list = &text[start
        ..text[start..]
            .find("The nickname")
            .map_or(text.len(), |end| start + end)];

    for field in [
        "instance",
        "graph",
        "hash",
        "node",
        "invited_by",
        "joined_at",
        "verified",
        "status",
        "leader",
    ] {
        assert!(
            list.contains(&format!("`{field}`")),
            "the restart list lacks {field}: {list}"
        );
    }
}

#[test]
fn the_session_check_looks_for_an_empty_sessions_list() {
    let tmp = TempDir::new().unwrap();
    let text = rendered_skill(tmp.path(), "graph-up").replace('\n', " ");

    assert!(
        text.contains("If the `sessions` list of the output is empty"),
        "{text}"
    );
}
