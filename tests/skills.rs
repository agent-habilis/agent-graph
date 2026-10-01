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
    let skill = rendered_skill(dir, "node-invite");
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
        .env("GRAPH", "default")
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
        .replace("<your nickname>", "bob");
    let json: Value =
        serde_json::from_str(&merge).unwrap_or_else(|error| panic!("{error}: {merge}"));
    let entry = &json["peers"]["bob"];
    assert_eq!(entry["instance"], "demo");
    assert_eq!(entry["graph"], "default");
    assert_eq!(entry["hash"], "abc123");
    assert_eq!(entry["node"], "worker");
    assert_eq!(entry["invited_by"], "ann");
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
        .replace("<your nickname>", "bob");
    let unverified_json: Value =
        serde_json::from_str(&unverified).unwrap_or_else(|error| panic!("{error}: {unverified}"));
    let unverified_entry = &unverified_json["peers"]["bob"];
    assert_eq!(unverified_entry["verified"], false);
    assert_eq!(unverified_entry["instance"], "demo");
    assert_eq!(unverified_entry["node"], "worker");
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
    let skill = rendered_skill(dir, "node-invite");
    let bin = stub(dir);
    let hash = format!("skills-test-{}", std::process::id());
    let env = [
        ("GOSSIP", "g1"),
        ("NICKNAME", "ann"),
        ("SOURCE", "default"),
        ("HASH", hash.as_str()),
    ];

    let hash_block = bash_block(&skill, "## Node offer", "CONTEXT_HASH=")
        .replace("<node>", "worker")
        + "\necho \"$CONTEXT_HASH\"\n";
    let context_hash = run_block(&hash_block, &project, &bin, &env);
    let send_block = bash_block(&skill, "## Drive", "node up")
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
        .args(["node", "up", "default/worker"])
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
