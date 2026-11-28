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

    let hash_block =
        bash_block(&skill, "## Node offer", "graph node-hash").replace("<node>", "worker");
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
    assert!(invite.contains("Lead: <yes or no>"), "the offer brief");
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
        for text in [
            rendered_skill(tmp.path(), skill),
            agy_rendered_skill(tmp.path(), skill),
        ] {
            let text = text.replace('\\', "");

            for old in [r#"{"peers":"#, "document.peers", "/peers/"] {
                assert!(!text.contains(old), "{skill} still has `{old}`");
            }
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
fn the_restart_list_names_every_field_of_the_entry_block() {
    let tmp = TempDir::new().unwrap();
    let text = rendered_skill(tmp.path(), "graph-down");
    let block = text
        .split("```json\n")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .unwrap();
    let fields: Vec<&str> = block
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix('"'))
        .filter_map(|rest| rest.split('"').next())
        .collect();
    let flat = text.replace('\n', " ");
    let start = flat.find("merges the whole entry again").unwrap();
    let end = flat[start..].find("The nickname").unwrap();
    let list = &flat[start..start + end];

    assert!(
        fields.len() >= 9,
        "the block lists too few fields: {fields:?}"
    );
    for field in fields {
        assert!(
            list.contains(&format!("`{field}`")),
            "the restart list lacks {field}: {list}"
        );
    }
}

#[test]
fn the_entry_block_holds_each_field_that_a_bullet_documents() {
    let tmp = TempDir::new().unwrap();
    let text = rendered_skill(tmp.path(), "graph-down");
    let block = text
        .split("```json\n")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .unwrap();

    for line in text.lines() {
        let Some(field) = line
            .strip_prefix("- `")
            .and_then(|rest| rest.split_once("` is"))
            .map(|(field, _)| field)
        else {
            continue;
        };
        assert!(block.contains(&format!("\"{field}\"")), "{field}: {block}");
    }
    assert!(block.contains("\"verified\""), "{block}");
    assert!(block.contains("\"leader\""), "{block}");
}

#[test]
fn the_session_check_stops_on_more_than_one_session_after_no_gossip() {
    let tmp = TempDir::new().unwrap();
    for skill in ["graph-up", "graph-down", "graph-leave", "graph-topology"] {
        let text = rendered_skill(tmp.path(), skill).replace('\n', " ");
        let none = text.find("graph · no gossip").unwrap();
        let many = text
            .find("If `sessions` has more than 1 entry")
            .unwrap_or_else(|| panic!("{skill}: no stop on many sessions"));
        let no_key = text.find("graph · no session key").unwrap();

        assert!(none < many && many < no_key, "{skill}: order of the stops");
    }
}

#[test]
fn every_skill_and_shared_file_keeps_prose_lines_within_80_characters() {
    let skills = Path::new(env!("CARGO_MANIFEST_DIR")).join("skills");
    let mut files = Vec::new();
    for entry in fs::read_dir(&skills).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        if path.ends_with("shared") {
            for file in fs::read_dir(&path).unwrap() {
                files.push(file.unwrap().path());
            }
        } else {
            files.push(path.join("SKILL.md"));
        }
    }
    for file in files
        .iter()
        .filter(|file| file.extension().is_some_and(|ext| ext == "md"))
    {
        let text = fs::read_to_string(file).unwrap();
        let body = if text.starts_with("---\n") {
            text.splitn(3, "---\n").nth(2).unwrap()
        } else {
            &text
        };
        let mut fenced = false;
        for line in body.lines() {
            if line.trim_start().starts_with("```") {
                fenced = !fenced;
                continue;
            }

            assert!(
                fenced || line.chars().count() <= 80,
                "{}: {line}",
                file.display()
            );
        }
    }
}

