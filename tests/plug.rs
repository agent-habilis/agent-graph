use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

fn root() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().canonicalize().unwrap();
    (tmp, path)
}

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_agent-graph"))
        .args(args)
        .env("HOME", home)
        .current_dir(home)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

/// The skill names that the repo's `skills/` sources hold.
fn source_skills() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("skills"))
        .unwrap()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            ["template-", "role-", "team-"]
                .iter()
                .any(|prefix| name.starts_with(prefix))
        })
        .collect();
    names.sort();
    names
}

#[test]
fn plug_path_writes_each_skill_as_one_rendered_file() {
    let (_tmp, home) = root();
    let dest = home.join("skills");
    let out = run(&home, &["plug", "--path", dest.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    let skills = source_skills();
    assert!(!skills.is_empty());
    for skill in &skills {
        let dir = dest.join(skill);
        let files: Vec<_> = fs::read_dir(&dir).unwrap().flatten().collect();
        assert_eq!(files.len(), 1, "{skill}: {files:?}");
        let body = fs::read_to_string(dir.join("SKILL.md")).unwrap();
        assert!(
            !body.contains("<!-- include") && !body.contains("<!-- slot"),
            "{skill}: unrendered directive"
        );
    }
    assert!(!dest.join("shared").exists());
}

#[test]
fn unplug_path_removes_only_the_owned_skills() {
    let (_tmp, home) = root();
    let dest = home.join("skills");
    fs::create_dir_all(dest.join("mine")).unwrap();
    fs::write(dest.join("mine/SKILL.md"), "mine").unwrap();
    fs::write(dest.join("keep.txt"), "keep").unwrap();
    assert!(
        run(&home, &["plug", "--path", dest.to_str().unwrap()])
            .status
            .success()
    );

    let out = run(&home, &["unplug", "--path", dest.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    for skill in source_skills() {
        assert!(!dest.join(&skill).exists(), "{skill} still there");
    }
    assert!(dest.join("mine/SKILL.md").is_file());
    assert!(dest.join("keep.txt").is_file());
}

#[cfg(unix)]
#[test]
fn plug_replaces_a_link_and_does_not_write_into_its_target() {
    let (_tmp, home) = root();
    let target = home.join("repo/team-up");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("SKILL.md"), "source").unwrap();
    let dest = home.join("skills");
    fs::create_dir_all(&dest).unwrap();
    std::os::unix::fs::symlink(&target, dest.join("team-up")).unwrap();

    let out = run(&home, &["plug", "--path", dest.to_str().unwrap()]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!dest.join("team-up").is_symlink());
    assert!(dest.join("team-up/SKILL.md").is_file());
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "source"
    );
}

#[test]
fn plug_installs_only_into_detected_agents() {
    let (_tmp, home) = root();
    fs::create_dir_all(home.join(".claude")).unwrap();
    let out = run(&home, &["plug"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(home.join(".claude/skills/team-up/SKILL.md").is_file());
    assert!(!home.join(".codex").exists());
    assert!(!home.join(".pi").exists());
}

#[test]
fn plug_agent_skips_an_agent_that_is_not_on_the_machine() {
    let (_tmp, home) = root();
    let out = run(&home, &["plug", "--agent", "codex"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!home.join(".codex").exists());
}

#[test]
fn unplug_removes_the_skills_from_an_agent() {
    let (_tmp, home) = root();
    fs::create_dir_all(home.join(".claude/skills/other")).unwrap();
    assert!(run(&home, &["plug"]).status.success());
    let out = run(&home, &["unplug"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!home.join(".claude/skills/team-up").exists());
    assert!(home.join(".claude/skills/other").is_dir());
}

#[test]
fn plug_renders_the_shared_team_sections_into_each_skill_that_includes_them() {
    let (_tmp, home) = root();
    let dest = home.join("skills");
    assert!(
        run(&home, &["plug", "--path", dest.to_str().unwrap()])
            .status
            .success()
    );
    for skill in ["team-up", "role-invite"] {
        let body = fs::read_to_string(dest.join(skill).join("SKILL.md")).unwrap();
        for section in ["## Gossip", "## Team meta", "## Role offer", "## Drive"] {
            assert!(body.contains(section), "{skill}: no {section}");
        }
    }
    let topology = fs::read_to_string(dest.join("team-topology/SKILL.md")).unwrap();
    assert!(topology.contains("## Gossip"));
    assert!(!topology.contains("--state"));
}

#[test]
fn plug_and_unplug_print_one_line_per_target() {
    let (_tmp, home) = root();
    let dest = home.join("skills");
    let dest_text = dest.to_str().unwrap();

    let plugged = run(&home, &["plug", "--agent", "codex", "--path", dest_text]);
    let unplugged = run(&home, &["unplug", "--path", dest_text]);
    let again = run(&home, &["unplug", "--path", dest_text]);

    assert_eq!(
        stdout(&plugged),
        format!("skipped\tcodex\tnot detected\ninstalled\tpath\t{dest_text}\n")
    );
    assert_eq!(stdout(&unplugged), format!("removed\tpath\t{dest_text}\n"));
    assert_eq!(stdout(&again), format!("skipped\tpath\t{dest_text}\n"));
}

#[cfg(unix)]
#[test]
fn unplug_removes_a_link_and_not_its_target() {
    let (_tmp, home) = root();
    let target = home.join("repo/team-up");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("SKILL.md"), "source").unwrap();
    let dest = home.join("skills");
    fs::create_dir_all(&dest).unwrap();
    std::os::unix::fs::symlink(&target, dest.join("team-up")).unwrap();

    let out = run(&home, &["unplug", "--path", dest.to_str().unwrap()]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(dest.join("team-up").symlink_metadata().is_err());
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "source"
    );
}

#[test]
fn plug_installs_pi_skills_under_agent_skills() {
    let (_tmp, home) = root();
    fs::create_dir_all(home.join(".pi")).unwrap();

    let out = run(&home, &["plug", "--agent", "pi"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(home.join(".pi/agent/skills/team-up/SKILL.md").is_file());
}

#[test]
fn plug_replaces_an_owned_folder_with_extra_files() {
    let (_tmp, home) = root();
    let dest = home.join("skills");
    fs::create_dir_all(dest.join("team-up")).unwrap();
    fs::write(dest.join("team-up/old.md"), "old").unwrap();

    let out = run(&home, &["plug", "--path", dest.to_str().unwrap()]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!dest.join("team-up/old.md").exists());
    assert!(dest.join("team-up/SKILL.md").is_file());
}

#[test]
fn plug_removes_a_graph_skill_that_an_older_plug_installed() {
    let (_tmp, home) = root();
    let dest = home.join("skills");
    let dest_text = dest.to_str().unwrap();
    assert!(run(&home, &["plug", "--path", dest_text]).status.success());
    let manifest = dest.join(".agent-graph-plug");
    let mut names = fs::read_to_string(&manifest).unwrap();
    names.push_str("graph-old\n");
    fs::write(&manifest, names).unwrap();
    fs::create_dir_all(dest.join("graph-old")).unwrap();
    fs::write(dest.join("graph-old/SKILL.md"), "old").unwrap();
    fs::create_dir_all(dest.join("graph-mine")).unwrap();

    let out = run(&home, &["plug", "--path", dest_text]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!dest.join("graph-old").exists());
    assert!(dest.join("graph-mine").is_dir());
    assert!(!fs::read_to_string(&manifest).unwrap().contains("graph-old"));
}

#[test]
fn unplug_removes_the_manifest() {
    let (_tmp, home) = root();
    let dest = home.join("skills");
    let dest_text = dest.to_str().unwrap();
    assert!(run(&home, &["plug", "--path", dest_text]).status.success());

    assert!(
        run(&home, &["unplug", "--path", dest_text])
            .status
            .success()
    );

    assert!(!dest.join(".agent-graph-plug").exists());
}
