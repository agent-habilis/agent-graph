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

    assert_eq!(
        state,
        "{\"node\":\"solo\",\"node_icon\":\"S\",\"pid\":4000006}\n"
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