#[test]
fn the_entry_block_says_that_it_shows_every_field() {
    let tmp = TempDir::new().unwrap();
    let text = rendered_skill(tmp.path(), "graph-down");

    assert!(
        text.contains("The block shows every field of an entry:\n\n```json"),
        "{text}"
    );
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

/// The skill folders in the source tree: every `graph-*` folder with a `SKILL.md`.
fn source_skills() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("skills");
    let mut skills: Vec<(String, String)> = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.join("SKILL.md").is_file())
        .map(|path| {
            let name = path.file_name().unwrap().to_str().unwrap().to_string();
            (name, fs::read_to_string(path.join("SKILL.md")).unwrap())
        })
        .collect();
    skills.sort();
    assert_ne!(skills.len(), 0, "no skill");
    skills
}

#[test]
fn every_skill_tells_agy_to_run_its_commands_outside_the_sandbox() {
    let tmp = TempDir::new().unwrap();
    for (name, _) in source_skills() {
        let body = rendered_skill(tmp.path(), &name);
        assert!(body.contains("BypassSandbox"), "{name}: no sandbox note");
        // The commands that the skills run now: no `tar`, `ls`, `curl`,
        // `mkdir`, or `cp`.
        for command in ["`agent-graph`", "`agent-gossip`"] {
            assert!(
                body.contains(command),
                "{name}: the sandbox note omits {command}"
            );
        }
        for command in ["`tar`", "`ls`", "`curl`", "`mkdir`", "`cp`"] {
            assert!(
                !body.contains(command),
                "{name}: the sandbox note names {command}, which no skill runs"
            );
        }
        assert!(body.contains("`~/.agent-graph`"), "{name}: no folder named");
    }
}

#[test]
fn graph_trait_run_gives_agy_a_subagent_that_carries_its_own_context() {
    let tmp = TempDir::new().unwrap();
    let body = rendered_skill(tmp.path(), "graph-trait-run");
    for needle in [
        "invoke_subagent",
        "cannot fork the conversation",
        "`TypeName` `self`",
        "subject of the run",
        "`## Boundaries`",
        "`BypassSandbox`",
    ] {
        assert!(body.contains(needle), "no {needle}");
    }
    assert!(
        body.contains("In `agy`, the subagent does not get a copy of this context"),
        "the intro still says that the fork copies the context"
    );
}

#[test]
fn the_node_offer_brief_tells_an_agy_invitee_to_bypass_the_sandbox() {
    let tmp = TempDir::new().unwrap();
    for skill in ["graph-up", "graph-node-invite"] {
        let body = rendered_skill(tmp.path(), skill);
        let brief = body
            .lines()
            .find(|line| line.contains("--text \"Node offer."))
            .unwrap_or_else(|| panic!("{skill}: no brief"));
        assert!(
            brief.contains("In agy, run each command of these steps")
                && brief.contains("BypassSandbox true"),
            "{skill}: the brief has no sandbox rule"
        );
    }
}

