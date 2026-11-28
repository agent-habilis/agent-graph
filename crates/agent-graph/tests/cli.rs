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

const GRAPH_INDEX: &str = "---\ntype: Graph\ndescription: g\nscope: branch\n---\n";

fn node_dir(level: &Path, graph: &str, name: &str) -> PathBuf {
    level
        .join(".agent-graph/graphs")
        .join(graph)
        .join("nodes")
        .join(name)
}

/// Write `<graph>/index.md` and `<graph>/nodes/<name>/index.md` under
/// `level`, and return the node folder.
fn graph_node(level: &Path, graph: &str, name: &str, index: &str) -> PathBuf {
    write(
        &level
            .join(".agent-graph/graphs")
            .join(graph)
            .join("index.md"),
        GRAPH_INDEX,
    );
    let dir = node_dir(level, graph, name);
    write(&dir.join("index.md"), index);
    dir
}

/// Write the standalone node `<name>` in `.agent-graph/nodes/` under `level`,
/// and return its folder.
fn node(level: &Path, name: &str, index: &str) -> PathBuf {
    let dir = level.join(".agent-graph/nodes").join(name);
    write(&dir.join("index.md"), index);
    dir
}

fn index(description: &str, tags: Option<&str>, body: &str) -> String {
    let tags = tags
        .map(|list| format!("tags: {list}\n"))
        .unwrap_or_default();
    format!("---\ntype: Node\ndescription: {description}\n{tags}---\n{body}")
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
fn node_list_shows_nodes_from_cwd_and_parents() {
    let (_tmp, root) = root();
    let outer = node(&root, "alpha", &index("outer node", None, "A\n"));
    let inner = node(&root.join("x"), "beta", &index("inner node", None, "B\n"));

    let out = run(&root.join("x/y"), &["node", "list"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        line("alpha", "outer node", "", &outer) + &line("beta", "inner node", "", &inner)
    );
}

#[test]
fn nearest_node_shadows_parent_node() {
    let (_tmp, root) = root();
    node(&root, "advisor", &index("outer", None, "outer body\n"));
    let inner = node(
        &root.join("x"),
        "advisor",
        &index("inner", None, "inner body\n"),
    );
    let cwd = root.join("x/y");

    let list = run(&cwd, &["node", "list"]);
    let get = run(&cwd, &["node", "up", "advisor"]);

    assert_eq!(stdout(&list), line("advisor", "inner", "", &inner));
    assert_eq!(stdout(&get), "inner body\n");
}

#[test]
fn node_list_warns_and_skips_invalid_node() {
    let (_tmp, root) = root();
    let ok = node(&root, "ok", &index("fine", None, ""));
    fs::create_dir_all(root.join(".agent-graph/nodes/no-index")).unwrap();
    node(
        &root,
        "wrong-type",
        "---\ntype: Note\ndescription: x\n---\n",
    );
    node(&root, "no-description", "---\ntype: Node\n---\n");
    node(
        &root,
        "block-tags",
        "---\ntype: Node\ndescription: x\ntags:\n  - a\n---\n",
    );

    let out = run(&root, &["node", "list"]);

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
fn node_list_prints_tags_column() {
    let (_tmp, root) = root();
    let dir = node(&root, "advisor", &index("d", Some("[review, go]"), ""));

    let out = run(&root, &["node", "list"]);

    assert_eq!(stdout(&out), line("advisor", "d", "review,go", &dir));
}

#[test]
fn node_list_filters_by_tag_regex() {
    let (_tmp, root) = root();
    let go = node(&root, "go-dev", &index("d", Some("[golang]"), ""));
    let mongo = node(&root, "mongo", &index("d", Some("[db, mongo]"), ""));
    node(&root, "rust", &index("d", Some("[rust]"), ""));
    node(&root, "untagged", &index("d", None, ""));

    let anchored = run(&root, &["node", "list", "--tag", "^go"]);
    let unanchored = run(&root, &["node", "list", "--tag", "go"]);

    assert_eq!(stdout(&anchored), line("go-dev", "d", "golang", &go));
    assert_eq!(
        stdout(&unanchored),
        line("go-dev", "d", "golang", &go) + &line("mongo", "d", "db,mongo", &mongo)
    );
}

#[test]
fn node_list_tag_filter_runs_after_shadowing() {
    let (_tmp, root) = root();
    node(&root, "advisor", &index("outer", Some("[x]"), ""));
    let inner = node(&root.join("a"), "advisor", &index("inner", Some("[y]"), ""));

    let parent_tag = run(&root.join("a"), &["node", "list", "--tag", "x"]);
    let inner_tag = run(&root.join("a"), &["node", "list", "--tag", "y"]);

    assert!(parent_tag.status.success());
    assert_eq!(stdout(&parent_tag), "");
    assert_eq!(stdout(&inner_tag), line("advisor", "inner", "y", &inner));
}

#[test]
fn node_list_invalid_tag_regex_exits_1() {
    let (_tmp, root) = root();

    let out = run(&root, &["node", "list", "--tag", "("]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
}

#[test]
fn node_up_prints_body_without_frontmatter() {
    let (_tmp, root) = root();
    node(
        &root,
        "advisor",
        &index("d", None, "# Advisor\n\nBe blunt.\n"),
    );

    let out = run(&root, &["node", "up", "advisor"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "# Advisor\n\nBe blunt.\n");
}

#[test]
fn node_up_expands_nested_includes() {
    let (_tmp, root) = root();
    let dir = node(&root, "advisor", &index("d", None, "A\n  @parts/b.md\nC\n"));
    write(&dir.join("parts/b.md"), "---\ntype: Note\n---\nB1\n@c.md\n");
    write(&dir.join("parts/c.md"), "C1\n");
    write(&dir.join("unused.md"), "never printed\n");

    let out = run(&root, &["node", "up", "advisor"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "A\nB1\nC1\nC\n");
}

#[test]
fn node_up_rejects_include_cycle() {
    let (_tmp, root) = root();
    let dir = node(&root, "advisor", &index("d", None, "@a.md\n"));
    write(&dir.join("a.md"), "@b.md\n");
    write(&dir.join("b.md"), "@a.md\n");

    let out = run(&root, &["node", "up", "advisor"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
    assert!(stderr(&out).contains("cycle"), "{}", stderr(&out));
}

#[test]
fn node_up_rejects_include_outside_graph() {
    let (_tmp, root) = root();
    write(&root.join("outside.md"), "secret\n");
    node(
        &root,
        "relative",
        &index("d", None, "@../../../../outside.md\n"),
    );
    let absolute = format!("@{}\n", root.join("outside.md").display());
    node(&root, "absolute", &index("d", None, &absolute));

    for name in ["relative", "absolute"] {
        let out = run(&root, &["node", "up", name]);

        assert_eq!(out.status.code(), Some(1), "{name}");
        assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
        assert!(!stdout(&out).contains("secret"), "{name}");
    }
}

#[test]
fn node_up_ignores_include_in_code_fence() {
    let (_tmp, root) = root();
    let body = "```md\n@missing.md\n```\n";
    node(&root, "advisor", &index("d", None, body));

    let out = run(&root, &["node", "up", "advisor"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), body);
}

#[test]
fn node_up_unknown_node_exits_1() {
    let (_tmp, root) = root();

    let out = run(&root, &["node", "up", "nope"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
}

#[test]
fn node_up_prints_node_with_valid_boundaries() {
    let (_tmp, root) = root();
    let body = "# Worker\n\n## Boundaries\n\n- No `git merge`.\n- No file changes\n  outside the branch.\n\n## Tone\n\nBe brief.\n";
    node(&root, "worker", &index("d", None, body));

    let out = run(&root, &["node", "up", "worker"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), body);
}

#[test]
fn node_up_rejects_node_with_bad_boundaries() {
    let (_tmp, root) = root();
    node(
        &root,
        "empty",
        &index("d", None, "## Boundaries\n\n## Next\n"),
    );
    node(
        &root,
        "prose",
        &index("d", None, "## Boundaries\n\n- No merge.\nAlso be nice.\n"),
    );

    for name in ["empty", "prose"] {
        let out = run(&root, &["node", "up", name]);

        assert_eq!(out.status.code(), Some(1), "{name}");
        assert!(stderr(&out).contains("Boundaries"), "{}", stderr(&out));
    }
}

#[test]
fn node_list_warns_on_node_with_bad_boundaries() {
    let (_tmp, root) = root();
    let ok = node(
        &root,
        "ok",
        &index("fine", None, "## Boundaries\n- No merge.\n"),
    );
    node(&root, "empty", &index("d", None, "## Boundaries\n"));

    let out = run(&root, &["node", "list"]);

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
fn graph_init_writes_defaults_into_cwd() {
    let (_tmp, root) = root();

    let out = run(&root, &["graph", "init"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(tree(&root.join(".agent-graph")), defaults());
    let names: Vec<String> = stdout(&run(&root, &["node", "list"]))
        .lines()
        .map(|line| line.split('\t').next().unwrap().to_string())
        .collect();
    assert_eq!(names, ["advisor", "worker"]);
}

#[test]
fn graph_init_squad1_has_opus_leader_sonnet_workers_and_fable_advisor() {
    let (_tmp, root) = root();
    run(&root, &["graph", "init"]);

    let out = run(&root, &["graph", "nodes", "squad1"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "leader\tsquad1/leader\tlead\t1\topus\n\
         worker\tsquad1/worker\t-\t2\tsonnet\n\
         advisor\tsquad1/advisor\t-\t1\tfable\n"
    );
}

#[test]
fn graph_init_squad2_has_sonnet_leader_haiku_workers_and_opus_advisor() {
    let (_tmp, root) = root();
    run(&root, &["graph", "init"]);

    let out = run(&root, &["graph", "nodes", "squad2"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "leader\tsquad2/leader\tlead\t1\tsonnet\n\
         worker\tsquad2/worker\t-\t2\thaiku\n\
         advisor\tsquad2/advisor\t-\t1\topus\n"
    );
}

#[test]
fn graph_init_prints_each_written_path() {
    let (_tmp, root) = root();

    let out = run(&root, &["graph", "init"]);

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
fn graph_init_writes_into_dir_argument() {
    let (_tmp, root) = root();
    let target = root.join("project");
    fs::create_dir_all(&target).unwrap();

    let out = run(&root, &["graph", "init", target.to_str().unwrap()]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        target
            .join(".agent-graph/graphs/squad1/nodes/worker/index.md")
            .is_file()
    );
    assert!(!root.join(".agent-graph").exists());
}

#[test]
fn graph_init_does_nothing_when_folder_exists() {
    let (_tmp, root) = root();
    write(&root.join(".agent-graph/mine/index.md"), "keep me\n");
    let before = tree(&root.join(".agent-graph"));

    let out = run(&root, &["graph", "init"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).contains("already exists"), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
    assert_eq!(tree(&root.join(".agent-graph")), before);
}

#[test]
fn graph_init_does_nothing_when_file_exists() {
    let (_tmp, root) = root();
    write(&root.join(".agent-graph"), "a file\n");

    let out = run(&root, &["graph", "init"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).contains("already exists"), "{}", stderr(&out));
    assert_eq!(
        fs::read_to_string(root.join(".agent-graph")).unwrap(),
        "a file\n"
    );
}

fn graph_index(icon: &str, body: &str) -> String {
    format!("---\ntype: Graph\ndescription: d\nicon: {icon}\nscope: branch\n---\n{body}")
}

fn node_index_with_icon(icon: &str, body: &str) -> String {
    format!("---\ntype: Node\ndescription: d\nicon: {icon}\n---\n{body}")
}

/// Run `node up <name>`, or `graph node-up <graph>/<node>`, with `--pid <pid>`
/// and return the state file it wrote. Each
/// test uses its own pid, because the state folder is shared.
fn up_state(cwd: &Path, name: &str, pid: u32) -> String {
    let path = PathBuf::from(format!("/tmp/agent-graph/{pid}.json"));
    let _ = fs::remove_file(&path);

    let up = if name.contains('/') {
        ["graph", "node-up"]
    } else {
        ["node", "up"]
    };
    let out = run(cwd, &[up[0], up[1], name, "--pid", &pid.to_string()]);

    assert!(out.status.success(), "{}", stderr(&out));
    let state = fs::read_to_string(&path).unwrap();
    fs::remove_file(&path).unwrap();
    state
}

#[test]
fn node_list_skips_nodes_of_graphs() {
    let (_tmp, root) = root();
    graph_node(&root, "graph", "worker", &index("w", None, ""));

    let out = run(&root, &["node", "list"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
}

#[test]
fn node_list_ignores_old_standalone_nodes() {
    let (_tmp, root) = root();
    write(
        &root.join(".agent-nodes/nodes/solo/index.md"),
        &index("alone", None, ""),
    );

    let out = run(&root, &["node", "list"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
}

#[test]
fn node_list_names_standalone_nodes_bare() {
    let (_tmp, root) = root();
    let solo = node(&root, "solo", &index("alone", None, ""));
    graph_node(&root, "graph", "worker", &index("w", None, ""));

    let out = run(&root, &["node", "list"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), line("solo", "alone", "", &solo));
}

#[test]
fn node_up_prints_only_the_body_of_a_node() {
    let (_tmp, root) = root();
    graph_node(
        &root,
        "graph",
        "solo",
        &index("in a graph", None, "graph body\n"),
    );
    node(&root, "solo", &index("alone", None, "solo body\n"));

    let out = run(&root, &["node", "up", "solo"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "solo body\n");
}

#[test]
fn node_up_takes_the_nearest_node() {
    let (_tmp, root) = root();
    node(&root, "solo", &index("far", None, "far\n"));
    let near = root.join("project");
    node(&near, "solo", &index("near", None, "near\n"));

    let out = run(&near, &["node", "up", "solo"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "near\n");
}

#[test]
fn node_up_records_a_standalone_node_without_a_graph() {
    let (_tmp, root) = root();
    node(&root, "solo", &node_index_with_icon("S", ""));

    let state = up_state(&root, "solo", 4_000_006);

    let mut value: serde_json::Value = serde_json::from_str(&state).unwrap();
    let name = value.as_object_mut().unwrap().remove("name").unwrap();
    assert!(is_short_hash(name.as_str().unwrap()), "{state}");
    assert_eq!(
        value.to_string(),
        "{\"node\":\"solo\",\"node_icon\":\"S\",\"pid\":4000006}"
    );
}

#[test]
fn node_up_ignores_a_graph_outside_graphs_folder() {
    let (_tmp, root) = root();
    write(&root.join(".agent-graph/old/index.md"), GRAPH_INDEX);
    write(
        &root.join(".agent-graph/old/nodes/worker/index.md"),
        &index("w", None, ""),
    );

    let out = run(&root, &["graph", "node-up", "old/worker"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("not found"), "{}", stderr(&out));
}

#[test]
fn node_up_prints_graph_then_node() {
    let (_tmp, root) = root();
    graph_node(&root, "graph", "worker", &index("w", None, "# Worker\n"));
    write(
        &root.join(".agent-graph/graphs/graph/index.md"),
        &graph_index("P", "# Graph\n"),
    );

    let out = run(&root, &["graph", "node-up", "graph/worker"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "# Graph\n# Worker\n");
}

#[test]
fn graph_node_up_needs_graph_slash_node() {
    let (_tmp, root) = root();
    graph_node(&root, "graph", "worker", &index("w", None, ""));

    let out = run(&root, &["graph", "node-up", "worker"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("<graph>/<node>"), "{}", stderr(&out));
}

#[test]
fn node_up_does_not_load_a_node_of_a_graph() {
    let (_tmp, root) = root();
    graph_node(
        &root,
        "graph",
        "worker",
        &index("w", None, "graph worker\n"),
    );

    let out = run(&root, &["node", "up", "graph/worker"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("not found"), "{}", stderr(&out));
    assert!(!stdout(&out).contains("graph worker"));
}

#[test]
fn graph_node_down_removes_state_file() {
    let (_tmp, root) = root();
    graph_node(&root, "graph", "worker", &index("w", None, ""));
    let path = PathBuf::from("/tmp/agent-graph/4000007.json");
    assert!(
        run(
            &root,
            &["graph", "node-up", "graph/worker", "--pid", "4000007"]
        )
        .status
        .success()
    );
    assert!(path.is_file());

    let out = run(&root, &["graph", "node-down", "--pid", "4000007"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!path.exists());
}

#[test]
fn graph_node_up_rejects_a_path_as_node_name() {
    let (_tmp, root) = root();
    graph_node(&root, "graph", "worker", &index("w", None, ""));
    graph_node(&root, "other", "worker", &index("w", None, "other\n"));

    for name in ["graph/../../other/nodes/worker", "graph/..", "graph/"] {
        let out = run(&root, &["graph", "node-up", name]);

        assert_eq!(out.status.code(), Some(1), "{name}");
        assert!(!stdout(&out).contains("other"), "{name}");
    }
}

#[test]
fn node_up_includes_files_of_its_graph_only() {
    let (_tmp, root) = root();
    let dir = graph_node(
        &root,
        "graph",
        "worker",
        &index("w", None, "@../../shared/rules.md\n"),
    );
    write(&dir.join("../../shared/rules.md"), "shared rules\n");
    graph_node(
        &root,
        "leak",
        "worker",
        &index("w", None, "@../../../graph/shared/rules.md\n"),
    );

    let shared = run(&root, &["graph", "node-up", "graph/worker"]);
    let leak = run(&root, &["graph", "node-up", "leak/worker"]);

    assert_eq!(stdout(&shared), "shared rules\n", "{}", stderr(&shared));
    assert_eq!(leak.status.code(), Some(1));
    assert!(!stdout(&leak).contains("shared rules"));
}

#[test]
fn node_up_with_pid_writes_graph_and_node_state() {
    let (_tmp, root) = root();
    graph_node(
        &root,
        "graph",
        "worker",
        &node_index_with_icon("\u{f1322}", ""),
    );
    write(
        &root.join(".agent-graph/graphs/graph/index.md"),
        &graph_index("\u{f0849}", ""),
    );

    let state = up_state(&root, "graph/worker", 4_000_001);

    assert_eq!(
        state,
        "{\"graph\":\"graph\",\"graph_icon\":\"\u{f0849}\",\"node\":\"worker\",\"node_icon\":\"\u{f1322}\",\"pid\":4000001}\n"
    );
}

#[test]
fn node_up_with_pid_omits_missing_icons() {
    let (_tmp, root) = root();
    graph_node(&root, "g", "solo", &index("d", None, ""));

    let state = up_state(&root, "g/solo", 4_000_002);

    assert_eq!(
        state,
        "{\"graph\":\"g\",\"node\":\"solo\",\"pid\":4000002}\n"
    );
}

#[test]
fn node_list_warns_on_bad_count() {
    let (_tmp, root) = root();
    let ok = node(
        &root,
        "ok",
        "---\ntype: Node\ndescription: d\ncount: 2\n---\n",
    );
    for (name, count) in [("zero", "0"), ("word", "two"), ("minus", "-1")] {
        node(
            &root,
            name,
            &format!("---\ntype: Node\ndescription: d\ncount: {count}\n---\n"),
        );
    }

    let out = run(&root, &["node", "list"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), line("ok", "d", "", &ok));
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
fn node_down_removes_state_file() {
    let (_tmp, root) = root();
    node(&root, "solo", &index("d", None, ""));
    let path = PathBuf::from("/tmp/agent-graph/4000003.json");
    assert!(
        run(&root, &["node", "up", "solo", "--pid", "4000003"])
            .status
            .success()
    );
    assert!(path.is_file());

    let out = run(&root, &["node", "down", "--pid", "4000003"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!path.exists());
}

#[test]
fn node_down_without_state_file_succeeds() {
    let (_tmp, root) = root();
    let _ = fs::remove_file("/tmp/agent-graph/4000004.json");

    let out = run(&root, &["node", "down", "--pid", "4000004"]);

    assert!(out.status.success(), "{}", stderr(&out));
}

#[test]
fn node_up_state_uses_graph_title() {
    let (_tmp, root) = root();
    graph_node(&root, "graph", "worker", &index("w", None, ""));
    write(
        &root.join(".agent-graph/graphs/graph/index.md"),
        "---\ntype: Graph\ndescription: d\ntitle: Default graph\nscope: branch\n---\n",
    );

    let state = up_state(&root, "graph/worker", 4_000_005);

    assert_eq!(
        state,
        "{\"graph\":\"Default graph\",\"node\":\"worker\",\"pid\":4000005}\n"
    );
}

fn is_short_hash(text: &str) -> bool {
    text.len() == 7
        && text
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_uppercase())
}

#[test]
fn name_prints_a_short_hash_that_differs_between_calls() {
    let (_tmp, root) = root();

    let first = stdout(&run(&root, &["name"]));
    let second = stdout(&run(&root, &["name"]));

    assert!(is_short_hash(first.trim_end()), "{first:?}");
    assert!(first.ends_with('\n'));
    assert_ne!(first, second);
}

#[test]
fn node_up_with_name_writes_the_name_to_the_state() {
    let (_tmp, root) = root();
    node(&root, "solo", &index("d", None, ""));
    let path = PathBuf::from("/tmp/agent-graph/4000008.json");
    let _ = fs::remove_file(&path);

    let out = run(
        &root,
        &["node", "up", "solo", "--name", "mine", "--pid", "4000008"],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "{\"name\":\"mine\",\"node\":\"solo\",\"pid\":4000008}\n"
    );
}

#[test]
fn node_up_without_name_writes_a_short_hash_name() {
    let (_tmp, root) = root();
    node(&root, "solo", &index("d", None, ""));

    let state = up_state(&root, "solo", 4_000_009);

    let value: serde_json::Value = serde_json::from_str(&state).unwrap();
    assert!(is_short_hash(value["name"].as_str().unwrap()), "{state}");
}

#[test]
fn graph_node_up_with_name_writes_the_name_to_the_state() {
    let (_tmp, root) = root();
    graph_node(&root, "g", "solo", &index("d", None, ""));
    let path = PathBuf::from("/tmp/agent-graph/4000010.json");
    let _ = fs::remove_file(&path);

    let out = run(
        &root,
        &[
            "graph", "node-up", "g/solo", "--name", "a3f9c1d", "--pid", "4000010",
        ],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "{\"graph\":\"g\",\"name\":\"a3f9c1d\",\"node\":\"solo\",\"pid\":4000010}\n"
    );
}

#[test]
fn default_graphs_send_user_questions_to_the_leader() {
    for graph in ["squad1", "squad2", "pair"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("defaults/graphs")
            .join(graph)
            .join("index.md");
        let text = fs::read_to_string(&path).unwrap();

        assert!(
            text.contains("## Questions for the user"),
            "{graph} has no section on questions for the user"
        );
        for needle in [
            "\"leader\":true",
            "`instance` and `hash` as you",
            "no live leader",
            "accept or decline question of gossip-task",
            "Read the meta each",
        ] {
            assert!(text.contains(needle), "{graph} lacks `{needle}`");
        }
    }
}

/// Run the binary behind `sh -c`, so that a shell stands between this test and
/// the command, as it does for an agent. The `exit` keeps the shell from
/// replacing itself with the command.
fn run_in_shell(cwd: &Path, args: &[&str]) -> Output {
    fs::create_dir_all(cwd).unwrap();
    Command::new("sh")
        .args(["-c", "\"$0\" \"$@\"; exit $?"])
        .arg(env!("CARGO_BIN_EXE_agent-graph"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}

#[test]
fn pid_auto_names_the_agent_above_the_shell_for_up_and_down() {
    let (_tmp, root) = root();
    node(&root, "solo", &index("d", None, ""));
    graph_node(&root, "graph", "worker", &index("w", None, ""));
    // This test process is the agent, and `sh` is the shell below it.
    let agent = std::process::id();
    let path = PathBuf::from(format!("/tmp/agent-graph/{agent}.json"));
    let _ = fs::remove_file(&path);

    for (up, down, name) in [
        (["node", "up"], ["node", "down"], "solo"),
        (["graph", "node-up"], ["graph", "node-down"], "graph/worker"),
    ] {
        let out = run_in_shell(&root, &[up[0], up[1], name, "--pid", "auto"]);
        assert!(out.status.success(), "{}", stderr(&out));
        // One line says which process `auto` found, so a wrong one shows.
        assert!(
            stderr(&out).contains(&format!("--pid auto is process {agent} (")),
            "{}",
            stderr(&out)
        );
        let state: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(state["pid"], agent, "{state}");

        let removed = run_in_shell(&root, &[down[0], down[1], "--pid", "auto"]);
        assert!(removed.status.success(), "{}", stderr(&removed));
        assert!(!path.exists(), "{name}: the state file stays");
    }

    // Only one test uses the pid of this process, so no other thread writes
    // or removes the file while the checks run.
    let plain = run_in_shell(&root, &["node", "up", "solo"]);
    assert!(plain.status.success(), "{}", stderr(&plain));
    assert!(!path.exists(), "a plain node up wrote a state file");
}

#[test]
fn pid_accepts_a_number_or_auto_and_nothing_else() {
    let (_tmp, root) = root();
    node(&root, "solo", &index("d", None, ""));

    let out = run(&root, &["node", "up", "solo", "--pid", "soon"]);

    assert_eq!(out.status.code(), Some(2));
    assert!(stderr(&out).contains("a pid or `auto`"), "{}", stderr(&out));
}

#[test]
fn graph_node_hash_is_the_sha256_of_what_graph_node_up_prints() {
    let (_tmp, root) = root();
    graph_node(&root, "graph", "worker", &index("w", None, "worker body\n"));

    let up = run(&root, &["graph", "node-up", "graph/worker"]);
    let out = run(&root, &["graph", "node-hash", "graph/worker"]);

    assert!(up.status.success(), "{}", stderr(&up));
    assert!(out.status.success(), "{}", stderr(&out));
    // The oracle is `shasum -a 256` on the same text, run by hand once.
    assert_eq!(stdout(&up), "worker body\n");
    assert_eq!(
        stdout(&out),
        "0b625c8918641b1c2414630cfba461ab4007c2f76bf05cfbcda268ad6fb07066\n"
    );
}

#[test]
fn graph_node_hash_writes_no_state_file_and_changes_with_the_node() {
    let (_tmp, root) = root();
    let worker = graph_node(&root, "graph", "worker", &index("w", None, "one\n"));
    let first = run(&root, &["graph", "node-hash", "graph/worker"]);
    write(&worker.join("index.md"), &index("w", None, "two\n"));
    let second = run(&root, &["graph", "node-hash", "graph/worker"]);

    assert!(first.status.success() && second.status.success());
    assert_ne!(stdout(&first), stdout(&second));
}

#[test]
fn graph_node_hash_rejects_the_names_that_graph_node_up_rejects() {
    let (_tmp, root) = root();
    graph_node(&root, "graph", "worker", &index("w", None, ""));

    for name in ["worker", "graph/nope", "nope/worker", "graph/../x"] {
        let up = run(&root, &["graph", "node-up", name]);
        let hash = run(&root, &["graph", "node-hash", name]);

        assert_eq!(hash.status.code(), up.status.code(), "{name}");
        assert_eq!(stderr(&hash), stderr(&up), "{name}");
    }
}

#[test]
fn graph_node_up_output_writes_the_text_to_the_file_and_prints_nothing() {
    let (_tmp, root) = root();
    graph_node(&root, "graph", "worker", &index("w", None, "worker body\n"));
    let file = root.join("context.md");

    let plain = run(&root, &["graph", "node-up", "graph/worker"]);
    let out = run(
        &root,
        &[
            "graph",
            "node-up",
            "graph/worker",
            "--output",
            file.to_str().unwrap(),
        ],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "");
    assert_eq!(fs::read(&file).unwrap(), plain.stdout);
}

#[test]
fn graph_node_up_output_to_a_folder_that_is_missing_fails() {
    let (_tmp, root) = root();
    graph_node(&root, "graph", "worker", &index("w", None, ""));
    let file = root.join("no-such-folder/context.md");

    let out = run(
        &root,
        &[
            "graph",
            "node-up",
            "graph/worker",
            "--output",
            file.to_str().unwrap(),
        ],
    );

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("context.md"), "{}", stderr(&out));
}

#[test]
fn graph_node_up_writes_the_output_file_before_the_statusline_record() {
    let (_tmp, root) = root();
    graph_node(&root, "graph", "worker", &index("w", None, ""));
    let missing = root.join("no-such-folder/context.md");
    let state = PathBuf::from("/tmp/agent-graph/4000012.json");
    let _ = fs::remove_file(&state);

    let out = run(
        &root,
        &[
            "graph",
            "node-up",
            "graph/worker",
            "--pid",
            "4000012",
            "--output",
            missing.to_str().unwrap(),
        ],
    );

    assert_eq!(out.status.code(), Some(1));
    assert!(!state.exists(), "the statusline record was written first");
}

#[cfg(unix)]
#[test]
fn graph_node_up_output_does_not_write_through_a_link() {
    let (_tmp, root) = root();
    graph_node(&root, "graph", "worker", &index("w", None, ""));
    let target = root.join("precious.txt");
    fs::write(&target, "keep\n").unwrap();
    let link = root.join("context.md");
    std::os::unix::fs::symlink(&target, &link).unwrap();

    let out = run(
        &root,
        &[
            "graph",
            "node-up",
            "graph/worker",
            "--output",
            link.to_str().unwrap(),
        ],
    );

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("symbolic link"), "{}", stderr(&out));
    assert_eq!(fs::read_to_string(&target).unwrap(), "keep\n");
}

#[test]
fn dir_starts_discovery_in_that_folder_wherever_the_command_runs() {
    let (_tmp, root) = root();
    let dir = node(&root.join("a"), "n", &index("d", None, "body\n"));
    let elsewhere = root.join("elsewhere");
    let start = root.join("a/b");
    fs::create_dir_all(&start).unwrap();

    let none = run(&elsewhere, &["node", "list"]);
    let after = run(
        &elsewhere,
        &["node", "list", "--dir", start.to_str().unwrap()],
    );
    let before = run(
        &elsewhere,
        &["--dir", start.to_str().unwrap(), "node", "list"],
    );

    assert_eq!(stdout(&none), "");
    assert_eq!(
        stdout(&after),
        line("n", "d", "", &dir),
        "{}",
        stderr(&after)
    );
    assert_eq!(stdout(&before), stdout(&after));
}

#[test]
fn dir_expands_a_leading_tilde_to_home() {
    let (_tmp, root) = root();
    let dir = node(&root.join("home"), "n", &index("d", None, "body\n"));
    let elsewhere = root.join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();

    for arg in ["~", "~/"] {
        let out = Command::new(env!("CARGO_BIN_EXE_agent-graph"))
            .args(["node", "list", "--dir", arg])
            .env("HOME", root.join("home"))
            .current_dir(&elsewhere)
            .output()
            .unwrap();
        assert_eq!(
            stdout(&out),
            line("n", "d", "", &dir),
            "{arg}: {}",
            stderr(&out)
        );
    }
}

#[test]
fn dir_that_is_not_a_folder_fails_and_names_it() {
    let (_tmp, root) = root();
    let missing = root.join("missing");

    let out = run(&root, &["node", "list", "--dir", missing.to_str().unwrap()]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("--dir"), "{}", stderr(&out));
    assert!(stderr(&out).contains("missing"), "{}", stderr(&out));
}

#[test]
fn node_up_rejects_an_include_of_an_apple_double_file() {
    let (_tmp, root) = root();
    let dir = node(&root, "advisor", &index("d", None, "A\n@._note.md\n"));
    write(&dir.join("._note.md"), "never printed\n");
    write(&dir.join("sub/._dir/x.md"), "never printed\n");
    node(&root, "deep", &index("d", None, "@sub/._dir/x.md\n"));
    let sub = dir.join("sub");
    fs::create_dir_all(&sub).unwrap();

    for name in ["advisor", "deep"] {
        let out = run(&root, &["node", "up", name]);

        assert_eq!(out.status.code(), Some(1), "{name}");
        assert!(stderr(&out).contains("._"), "{name}: {}", stderr(&out));
        assert!(!stdout(&out).contains("never printed"), "{name}");
    }
}
