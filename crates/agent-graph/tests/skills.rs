//! Run commands from the rendered skills in `bash`, with a stub
//! `agent-gossip` that records its arguments.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

/// `bash`.
#[cfg(not(windows))]
fn bash() -> Command {
    Command::new("bash")
}

/// The bash of Git for Windows. `Command::new("bash")` would find the WSL
/// `bash.exe` in System32 before the folders of the parent's `PATH`.
#[cfg(windows)]
fn bash() -> Command {
    Command::new(git_usr_bin().join("bash.exe"))
}

/// The `usr\bin` folder of Git for Windows: three folders above
/// `git --exec-path`, which is `<git>/mingw64/libexec/git-core`.
#[cfg(windows)]
fn git_usr_bin() -> PathBuf {
    let out = Command::new("git")
        .arg("--exec-path")
        .output()
        .expect("git on PATH");
    let exec = PathBuf::from(String::from_utf8(out.stdout).unwrap().trim());
    exec.ancestors()
        .nth(3)
        .expect("git --exec-path is <git>/mingw64/libexec/git-core")
        .join("usr")
        .join("bin")
}

/// The `PATH` of a bash block: `dirs`, then the folders of the system.
#[cfg(not(windows))]
fn search_path(dirs: &[&Path]) -> String {
    let mut path = String::new();
    for dir in dirs {
        path.push_str(&dir.display().to_string());
        path.push(':');
    }
    path + "/usr/bin:/bin"
}

/// The `PATH` of a bash block: `dirs`, then the `core_perl` of Git, where
/// `shasum` is, and its `usr\bin`, joined with `;`. Bash turns it into its own
/// form when it starts.
#[cfg(windows)]
fn search_path(dirs: &[&Path]) -> std::ffi::OsString {
    let usr_bin = git_usr_bin();
    let dirs = dirs.iter().map(|dir| dir.to_path_buf());
    std::env::join_paths(dirs.chain([usr_bin.join("core_perl"), usr_bin])).unwrap()
}

/// `path` as a shell script writes it. On Windows `\` is an escape in `sh`, so
/// the path takes `/`.
fn shell_path(path: &Path) -> String {
    let text = path.display().to_string();
    #[cfg(windows)]
    let text = text.replace('\\', "/");
    text
}

/// The folder that bash calls `/tmp`.
#[cfg(not(windows))]
fn bash_tmp() -> PathBuf {
    PathBuf::from("/tmp")
}

/// The folder that bash calls `/tmp`. On Windows Git Bash puts it in the
/// temp folder of the user, so `cygpath` names it.
#[cfg(windows)]
fn bash_tmp() -> PathBuf {
    let out = bash()
        .args(["-c", "cygpath -w /tmp"])
        .env("PATH", search_path(&[]))
        .output()
        .unwrap();
    PathBuf::from(String::from_utf8(out.stdout).unwrap().trim())
}

/// The output of `shasum -a 256 <path>`.
#[cfg(not(windows))]
fn shasum(path: &Path) -> Vec<u8> {
    Command::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .unwrap()
        .stdout
}

/// The output of `shasum -a 256 <path>`. On Windows `shasum` is a Perl
/// script of Git Bash, so bash runs it.
#[cfg(windows)]
fn shasum(path: &Path) -> Vec<u8> {
    bash()
        .args(["-c", "shasum -a 256 \"$1\"", "shasum"])
        .arg(shell_path(path))
        .env("PATH", search_path(&[]))
        .output()
        .unwrap()
        .stdout
}

fn rendered_skill(dir: &Path, skill: &str) -> String {
    let dest = dir.join("skills");
    let out = Command::new(env!("CARGO_BIN_EXE_agent-graph"))
        .args(["plug", "--path", dest.to_str().unwrap()])
        .env("HOME", dir)
        // Windows reads the home from `USERPROFILE`.
        .env("USERPROFILE", dir)
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
            shell_path(&dir.join("args"))
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

    let out = bash()
        .args(["-c", &command])
        .env("PATH", search_path(&[&bin]))
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
        .replace("<your nickname>", "bob")
        .replace("<epoch seconds>", "1700000000");
    let json: Value =
        serde_json::from_str(&merge).unwrap_or_else(|error| panic!("{error}: {merge}"));
    let entry = &json["peers"]["bob"];
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
        .replace("<your nickname>", "bob")
        .replace("<epoch seconds>", "1700000000");
    let unverified_json: Value =
        serde_json::from_str(&unverified).unwrap_or_else(|error| panic!("{error}: {unverified}"));
    let unverified_entry = &unverified_json["peers"]["bob"];
    assert_eq!(unverified_entry["verified"], false);
    assert_eq!(unverified_entry["instance"], "demo");
    assert_eq!(unverified_entry["node"], "worker");
    assert_eq!(unverified_entry["joined_at"], 1_700_000_000);
}

/// Run a bash block in `cwd` with the real `agent-graph` and a stub
/// `agent-gossip` on `PATH`, and return its stdout.
fn run_block(block: &str, cwd: &Path, bin: &Path, env: &[(&str, &str)]) -> String {
    let agent_graph = Path::new(env!("CARGO_BIN_EXE_agent-graph"));
    let out = bash()
        .args(["-c", block])
        .current_dir(cwd)
        .env("PATH", search_path(&[bin, agent_graph.parent().unwrap()]))
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

    // The block writes `/tmp/<hash>-worker.md` in bash, and passes that path
    // to agent-gossip.
    let shell_sent = format!("/tmp/{hash}-worker.md");
    let sent = bash_tmp().join(format!("{hash}-worker.md"));
    let body = fs::read(&sent).unwrap();
    let digest = shasum(&sent);
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
    assert!(args.contains(&shell_sent), "{args}");
}

/// On Windows the rendered skills pass `auto` for the agent pid and plain
/// `hostname`. `$PPID` is left only in the brief and the broadcast, which
/// run on a peer that may not be Windows, and there beside `--pid auto`.
#[cfg(windows)]
#[test]
fn windows_skills_pass_auto_and_plain_hostname() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();
    rendered_skill(dir, "graph-up");
    for entry in fs::read_dir(dir.join("skills")).unwrap() {
        let path = entry.unwrap().path().join("SKILL.md");
        if !path.is_file() {
            // `.agent-graph-plug`, the marker of the install.
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let name = path.display();
        assert!(!text.contains("--pid \"$PPID\""), "{name}");
        assert!(!text.contains("--session-pid \"$PPID\""), "{name}");
        assert!(!text.contains("hostname -s"), "{name}");
        for line in text.lines().filter(|line| line.contains("PPID")) {
            assert!(
                line.contains("--pid auto on Windows, else with --pid"),
                "{name}: {line}"
            );
        }
    }
}