fn agy_rendered_skill(dir: &Path, skill: &str) -> String {
    fs::create_dir_all(dir.join(".gemini/antigravity-cli")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_agent-graph"))
        .args(["plug", "--agent", "agy"])
        .env("HOME", dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    fs::read_to_string(
        dir.join(".gemini/config/plugins/agent-graph/skills")
            .join(skill)
            .join("SKILL.md"),
    )
    .unwrap()
}

/// Each place where `body` names the slash command `/<name>` bare: after a
/// space, a quote (double or single), a backtick, a bracket, or at the start of a line, and not
/// followed by more of a name.
fn bare_commands(body: &str, name: &str) -> usize {
    let command = format!("/{name}");
    body.match_indices(&command)
        .filter(|(at, _)| {
            let before = body[..*at].chars().next_back();
            let after = body[at + command.len()..].chars().next();
            before.is_none_or(|ch| matches!(ch, ' ' | '"' | '\'' | '`' | '(' | '[' | '\n'))
                && after.is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
        })
        .count()
}

#[test]
fn the_agy_render_names_each_slash_command_with_the_plugin_namespace() {
    let tmp = TempDir::new().unwrap();
    let names: Vec<String> = source_skills().into_iter().map(|(name, _)| name).collect();
    let mut namespaced = 0;
    for skill in &names {
        // The down broadcast is for peers of every agent, so it keeps the bare
        // form on purpose. Another test pins it.
        let body: String = agy_rendered_skill(tmp.path(), skill)
            .lines()
            .filter(|line| !line.contains("is down · each peer"))
            .collect::<Vec<_>>()
            .join("\n");
        for name in &names {
            assert_eq!(
                bare_commands(&body, name),
                0,
                "{skill}: names `/{name}` without the namespace"
            );
            namespaced += body.matches(&format!("/agent-graph:{name}")).count();
        }
    }
    assert!(namespaced > 20, "{namespaced}");
}

#[test]
fn the_default_render_keeps_the_bare_slash_commands() {
    let tmp = TempDir::new().unwrap();
    let body = rendered_skill(tmp.path(), "graph-up");
    assert!(bare_commands(&body, "graph-leave") > 0);
    assert!(!body.contains("/agent-graph:"));
}

#[test]
fn the_down_broadcast_is_the_same_in_each_render_and_names_both_forms() {
    let tmp = TempDir::new().unwrap();
    let broadcast = |body: String| {
        body.lines()
            .find(|line| line.contains("is down · each peer"))
            .map(str::to_string)
            .expect("no down broadcast")
    };
    let bare = broadcast(rendered_skill(tmp.path(), "graph-down"));
    let agy = broadcast(agy_rendered_skill(tmp.path(), "graph-down"));

    assert_eq!(bare, agy, "peers of every agent read this text");
    assert!(bare.contains("/graph-leave"), "{bare}");
    assert!(bare.contains("/agent-graph:graph-leave"), "{bare}");
}

#[test]
fn the_agy_sandbox_rule_of_the_offer_brief_follows_the_a_or_b_choice() {
    let tmp = TempDir::new().unwrap();
    for skill in ["graph-up", "graph-node-invite"] {
        let body = rendered_skill(tmp.path(), skill);
        let brief = body
            .lines()
            .find(|line| line.contains("--text \"Node offer."))
            .unwrap_or_else(|| panic!("{skill}: no brief"));
        let rule = brief
            .find("In agy, run each command of these steps")
            .unwrap_or_else(|| panic!("{skill}: no sandbox rule"));
        let choice = brief.find("else do B.").unwrap();
        let first_step = brief.find("A1.").unwrap();

        assert!(
            brief.contains("decline with the reason: has a node. Else, after you accept"),
            "{skill}: the rule splits the decline sentence from its Else"
        );
        assert!(
            choice < rule && rule < first_step,
            "{skill}: the rule is not between the A-or-B choice and A1"
        );
        assert!(
            brief.contains("each command that reads or writes under ~/.agent-graph"),
            "{skill}: the rule does not cover each command under ~/.agent-graph"
        );
    }
}

#[test]
fn the_agy_sandbox_rule_applies_only_when_run_command_has_the_parameter() {
    let tmp = TempDir::new().unwrap();
    for (name, _) in source_skills() {
        let body = rendered_skill(tmp.path(), &name).replace('\n', " ");
        assert!(
            body.contains("has the parameter `BypassSandbox`")
                || body.contains("has the `BypassSandbox` parameter"),
            "{name}: the rule does not depend on the parameter"
        );
        assert!(
            body.contains("already runs outside the sandbox"),
            "{name}: no word on a run_command without the parameter"
        );
    }
    for skill in ["graph-up", "graph-node-invite"] {
        let body = rendered_skill(tmp.path(), skill);
        let brief = body
            .lines()
            .find(|line| line.contains("--text \"Node offer."))
            .unwrap_or_else(|| panic!("{skill}: no brief"));
        assert!(
            brief.contains("if run_command has that parameter")
                && brief.contains("Without it the command already runs outside the sandbox"),
            "{skill}: the brief rule does not depend on the parameter"
        );
    }
    let readme = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md"))
        .unwrap()
        .replace('\n', " ");
    assert!(
        readme.contains("has the parameter `BypassSandbox`")
            && readme.contains("already runs outside the sandbox"),
        "README: the rule does not depend on the parameter"
    );
}

#[test]
fn graph_trait_run_tells_agy_not_to_poll_the_subagent() {
    let tmp = TempDir::new().unwrap();
    let body = rendered_skill(tmp.path(), "graph-trait-run").replace('\n', " ");
    for needle in [
        "`invoke_subagent` returns at once with a `conversationId`",
        "the subagent runs in the background",
        "Do not poll the subagent",
        "End the turn with the gossip bell armed",
        "`agy` delivers the reply of the subagent as a message",
    ] {
        assert!(body.contains(needle), "no `{needle}`");
    }
}

/// What a shell would do in a command text that `agy` cannot match against an
/// allow rule: a substitution, a pipe, a list, a redirect, a background job.
/// With `strict`, quotes protect nothing, for text that a peer reads and
/// then types as a command. Else `$(`, `${`, and a backtick count in double
/// quotes too, and nothing counts in single quotes or after a backslash. A
/// `<word>` or `<some words>` is a placeholder. A `#` that starts a word
/// outside quotes begins a comment.
fn scan_shell(command: &str, strict: bool) -> Vec<String> {
    let chars: Vec<char> = command.chars().collect();
    let mut found = Vec::new();
    let (mut single, mut double, mut ticked) = (false, false, false);
    let mut at = 0;
    while at < chars.len() {
        let here = chars[at];
        let next = chars.get(at + 1).copied();
        let quoted = !strict && (single || double);
        if single && !strict {
            single = here != '\'';
        } else if here == '\\' {
            at += 1;
        } else if !strict && !single && here == '\'' && !double {
            single = true;
        } else if !strict && here == '"' {
            double = !double;
        } else {
            match (here, next) {
                ('$', Some('(' | '{')) => found.push(format!("`${}`", next.unwrap())),
                ('`', _) => {
                    // A pair of backticks is one substitution.
                    ticked = !ticked;
                    if ticked {
                        found.push("a backtick".to_string());
                    }
                }
                _ if quoted => {}
                ('#', _) if at == 0 || chars[at - 1].is_whitespace() => {
                    while at < chars.len() && chars[at] != '\n' {
                        at += 1;
                    }
                }
                ('&', Some('&')) | ('|', Some('|')) => {
                    found.push(format!("`{here}{here}`"));
                    at += 1;
                }
                ('&' | '|' | ';' | '>', _) => found.push(format!("`{here}`")),
                ('<', Some('(')) => found.push("`<(`".to_string()),
                ('<', _) => match placeholder_end(&chars, at) {
                    Some(end) => at = end,
                    None => found.push("`<`".to_string()),
                },
                _ => {}
            }
        }
        at += 1;
    }
    found
}

/// The index of the `>` that closes the placeholder that starts at `open`:
/// a letter, then letters, digits, spaces, `_`, `.`, or `-`.
fn placeholder_end(chars: &[char], open: usize) -> Option<usize> {
    let mut at = open + 1;
    if !chars.get(at)?.is_ascii_alphabetic() {
        return None;
    }
    while let Some(&here) = chars.get(at) {
        match here {
            '>' => return Some(at),
            ' ' | '_' | '.' | '-' => {}
            _ if here.is_ascii_alphanumeric() => {}
            _ => return None,
        }
        at += 1;
    }
    None
}

fn shell_constructs(command: &str) -> Vec<String> {
    scan_shell(command, false)
}

/// The commands of the `bash`, `sh`, `shell`, and untagged blocks of a skill,
/// each with the line where it starts. A block with another tag, such as
/// `text`, holds output or data.
fn commands_of(body: &str) -> Vec<(usize, String)> {
    let mut commands = Vec::new();
    let mut open: Option<(bool, usize, Vec<&str>)> = None;
    for (number, line) in body.lines().enumerate() {
        let trimmed = line.trim_start();
        if let Some(tag) = trimmed.strip_prefix("```") {
            match open.take() {
                Some((true, start, lines)) => commands.push((start, lines.join("\n"))),
                Some((false, ..)) => {}
                None => {
                    let is_command = matches!(tag, "" | "bash" | "sh" | "shell");
                    open = Some((is_command, number + 2, Vec::new()));
                }
            }
        } else if let Some((_, _, lines)) = open.as_mut() {
            lines.push(trimmed);
        }
    }
    commands
}

#[test]
fn the_shell_construct_scanner_knows_quotes_comments_and_escapes() {
    for clean in [
        "agent-gossip a2a call --text \"run <id> or <node>\"",
        "agent-graph node list --tag '<regex>' # only nodes with a tag > 1",
        "agent-gossip a2a call --text \"mkdir && tar, a || b, a > b, a | b; c &\"",
        "echo '$(date) ${HOME} && || `x`'",
        "echo \"\\$(date) \\${HOME} \\`x\\`\"",
        "agent-gossip meta merge --merge '{\"a\":\"'\"$INSTANCE\"'\"}'",
        "agent-graph graph topology \\\n  --me \"$PUBKEY\"",
        "agent-graph graph nodes <graph name> --x [--name <name>] <graph>/<node>",
        "agent-graph graph pack \"$SOURCE\" --output /tmp/$HASH.tar",
    ] {
        assert_eq!(shell_constructs(clean), Vec::<String>::new(), "{clean}");
    }
    for (dirty, construct) in [
        ("a $(date +%s)", "`$(`"),
        ("a \"$(date +%s)\"", "`$(`"),
        ("a ${HOME}", "`${`"),
        ("a && b", "`&&`"),
        ("a || b", "`||`"),
        ("a > b", "`>`"),
        ("a 2>/dev/null", "`>`"),
        ("a `date`", "a backtick"),
        ("a \"`date`\"", "a backtick"),
        ("a | b", "`|`"),
        ("a ; b", "`;`"),
        ("a;b", "`;`"),
        ("diff <(a)", "`<(`"),
        ("a &", "`&`"),
        ("a < in", "`<`"),
        ("a <2", "`<`"),
    ] {
        assert_eq!(shell_constructs(dirty), [construct], "{dirty}");
    }
    assert_eq!(shell_constructs("a < in > out"), ["`<`", "`>`"]);
    assert_eq!(
        scan_shell("say \"a && b\" and '$(x)'", true),
        ["`&&`", "`$(`"]
    );
}

#[test]
fn the_scanner_reads_each_kind_of_fence_that_holds_commands() {
    let body = "```bash\na $(x)\n```\n\n```sh\na | b\n```\n\n```shell\na ; b\n```\n\n```\na && b\n```\n\n```text\n💬 a $(x) | b\n```\n\nafter\n";
    let found: Vec<(usize, Vec<String>)> = commands_of(body)
        .into_iter()
        .map(|(line, command)| (line, shell_constructs(&command)))
        .collect();
    assert_eq!(
        found,
        [
            (2, vec!["`$(`".to_string()]),
            (6, vec!["`|`".to_string()]),
            (10, vec!["`;`".to_string()]),
            (14, vec!["`&&`".to_string()]),
        ]
    );
}

#[test]
fn the_agy_render_has_no_shell_construct_in_a_command_line() {
    let tmp = TempDir::new().unwrap();
    let mut problems = Vec::new();
    for (name, _) in source_skills() {
        for (line, command) in commands_of(&agy_rendered_skill(tmp.path(), &name)) {
            for construct in shell_constructs(&command) {
                problems.push(format!("{name}:{line} {construct}: {command:.70}"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_node_offer_brief_asks_no_peer_for_a_shell_construct() {
    let tmp = TempDir::new().unwrap();
    for skill in ["graph-up", "graph-node-invite"] {
        let body = agy_rendered_skill(tmp.path(), skill);
        let brief = body
            .lines()
            .find(|line| line.contains("--text \"Node offer."))
            .unwrap_or_else(|| panic!("{skill}: no brief"));
        // The peer reads the text of the brief as plain text, so quotes
        // protect nothing in it.
        let text = brief.split_once("--text \"").unwrap().1;
        assert_eq!(scan_shell(text, true), Vec::<String>::new(), "{skill}");
    }
}

#[test]
fn the_readme_sandbox_rule_names_no_command_that_the_skills_do_not_run() {
    let readme =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../README.md")).unwrap();
    assert!(
        !readme.contains("`curl` command"),
        "README still names curl"
    );
}

#[test]
fn the_node_offer_brief_declines_when_agent_graph_is_too_old() {
    let tmp = TempDir::new().unwrap();
    for skill in ["graph-up", "graph-node-invite"] {
        let body = rendered_skill(tmp.path(), skill);
        let brief = body
            .lines()
            .find(|line| line.contains("--text \"Node offer."))
            .unwrap_or_else(|| panic!("{skill}: no brief"));
        let rule = brief
            .find("decline with the reason: agent-graph too old, run agent-graph plug after an upgrade")
            .unwrap_or_else(|| panic!("{skill}: no too-old rule"));
        let first_step = brief.find("A1.").unwrap();

        assert!(rule < first_step, "{skill}: the rule is not before A1");
        let sentence = &brief[..rule];
        assert!(
            sentence.ends_with(
                "graph install is an unknown command, or --pid auto or --dir is not accepted, "
            ),
            "{skill}: {sentence}"
        );
    }
}

/// `run_command` in agy has no working-folder parameter, and each call is a
/// fresh shell, so no skill can say `cd`. A command that needs a start folder
/// passes `--dir`.
#[test]
fn no_skill_depends_on_the_working_folder_of_a_command() {
    let tmp = TempDir::new().unwrap();
    let init = rendered_skill(tmp.path(), "graph-init");
    assert!(!init.contains("cd \"$TARGET\""), "graph-init still says cd");
    assert!(
        !init.contains("working-directory"),
        "graph-init: no such parameter"
    );
    for command in [
        "agent-graph graph list --dir \"$TARGET\"",
        "agent-graph node list --dir \"$TARGET\"",
    ] {
        assert!(init.contains(command), "graph-init: no `{command}`");
    }
    for skill in ["graph-up", "graph-node-invite"] {
        let body = rendered_skill(tmp.path(), skill);
        assert!(
            !body.contains("from your home folder"),
            "{skill}: names the home folder"
        );
        assert!(
            !body.contains("From your home folder"),
            "{skill}: names the home folder"
        );
        assert!(body.contains("--dir ~"), "{skill}: no --dir ~");
        let brief = body
            .lines()
            .find(|line| line.contains("--text \"Node offer."))
            .unwrap_or_else(|| panic!("{skill}: no brief"));
        assert!(
            brief.contains("A2. Run agent-graph graph node-up $HASH/<node> --dir ~ --name"),
            "{skill}"
        );
    }
}

const PLACEHOLDER_RULE: &str = "is a placeholder, not a shell variable";
const ESCAPED_RULE: &str = "stays as it is: it is text for a peer";

/// agy matches an Always Allow rule against the start of a command line, and
/// its documented rule says that `$VAR` and `${VAR}` disable the match. So an
/// agy command line holds literal values: each `$NAME` in a command of the
/// agy render is a placeholder, and the skill must say to write the value in
/// its place. A `'"$NAME"'` quote juggle is not a command line that an agent
/// can fill in, so the agy render writes it as `$NAME`.
#[test]
fn a_placeholder_in_an_agy_command_line_comes_with_the_rule_to_write_its_value() {
    let tmp = TempDir::new().unwrap();
    let mut seen = 0;
    for (name, _) in source_skills() {
        let body = agy_rendered_skill(tmp.path(), &name);
        let mut placeholders = Vec::new();
        for (line, commands, _) in blocks_of(&body) {
            for command in commands {
                assert!(
                    !command.contains("'\"$"),
                    "{name}:{line}: a quote juggle in an agy command: {command:.80}"
                );
                for (at, _) in command.match_indices('$') {
                    let named = command[at + 1..]
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_uppercase() || ch == '_');
                    if named && !command[..at].ends_with('\\') {
                        placeholders.push(format!("{name}:{line}: {command:.60}"));
                    }
                }
            }
        }
        assert!(
            body.contains(ESCAPED_RULE),
            "{name}: no rule that an escaped placeholder is text for a peer"
        );
        if !placeholders.is_empty() {
            assert!(
                body.contains(PLACEHOLDER_RULE),
                "{name}: placeholders in commands but no rule to write their values: {}",
                placeholders[0]
            );
            seen += 1;
        }
    }
    assert!(seen > 0, "no agy command line has a placeholder");
}

/// The scanner above sees only fences, so a command that the prose tells the
/// agent to run in an inline span must be allowed too.
#[test]
fn prose_names_no_command_that_the_skill_does_not_allow() {
    let tmp = TempDir::new().unwrap();
    let mut problems = Vec::new();
    for (name, source) in source_skills() {
        let allowed = allowed_prefixes(&source);
        let mut in_fence = false;
        for (number, line) in rendered_skill(tmp.path(), &name).lines().enumerate() {
            if line.trim_start().starts_with("```") {
                in_fence = !in_fence;
            } else if !in_fence && !line.contains("--text \"") {
                for span in line.split('`').skip(1).step_by(2) {
                    let command = span.split_whitespace().next().unwrap_or_default();
                    let shell = [
                        "hostname", "date", "tar", "ls", "cp", "mkdir", "curl", "mktemp",
                    ];
                    if shell.contains(&command) && !allowed.iter().any(|prefix| prefix == command) {
                        problems.push(format!("{name}:{}: `{span}`", number + 1));
                    }
                }
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_graph_down_broadcast_tells_agy_to_write_pid_auto() {
    let tmp = TempDir::new().unwrap();
    let body = rendered_skill(tmp.path(), "graph-down");
    let broadcast = body
        .lines()
        .find(|line| line.contains("--text \"graph $INSTANCE is down"))
        .expect("the broadcast");
    let ppid = broadcast
        .find("node-down --pid \\\"\\$PPID\\\"")
        .expect("--pid");
    let advice = broadcast
        .find("In agy, write --pid auto in place of --pid \\\"\\$PPID\\\".")
        .expect("no agy advice");
    assert!(ppid < advice);
}

#[test]
fn the_node_offer_brief_declines_when_agent_gossip_is_too_old_and_names_the_conversation_id() {
    let tmp = TempDir::new().unwrap();
    for skill in ["graph-up", "graph-node-invite"] {
        let body = rendered_skill(tmp.path(), skill);
        let brief = body
            .lines()
            .find(|line| line.contains("--text \"Node offer."))
            .unwrap_or_else(|| panic!("{skill}: no brief"));
        let first_step = brief.find("A1.").unwrap();
        for text in [
            "\\$CONVERSATION is not a shell variable: write your conversation id in its place",
            "If agent-gossip rejects --session, decline with the reason: agent-gossip too old, run agent-gossip plug after an upgrade",
        ] {
            let at = brief
                .find(text)
                .unwrap_or_else(|| panic!("{skill}: no `{text}`"));
            assert!(at < first_step, "{skill}: `{text}` is not before A1");
        }
    }
}

#[test]
fn each_skill_that_finds_the_gossip_says_what_to_write_for_conversation_and_stops_when_agent_gossip_is_too_old()
 {
    let tmp = TempDir::new().unwrap();
    let mut seen = 0;
    for (name, source) in source_skills() {
        if !source.contains("shared/gossip-session.md") {
            continue;
        }
        let body = agy_rendered_skill(tmp.path(), &name);
        for text in [
            "`$CONVERSATION` is not a shell variable. Write the conversation id",
            "graph \u{b7} agent-gossip too old \u{b7} run agent-gossip plug after an upgrade",
        ] {
            assert!(body.contains(text), "{name}: no `{text}`");
        }
        seen += 1;
    }
    assert!(seen > 0, "no skill includes gossip-session.md");
}

/// The `bash` and `sh` blocks of a skill: the line of the fence, the command
/// lines (a trailing `\` joins the next line, a blank or `#` line is not one),
/// and the text before the fence.
fn blocks_of(body: &str) -> Vec<(usize, Vec<String>, String)> {
    let lines: Vec<&str> = body.lines().collect();
    let mut blocks = Vec::new();
    let mut at = 0;
    while at < lines.len() {
        let fence = lines[at].trim_start();
        if matches!(fence, "```bash" | "```sh") {
            let before = lines[at.saturating_sub(4)..at].join(" ");
            let mut commands: Vec<String> = Vec::new();
            let mut joined = false;
            at += 1;
            while at < lines.len() && !lines[at].trim_start().starts_with("```") {
                let line = lines[at].trim();
                if !line.is_empty() && !line.starts_with('#') {
                    match (joined, commands.last_mut()) {
                        (true, Some(last)) => last.push_str(line),
                        _ => commands.push(line.to_string()),
                    }
                    joined = line.ends_with('\\');
                }
                at += 1;
            }
            blocks.push((at, commands, before));
        }
        at += 1;
    }
    blocks
}

#[test]
fn a_block_with_more_than_one_command_says_to_run_each_as_its_own_command() {
    let tmp = TempDir::new().unwrap();
    let mut problems = Vec::new();
    for (name, _) in source_skills() {
        for (line, commands, before) in blocks_of(&rendered_skill(tmp.path(), &name)) {
            if commands.len() > 1 && !before.contains("own command") {
                problems.push(format!(
                    "{name}:{line}: {} commands, no `own command`",
                    commands.len()
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The command prefixes in the `allowed-tools` line of a skill: `agent-graph`
/// for `Bash(agent-graph:*)`.
fn allowed_prefixes(source: &str) -> Vec<String> {
    let line = source
        .lines()
        .find(|line| line.starts_with("allowed-tools:"))
        .unwrap_or_default();
    line.split("Bash(")
        .skip(1)
        .filter_map(|part| part.split(":*)").next())
        .map(str::to_string)
        .collect()
}

/// Each command of each block of a skill must be allowed by its
/// `allowed-tools`, and each allowed tool must run: a command that is not
/// allowed asks the user for approval, and a tool that no command uses is a
/// permission for nothing.
#[test]
fn the_allowed_tools_of_a_skill_are_the_commands_that_it_runs() {
    let tmp = TempDir::new().unwrap();
    let mut problems = Vec::new();
    for (name, source) in source_skills() {
        let allowed = allowed_prefixes(&source);
        let mut used = vec![false; allowed.len()];
        for (line, commands, _) in blocks_of(&rendered_skill(tmp.path(), &name)) {
            for command in commands {
                let found = allowed.iter().position(|prefix| {
                    command == *prefix || command.starts_with(&format!("{prefix} "))
                });
                match found {
                    Some(at) => used[at] = true,
                    None => problems.push(format!(
                        "{name}:{line}: `{command:.40}` is not in allowed-tools"
                    )),
                }
            }
        }
        for (prefix, was_used) in allowed.iter().zip(used) {
            if !was_used {
                problems.push(format!(
                    "{name}: allows `{prefix}` and runs no such command"
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn the_agy_render_has_ppid_only_in_escaped_brief_text() {
    let tmp = TempDir::new().unwrap();
    let mut seen = 0;
    for (name, _) in source_skills() {
        for (number, line) in agy_rendered_skill(tmp.path(), &name).lines().enumerate() {
            for (at, _) in line.match_indices("$PPID") {
                let before = &line[..at];
                let escaped = before.ends_with('\\');
                // `--pid "$PPID"` is `--pid auto` and `--session-pid "$PPID"`
                // is `--session $CONVERSATION` in this render. A brief that
                // goes to a peer of any agent can still name `$PPID`, escaped.
                assert!(
                    escaped,
                    "{name}:{}: `$PPID` outside escaped brief text: {line:.100}",
                    number + 1
                );
                seen += 1;
            }
        }
    }
    assert!(seen > 0, "the check found no `$PPID` at all");
}

#[test]
fn the_session_command_is_by_conversation_in_agy_and_by_pid_elsewhere() {
    let tmp = TempDir::new().unwrap();
    let agy = agy_rendered_skill(tmp.path(), "graph-down");
    assert!(agy.contains("agent-gossip session --session $CONVERSATION\n"));
    assert!(!agy.contains("agent-gossip session --session-pid \"$PPID\"\n"));
    assert!(
        agy.contains("agent-gossip session` with no flag"),
        "no id fallback"
    );
    let other = rendered_skill(tmp.path(), "graph-down");
    assert!(other.contains("agent-gossip session --session-pid \"$PPID\"\n"));
    assert!(!other.contains("--session $CONVERSATION"));
}
