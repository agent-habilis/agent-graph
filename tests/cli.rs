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

const TEMPLATE_INDEX: &str = "---\ntype: TeamTemplate\ndescription: g\nscope: branch\n---\n";

fn role_dir(level: &Path, template: &str, name: &str) -> PathBuf {
    level
        .join(".agent-graph")
        .join(template)
        .join("roles")
        .join(name)
}

/// Write `<template>/index.md` and `<template>/roles/<name>/index.md` under
/// `level`, and return the role folder.
fn template_role(level: &Path, template: &str, name: &str, index: &str) -> PathBuf {
    write(
        &level.join(".agent-graph").join(template).join("index.md"),
        TEMPLATE_INDEX,
    );
    let dir = role_dir(level, template, name);
    write(&dir.join("index.md"), index);
    dir
}

/// Write the role `<name>` in the template `g` under `level`.
fn role(level: &Path, name: &str, index: &str) -> PathBuf {
    template_role(level, "g", name, index)
}

fn index(description: &str, tags: Option<&str>, body: &str) -> String {
    let tags = tags
        .map(|list| format!("tags: {list}\n"))
        .unwrap_or_default();
    format!("---\ntype: Role\ndescription: {description}\n{tags}---\n{body}")
}

fn run(cwd: &Path, args: &[&str]) -> Output {
    fs::create_dir_all(cwd).unwrap();
    Command::new(env!("CARGO_BIN_EXE_agent-graph"))
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
fn role_list_shows_roles_from_cwd_and_parents() {
    let (_tmp, root) = root();
    let outer = template_role(&root, "outer", "alpha", &index("outer role", None, "A\n"));
    let inner = template_role(
        &root.join("x"),
        "inner",
        "beta",
        &index("inner role", None, "B\n"),
    );

    let out = run(&root.join("x/y"), &["role", "list"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        line("inner/beta", "inner role", "", &inner)
            + &line("outer/alpha", "outer role", "", &outer)
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

    let list = run(&cwd, &["role", "list"]);
    let get = run(&cwd, &["role", "up", "g/advisor"]);

    assert_eq!(stdout(&list), line("g/advisor", "inner", "", &inner));
    assert_eq!(stdout(&get), "inner body\n");
}

#[test]
fn role_list_warns_and_skips_invalid_role() {
    let (_tmp, root) = root();
    let ok = role(&root, "ok", &index("fine", None, ""));
    fs::create_dir_all(role_dir(&root, "g", "no-index")).unwrap();
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

    let out = run(&root, &["role", "list"]);

    assert!(out.status.success());
    assert_eq!(stdout(&out), line("g/ok", "fine", "", &ok));
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
fn role_list_prints_tags_column() {
    let (_tmp, root) = root();
    let dir = role(&root, "advisor", &index("d", Some("[review, go]"), ""));

    let out = run(&root, &["role", "list"]);

    assert_eq!(stdout(&out), line("g/advisor", "d", "review,go", &dir));
}

#[test]
fn role_list_filters_by_tag_regex() {
    let (_tmp, root) = root();
    let go = role(&root, "go-dev", &index("d", Some("[golang]"), ""));
    let mongo = role(&root, "mongo", &index("d", Some("[db, mongo]"), ""));
    role(&root, "rust", &index("d", Some("[rust]"), ""));
    role(&root, "untagged", &index("d", None, ""));

    let anchored = run(&root, &["role", "list", "--tag", "^go"]);
    let unanchored = run(&root, &["role", "list", "--tag", "go"]);

    assert_eq!(stdout(&anchored), line("g/go-dev", "d", "golang", &go));
    assert_eq!(
        stdout(&unanchored),
        line("g/go-dev", "d", "golang", &go) + &line("g/mongo", "d", "db,mongo", &mongo)
    );
}

#[test]
fn role_list_tag_filter_runs_after_shadowing() {
    let (_tmp, root) = root();
    role(&root, "advisor", &index("outer", Some("[x]"), ""));
    let inner = role(&root.join("a"), "advisor", &index("inner", Some("[y]"), ""));

    let parent_tag = run(&root.join("a"), &["role", "list", "--tag", "x"]);
    let inner_tag = run(&root.join("a"), &["role", "list", "--tag", "y"]);

    assert!(parent_tag.status.success());
    assert_eq!(stdout(&parent_tag), "");
    assert_eq!(stdout(&inner_tag), line("g/advisor", "inner", "y", &inner));
}

#[test]
fn role_list_invalid_tag_regex_exits_1() {
    let (_tmp, root) = root();

    let out = run(&root, &["role", "list", "--tag", "("]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
}

#[test]
fn role_up_prints_body_without_frontmatter() {
    let (_tmp, root) = root();
    role(
        &root,
        "advisor",
        &index("d", None, "# Advisor\n\nBe blunt.\n"),
    );

    let out = run(&root, &["role", "up", "g/advisor"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "# Advisor\n\nBe blunt.\n");
}

#[test]
fn role_up_expands_nested_includes() {
    let (_tmp, root) = root();
    let dir = role(&root, "advisor", &index("d", None, "A\n  @parts/b.md\nC\n"));
    write(&dir.join("parts/b.md"), "---\ntype: Note\n---\nB1\n@c.md\n");
    write(&dir.join("parts/c.md"), "C1\n");
    write(&dir.join("unused.md"), "never printed\n");

    let out = run(&root, &["role", "up", "g/advisor"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "A\nB1\nC1\nC\n");
}

#[test]
fn role_up_rejects_include_cycle() {
    let (_tmp, root) = root();
    let dir = role(&root, "advisor", &index("d", None, "@a.md\n"));
    write(&dir.join("a.md"), "@b.md\n");
    write(&dir.join("b.md"), "@a.md\n");

    let out = run(&root, &["role", "up", "g/advisor"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
    assert!(stderr(&out).contains("cycle"), "{}", stderr(&out));
}

#[test]
fn role_up_rejects_include_outside_template() {
    let (_tmp, root) = root();
    write(&root.join("outside.md"), "secret\n");
    role(
        &root,
        "relative",
        &index("d", None, "@../../../../outside.md\n"),
    );
    let absolute = format!("@{}\n", root.join("outside.md").display());
    role(&root, "absolute", &index("d", None, &absolute));

    for name in ["relative", "absolute"] {
        let out = run(&root, &["role", "up", &format!("g/{name}")]);

        assert_eq!(out.status.code(), Some(1), "{name}");
        assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
        assert!(!stdout(&out).contains("secret"), "{name}");
    }
}

#[test]
fn role_up_ignores_include_in_code_fence() {
    let (_tmp, root) = root();
    let body = "```md\n@missing.md\n```\n";
    role(&root, "advisor", &index("d", None, body));

    let out = run(&root, &["role", "up", "g/advisor"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), body);
}

#[test]
fn role_up_unknown_role_exits_1() {
    let (_tmp, root) = root();

    let out = run(&root, &["role", "up", "g/nope"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
}

#[test]
fn role_up_prints_role_with_valid_boundaries() {
    let (_tmp, root) = root();
    let body = "# Worker\n\n## Boundaries\n\n- No `git merge`.\n- No file changes\n  outside the branch.\n\n## Tone\n\nBe brief.\n";
    role(&root, "worker", &index("d", None, body));

    let out = run(&root, &["role", "up", "g/worker"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), body);
}

#[test]
fn role_up_rejects_role_with_bad_boundaries() {
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
        let out = run(&root, &["role", "up", &format!("g/{name}")]);

        assert_eq!(out.status.code(), Some(1), "{name}");
        assert!(stderr(&out).contains("Boundaries"), "{}", stderr(&out));
    }
}

#[test]
fn role_list_warns_on_role_with_bad_boundaries() {
    let (_tmp, root) = root();
    let ok = role(
        &root,
        "ok",
        &index("fine", None, "## Boundaries\n- No merge.\n"),
    );
    role(&root, "empty", &index("d", None, "## Boundaries\n"));

    let out = run(&root, &["role", "list"]);

    assert!(out.status.success());
    assert_eq!(stdout(&out), line("g/ok", "fine", "", &ok));
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
fn template_init_writes_defaults_into_cwd() {
    let (_tmp, root) = root();

    let out = run(&root, &["template", "init"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(tree(&root.join(".agent-graph")), defaults());
    let names: Vec<String> = stdout(&run(&root, &["role", "list"]))
        .lines()
        .map(|line| line.split('\t').next().unwrap().to_string())
        .collect();
    assert_eq!(
        names,
        ["default/advisor", "default/orchestrator", "default/worker"]
    );
}

#[test]
fn template_init_defaults_have_opus_orchestrator_sonnet_workers_and_fable_advisor() {
    let (_tmp, root) = root();
    run(&root, &["template", "init"]);

    let out = run(&root, &["template", "roles", "default"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "orchestrator\tdefault/orchestrator\tlead\t1\topus\n\
         worker\tdefault/worker\t-\t2\tsonnet\n\
         advisor\tdefault/advisor\t-\t1\tfable\n"
    );
}

#[test]
fn template_init_prints_each_written_path() {
    let (_tmp, root) = root();

    let out = run(&root, &["template", "init"]);

    let mut expected: Vec<String> = defaults()
        .into_iter()
        .map(|(path, _)| root.join(".agent-graph").join(path).display().to_string())
        .collect();
    expected.sort();
    let mut printed: Vec<String> = stdout(&out).lines().map(str::to_string).collect();
    printed.sort();
    assert_eq!(printed, expected);
}

#[test]
fn template_init_writes_into_dir_argument() {
    let (_tmp, root) = root();
    let target = root.join("project");
    fs::create_dir_all(&target).unwrap();

    let out = run(&root, &["template", "init", target.to_str().unwrap()]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        target
            .join(".agent-graph/default/roles/worker/index.md")
            .is_file()
    );
    assert!(!root.join(".agent-graph").exists());
}

#[test]
fn template_init_does_nothing_when_folder_exists() {
    let (_tmp, root) = root();
    write(&root.join(".agent-graph/mine/index.md"), "keep me\n");
    let before = tree(&root.join(".agent-graph"));

    let out = run(&root, &["template", "init"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).contains("already exists"), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
    assert_eq!(tree(&root.join(".agent-graph")), before);
}

#[test]
fn template_init_does_nothing_when_file_exists() {
    let (_tmp, root) = root();
    write(&root.join(".agent-graph"), "a file\n");

    let out = run(&root, &["template", "init"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).contains("already exists"), "{}", stderr(&out));
    assert_eq!(
        fs::read_to_string(root.join(".agent-graph")).unwrap(),
        "a file\n"
    );
}

fn template_index(icon: &str, body: &str) -> String {
    format!("---\ntype: TeamTemplate\ndescription: d\nicon: {icon}\nscope: branch\n---\n{body}")
}

fn role_index_with_icon(icon: &str, body: &str) -> String {
    format!("---\ntype: Role\ndescription: d\nicon: {icon}\n---\n{body}")
}

/// Run `role up <name> --pid <pid>` and return the state file it wrote. Each
/// test uses its own pid, because the state folder is shared.
fn up_state(cwd: &Path, name: &str, pid: u32) -> String {
    let path = PathBuf::from(format!("/tmp/agent-graph/{pid}.json"));
    let _ = fs::remove_file(&path);

    let out = run(cwd, &["role", "up", name, "--pid", &pid.to_string()]);

    assert!(out.status.success(), "{}", stderr(&out));
    let state = fs::read_to_string(&path).unwrap();
    fs::remove_file(&path).unwrap();
    state
}

#[test]
fn role_list_names_roles_template_slash_role() {
    let (_tmp, root) = root();
    let worker = template_role(&root, "team", "worker", &index("w", None, ""));

    let out = run(&root, &["role", "list"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), line("team/worker", "w", "", &worker));
}

#[test]
fn role_list_ignores_old_standalone_roles() {
    let (_tmp, root) = root();
    write(
        &root.join(".agent-roles/roles/solo/index.md"),
        &index("alone", None, ""),
    );

    let out = run(&root, &["role", "list"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
}

#[test]
fn role_up_prints_template_then_role() {
    let (_tmp, root) = root();
    template_role(&root, "team", "worker", &index("w", None, "# Worker\n"));
    write(
        &root.join(".agent-graph/team/index.md"),
        &template_index("P", "# Team\n"),
    );

    let out = run(&root, &["role", "up", "team/worker"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "# Team\n# Worker\n");
}

#[test]
fn role_up_needs_template_slash_role() {
    let (_tmp, root) = root();
    template_role(&root, "team", "worker", &index("w", None, ""));

    let out = run(&root, &["role", "up", "worker"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("<template>/<role>"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn role_up_includes_files_of_its_template_only() {
    let (_tmp, root) = root();
    let dir = template_role(
        &root,
        "team",
        "worker",
        &index("w", None, "@../../shared/rules.md\n"),
    );
    write(&dir.join("../../shared/rules.md"), "shared rules\n");
    template_role(
        &root,
        "leak",
        "worker",
        &index("w", None, "@../../../team/shared/rules.md\n"),
    );

    let shared = run(&root, &["role", "up", "team/worker"]);
    let leak = run(&root, &["role", "up", "leak/worker"]);

    assert_eq!(stdout(&shared), "shared rules\n", "{}", stderr(&shared));
    assert_eq!(leak.status.code(), Some(1));
    assert!(!stdout(&leak).contains("shared rules"));
}

#[test]
fn role_up_with_pid_writes_template_and_role_state() {
    let (_tmp, root) = root();
    template_role(
        &root,
        "team",
        "worker",
        &role_index_with_icon("\u{f1322}", ""),
    );
    write(
        &root.join(".agent-graph/team/index.md"),
        &template_index("\u{f0849}", ""),
    );

    let state = up_state(&root, "team/worker", 4_000_001);

    assert_eq!(
        state,
        "{\"graph\":\"team\",\"graph_icon\":\"\u{f0849}\",\"node\":\"worker\",\"node_icon\":\"\u{f1322}\",\"pid\":4000001}\n"
    );
}

#[test]
fn role_up_with_pid_omits_missing_icons() {
    let (_tmp, root) = root();
    role(&root, "solo", &index("d", None, ""));

    let state = up_state(&root, "g/solo", 4_000_002);

    assert_eq!(
        state,
        "{\"graph\":\"g\",\"node\":\"solo\",\"pid\":4000002}\n"
    );
}

#[test]
fn role_list_warns_on_bad_count() {
    let (_tmp, root) = root();
    let ok = role(
        &root,
        "ok",
        "---\ntype: Role\ndescription: d\ncount: 2\n---\n",
    );
    for (name, count) in [("zero", "0"), ("word", "two"), ("minus", "-1")] {
        role(
            &root,
            name,
            &format!("---\ntype: Role\ndescription: d\ncount: {count}\n---\n"),
        );
    }

    let out = run(&root, &["role", "list"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), line("g/ok", "d", "", &ok));
    for name in ["zero", "word", "minus"] {
        assert!(
            stderr(&out)
                .lines()
                .any(|warning| warning.contains(name) && warning.contains("count")),
            "no count warning for {name} in:\n{}",
            stderr(&out)
        );
    }
}

#[test]
fn role_down_removes_state_file() {
    let (_tmp, root) = root();
    role(&root, "solo", &index("d", None, ""));
    let path = PathBuf::from("/tmp/agent-graph/4000003.json");
    assert!(
        run(&root, &["role", "up", "g/solo", "--pid", "4000003"])
            .status
            .success()
    );
    assert!(path.is_file());

    let out = run(&root, &["role", "down", "--pid", "4000003"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!path.exists());
}

#[test]
fn role_down_without_state_file_succeeds() {
    let (_tmp, root) = root();
    let _ = fs::remove_file("/tmp/agent-graph/4000004.json");

    let out = run(&root, &["role", "down", "--pid", "4000004"]);

    assert!(out.status.success(), "{}", stderr(&out));
}

#[test]
fn role_up_state_uses_template_title() {
    let (_tmp, root) = root();
    template_role(&root, "team", "worker", &index("w", None, ""));
    write(
        &root.join(".agent-graph/team/index.md"),
        "---\ntype: TeamTemplate\ndescription: d\ntitle: Default team\nscope: branch\n---\n",
    );

    let state = up_state(&root, "team/worker", 4_000_005);

    assert_eq!(
        state,
        "{\"graph\":\"Default team\",\"node\":\"worker\",\"pid\":4000005}\n"
    );
}
