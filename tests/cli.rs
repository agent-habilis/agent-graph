use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

/// A temp root, canonicalized so paths match what the binary prints on
/// macOS, where `/var` is a symlink to `/private/var`.
fn root() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().canonicalize().unwrap();
    (tmp, path)
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn role_dir(level: &Path, name: &str) -> PathBuf {
    level.join(".agent-roles/roles").join(name)
}

/// Write `roles/<name>/index.md` under `level` and return the role folder.
fn role(level: &Path, name: &str, index: &str) -> PathBuf {
    let dir = role_dir(level, name);
    write(&dir.join("index.md"), index);
    dir
}

fn index(description: &str, tags: Option<&str>, body: &str) -> String {
    let tags = tags
        .map(|list| format!("tags: {list}\n"))
        .unwrap_or_default();
    format!("---\ntype: Role\ndescription: {description}\n{tags}---\n{body}")
}

fn run(cwd: &Path, args: &[&str]) -> Output {
    fs::create_dir_all(cwd).unwrap();
    Command::new(env!("CARGO_BIN_EXE_agent-role"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8(out.stderr.clone()).unwrap()
}

fn line(name: &str, description: &str, tags: &str, dir: &Path) -> String {
    format!("{name}\t{description}\t{tags}\t{}\n", dir.display())
}

#[test]
fn list_shows_roles_from_cwd_and_parents() {
    let (_tmp, root) = root();
    let outer = role(&root, "alpha", &index("outer role", None, "A\n"));
    let inner = role(&root.join("x"), "beta", &index("inner role", None, "B\n"));

    let out = run(&root.join("x/y"), &["list"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        line("alpha", "outer role", "", &outer) + &line("beta", "inner role", "", &inner)
    );
}

#[test]
fn nearest_role_shadows_parent_role() {
    let (_tmp, root) = root();
    role(&root, "advisor", &index("outer", None, "outer body\n"));
    let inner = role(
        &root.join("x"),
        "advisor",
        &index("inner", None, "inner body\n"),
    );
    let cwd = root.join("x/y");

    let list = run(&cwd, &["list"]);
    let get = run(&cwd, &["get", "advisor"]);

    assert_eq!(stdout(&list), line("advisor", "inner", "", &inner));
    assert_eq!(stdout(&get), "inner body\n");
}

#[test]
fn list_warns_and_skips_invalid_role() {
    let (_tmp, root) = root();
    let ok = role(&root, "ok", &index("fine", None, ""));
    fs::create_dir_all(role_dir(&root, "no-index")).unwrap();
    role(
        &root,
        "wrong-type",
        "---\ntype: Note\ndescription: x\n---\n",
    );
    role(&root, "no-description", "---\ntype: Role\n---\n");
    role(
        &root,
        "block-tags",
        "---\ntype: Role\ndescription: x\ntags:\n  - a\n---\n",
    );

    let out = run(&root, &["list"]);

    assert!(out.status.success());
    assert_eq!(stdout(&out), line("ok", "fine", "", &ok));
    let err = stderr(&out);
    for name in ["no-index", "wrong-type", "no-description", "block-tags"] {
        assert!(
            err.lines()
                .any(|warning| warning.starts_with("warning: ") && warning.contains(name)),
            "no warning for {name} in:\n{err}"
        );
    }
}

#[test]
fn list_prints_tags_column() {
    let (_tmp, root) = root();
    let dir = role(&root, "advisor", &index("d", Some("[review, go]"), ""));

    let out = run(&root, &["list"]);

    assert_eq!(stdout(&out), line("advisor", "d", "review,go", &dir));
}

#[test]
fn list_filters_by_tag_regex() {
    let (_tmp, root) = root();
    let go = role(&root, "go-dev", &index("d", Some("[golang]"), ""));
    let mongo = role(&root, "mongo", &index("d", Some("[db, mongo]"), ""));
    role(&root, "rust", &index("d", Some("[rust]"), ""));
    role(&root, "untagged", &index("d", None, ""));

    let anchored = run(&root, &["list", "--tag", "^go"]);
    let unanchored = run(&root, &["list", "--tag", "go"]);

    assert_eq!(stdout(&anchored), line("go-dev", "d", "golang", &go));
    assert_eq!(
        stdout(&unanchored),
        line("go-dev", "d", "golang", &go) + &line("mongo", "d", "db,mongo", &mongo)
    );
}

#[test]
fn list_tag_filter_runs_after_shadowing() {
    let (_tmp, root) = root();
    role(&root, "advisor", &index("outer", Some("[x]"), ""));
    let inner = role(&root.join("a"), "advisor", &index("inner", Some("[y]"), ""));

    let parent_tag = run(&root.join("a"), &["list", "--tag", "x"]);
    let inner_tag = run(&root.join("a"), &["list", "--tag", "y"]);

    assert!(parent_tag.status.success());
    assert_eq!(stdout(&parent_tag), "");
    assert_eq!(stdout(&inner_tag), line("advisor", "inner", "y", &inner));
}

#[test]
fn list_invalid_tag_regex_exits_1() {
    let (_tmp, root) = root();

    let out = run(&root, &["list", "--tag", "("]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
}

#[test]
fn get_prints_body_without_frontmatter() {
    let (_tmp, root) = root();
    role(
        &root,
        "advisor",
        &index("d", None, "# Advisor\n\nBe blunt.\n"),
    );

    let out = run(&root, &["get", "advisor"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "# Advisor\n\nBe blunt.\n");
}

#[test]
fn get_expands_nested_includes() {
    let (_tmp, root) = root();
    let dir = role(&root, "advisor", &index("d", None, "A\n  @parts/b.md\nC\n"));
    write(&dir.join("parts/b.md"), "---\ntype: Note\n---\nB1\n@c.md\n");
    write(&dir.join("parts/c.md"), "C1\n");
    write(&dir.join("unused.md"), "never printed\n");

    let out = run(&root, &["get", "advisor"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "A\nB1\nC1\nC\n");
}

#[test]
fn get_rejects_include_cycle() {
    let (_tmp, root) = root();
    let dir = role(&root, "advisor", &index("d", None, "@a.md\n"));
    write(&dir.join("a.md"), "@b.md\n");
    write(&dir.join("b.md"), "@a.md\n");

    let out = run(&root, &["get", "advisor"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
    assert!(stderr(&out).contains("cycle"), "{}", stderr(&out));
}

#[test]
fn get_rejects_include_outside_role() {
    let (_tmp, root) = root();
    role(&root, "other", &index("d", None, "secret\n"));
    write(&root.join("outside.md"), "secret\n");
    role(&root, "relative", &index("d", None, "@../other/index.md\n"));
    let absolute = format!("@{}\n", root.join("outside.md").display());
    role(&root, "absolute", &index("d", None, &absolute));

    for name in ["relative", "absolute"] {
        let out = run(&root, &["get", name]);

        assert_eq!(out.status.code(), Some(1), "{name}");
        assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
        assert!(!stdout(&out).contains("secret"), "{name}");
    }
}

#[test]
fn get_ignores_include_in_code_fence() {
    let (_tmp, root) = root();
    let body = "```md\n@missing.md\n```\n";
    role(&root, "advisor", &index("d", None, body));

    let out = run(&root, &["get", "advisor"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), body);
}

#[test]
fn get_unknown_role_exits_1() {
    let (_tmp, root) = root();

    let out = run(&root, &["get", "nope"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
}

#[test]
fn get_prints_role_with_valid_boundaries() {
    let (_tmp, root) = root();
    let body = "# Worker\n\n## Boundaries\n\n- No `git merge`.\n- No file changes\n  outside the branch.\n\n## Tone\n\nBe brief.\n";
    role(&root, "worker", &index("d", None, body));

    let out = run(&root, &["get", "worker"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), body);
}

#[test]
fn get_rejects_role_with_bad_boundaries() {
    let (_tmp, root) = root();
    role(
        &root,
        "empty",
        &index("d", None, "## Boundaries\n\n## Next\n"),
    );
    role(
        &root,
        "prose",
        &index("d", None, "## Boundaries\n\n- No merge.\nAlso be nice.\n"),
    );

    for name in ["empty", "prose"] {
        let out = run(&root, &["get", name]);

        assert_eq!(out.status.code(), Some(1), "{name}");
        assert!(stderr(&out).contains("Boundaries"), "{}", stderr(&out));
    }
}

#[test]
fn list_warns_on_role_with_bad_boundaries() {
    let (_tmp, root) = root();
    let ok = role(
        &root,
        "ok",
        &index("fine", None, "## Boundaries\n- No merge.\n"),
    );
    role(&root, "empty", &index("d", None, "## Boundaries\n"));

    let out = run(&root, &["list"]);

    assert!(out.status.success());
    assert_eq!(stdout(&out), line("ok", "fine", "", &ok));
    assert!(
        stderr(&out).contains("empty") && stderr(&out).contains("Boundaries"),
        "{}",
        stderr(&out)
    );
}

/// Every file under `defaults/`, as (path relative to `defaults/`, bytes).
fn defaults() -> Vec<(PathBuf, Vec<u8>)> {
    let out = tree(&Path::new(env!("CARGO_MANIFEST_DIR")).join("defaults"));
    assert!(!out.is_empty(), "no files in defaults/");
    out
}

/// Every file under `dir`, as (path relative to `dir`, bytes), sorted.
fn tree(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let relative = path.strip_prefix(root).unwrap().to_path_buf();
                out.push((relative, fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

#[test]
fn init_writes_defaults_into_cwd() {
    let (_tmp, root) = root();

    let out = run(&root, &["init"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(tree(&root.join(".agent-roles")), defaults());
    let names: Vec<String> = stdout(&run(&root, &["list"]))
        .lines()
        .map(|line| line.split('\t').next().unwrap().to_string())
        .collect();
    assert_eq!(names, ["advisor", "qa", "worker"]);
}

#[test]
fn init_prints_each_written_path() {
    let (_tmp, root) = root();

    let out = run(&root, &["init"]);

    let mut expected: Vec<String> = defaults()
        .into_iter()
        .map(|(path, _)| root.join(".agent-roles").join(path).display().to_string())
        .collect();
    expected.sort();
    let mut printed: Vec<String> = stdout(&out).lines().map(str::to_string).collect();
    printed.sort();
    assert_eq!(printed, expected);
}

#[test]
fn init_writes_into_dir_argument() {
    let (_tmp, root) = root();
    let target = root.join("project");
    fs::create_dir_all(&target).unwrap();

    let out = run(&root, &["init", target.to_str().unwrap()]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(target.join(".agent-roles/roles/worker/index.md").is_file());
    assert!(!root.join(".agent-roles").exists());
}

#[test]
fn init_does_nothing_when_folder_exists() {
    let (_tmp, root) = root();
    write(&root.join(".agent-roles/roles/mine/index.md"), "keep me\n");
    let before = tree(&root.join(".agent-roles"));

    let out = run(&root, &["init"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).contains("already exists"), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
    assert_eq!(tree(&root.join(".agent-roles")), before);
}

#[test]
fn init_does_nothing_when_file_exists() {
    let (_tmp, root) = root();
    write(&root.join(".agent-roles"), "a file\n");

    let out = run(&root, &["init"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).contains("already exists"), "{}", stderr(&out));
    assert_eq!(
        fs::read_to_string(root.join(".agent-roles")).unwrap(),
        "a file\n"
    );
}
