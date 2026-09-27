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
fn role_offer_brief_runs_in_bash_and_gives_the_invitee_valid_meta_json() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    let skill = rendered_skill(dir, "role-invite");
    let command = bash_block(&skill, "## Role offer", "a2a call")
        .replace("<id>", "worker")
        .replace("<role>", "worker");
    let bin = stub(dir);

    let out = Command::new("bash")
        .args(["-c", &command])
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("GOSSIP", "g1")
        .env("INVITER", "ann")
        .env("PEER", "bob")
        .env("INSTANCE", "demo")
        .env("TEMPLATE", "default")
        .env("HASH", "abc123")
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
    assert_eq!(entry["team"], "demo");
    assert_eq!(entry["template"], "default");
    assert_eq!(entry["hash"], "abc123");
    assert_eq!(entry["role"], "worker");
    assert_eq!(entry["invited_by"], "ann");
    assert!(text.contains("--to ann "), "{text}");
    assert!(
        text.contains("\"$NICKNAME\""),
        "the invitee's own nickname stays a variable: {text}"
    );
}
