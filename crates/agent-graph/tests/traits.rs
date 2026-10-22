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
const READ_ONLY: &str = "## Boundaries\n\n- No changes to files that git tracks, and no commits.\n";

/// Write the trait `<name>` under `level`, and return its folder.
fn write_trait(level: &Path, name: &str, index: &str) -> PathBuf {
    let dir = level.join(".agent-graph/traits").join(name);
    write(&dir.join("index.md"), index);
    dir
}

fn trait_index(description: &str, writes: Option<&str>, body: &str) -> String {
    let writes = writes
        .map(|value| format!("writes: {value}\n"))
        .unwrap_or_default();
    format!("---\ntype: Trait\ndescription: {description}\n{writes}---\n{body}")
}

/// Write the graph `g` with the node `<name>` under `level`.
fn node(level: &Path, name: &str, traits: Option<&str>) {
    write(&level.join(".agent-graph/g/index.md"), GRAPH_INDEX);
    let traits = traits
        .map(|list| format!("traits: {list}\n"))
        .unwrap_or_default();
    write(
        &level
            .join(".agent-graph/g/nodes")
            .join(name)
            .join("index.md"),
        &format!("---\ntype: Node\ndescription: n\n{traits}---\nN\n"),
    );
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

#[test]
fn trait_list_shows_traits_from_cwd_and_parents() {
    let (_tmp, root) = root();
    let outer = write_trait(&root, "qa", &trait_index("outer qa", None, "Q\n"));
    let inner = write_trait(
        &root.join("x"),
        "fix",
        &trait_index("inner fix", Some("true"), "F\n"),
    );

    let out = run(&root.join("x/y"), &["trait", "list"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        format!(
            "fix\tinner fix\ttrue\t{}\nqa\touter qa\tfalse\t{}\n",
            inner.display(),
            outer.display()
        )
    );
}

#[test]
fn nearest_trait_shadows_parent_trait() {
    let (_tmp, root) = root();
    write_trait(&root, "qa", &trait_index("outer", None, "O\n"));
    let inner = write_trait(&root.join("x"), "qa", &trait_index("inner", None, "I\n"));

    let out = run(&root.join("x"), &["trait", "list"]);

    assert_eq!(
        stdout(&out),
        format!("qa\tinner\tfalse\t{}\n", inner.display())
    );
}

#[test]
fn graph_list_does_not_read_the_traits_folder_as_a_graph() {
    let (_tmp, root) = root();
    write_trait(&root, "qa", &trait_index("qa", None, "Q\n"));

    let out = run(&root, &["graph", "list"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(!stdout(&out).contains("traits"), "{}", stdout(&out));
    assert!(!stderr(&out).contains("traits"), "{}", stderr(&out));
}

#[test]
fn trait_show_appends_the_read_only_boundary_by_default() {
    let (_tmp, root) = root();
    write_trait(&root, "qa", &trait_index("qa", None, "# QA\n\nRun it.\n"));

    let out = run(&root, &["trait", "show", "qa"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), format!("# QA\n\nRun it.\n\n{READ_ONLY}"));
}

#[test]
fn trait_show_has_no_boundary_when_the_trait_writes() {
    let (_tmp, root) = root();
    write_trait(&root, "fix", &trait_index("fix", Some("true"), "# Fix\n"));

    let out = run(&root, &["trait", "show", "fix"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "# Fix\n");
}

#[test]
fn trait_show_expands_includes() {
    let (_tmp, root) = root();
    let dir = write_trait(
        &root,
        "fix",
        &trait_index("fix", Some("true"), "@steps.md\n"),
    );
    write(&dir.join("steps.md"), "1. Step.\n");

    let out = run(&root, &["trait", "show", "fix"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "1. Step.\n");
}

#[test]
fn trait_show_runs_for_a_node_that_lists_the_trait() {
    let (_tmp, root) = root();
    write_trait(&root, "qa", &trait_index("qa", Some("true"), "Q\n"));
    node(&root, "worker", Some("[qa]"));

    let out = run(&root, &["trait", "show", "qa", "--node", "g/worker"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), "Q\n");
}

#[test]
fn trait_show_refuses_a_trait_that_the_node_does_not_list() {
    let (_tmp, root) = root();
    write_trait(&root, "qa", &trait_index("qa", None, "Q\n"));
    node(&root, "advisor", None);

    let out = run(&root, &["trait", "show", "qa", "--node", "g/advisor"]);

    assert!(!out.status.success());
    assert_eq!(stdout(&out), "");
    assert_eq!(
        stderr(&out),
        "error: node `g/advisor` does not have trait `qa`\n"
    );
}

#[test]
fn trait_show_fails_for_an_unknown_node() {
    let (_tmp, root) = root();
    write_trait(&root, "qa", &trait_index("qa", None, "Q\n"));

    let out = run(&root, &["trait", "show", "qa", "--node", "g/nobody"]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("node `g/nobody` not found"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn trait_show_fails_for_an_unknown_trait() {
    let (_tmp, root) = root();

    let out = run(&root, &["trait", "show", "qa"]);

    assert!(!out.status.success());
    assert_eq!(stderr(&out), "error: trait `qa` not found\n");
}

#[test]
fn trait_show_fails_for_a_folder_that_is_not_a_trait() {
    let (_tmp, root) = root();
    write_trait(&root, "qa", "---\ntype: Node\ndescription: qa\n---\nQ\n");

    let out = run(&root, &["trait", "show", "qa"]);

    assert!(!out.status.success());
    assert_eq!(
        stderr(&out),
        "error: trait `qa` is not valid: type is `Node`, must be `Trait`\n"
    );
}

#[test]
fn trait_show_fails_for_a_bad_writes_value() {
    let (_tmp, root) = root();
    write_trait(&root, "qa", &trait_index("qa", Some("yes"), "Q\n"));

    let out = run(&root, &["trait", "show", "qa"]);

    assert!(!out.status.success());
    assert_eq!(
        stderr(&out),
        "error: trait `qa` is not valid: writes is `yes`, must be `true` or `false`\n"
    );
}

#[test]
fn node_list_warns_for_traits_that_are_not_an_inline_list() {
    let (_tmp, root) = root();
    node(&root, "worker", Some("qa"));

    let out = run(&root, &["node", "list"]);

    assert_eq!(stdout(&out), "");
    assert!(
        stderr(&out).contains("traits must be an inline list"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn graph_init_writes_the_default_traits() {
    let (_tmp, root) = root();
    run(&root, &["graph", "init"]);

    let list = run(&root, &["trait", "list"]);
    let rows: Vec<(String, String)> = stdout(&list)
        .lines()
        .map(|row| {
            let cols: Vec<&str> = row.split('\t').collect();
            (cols[0].to_string(), cols[2].to_string())
        })
        .collect();
    let has = |node: &str, name: &str| {
        run(&root, &["trait", "show", name, "--node", node])
            .status
            .success()
    };

    assert!(list.status.success(), "{}", stderr(&list));
    assert_eq!(
        rows,
        [
            ("perf", "false"),
            ("qa", "false"),
            ("repro", "true"),
            ("review", "false")
        ]
        .map(|(name, writes)| (name.to_string(), writes.to_string()))
    );
    for graph in ["pair", "squad"] {
        let worker = format!("{graph}/worker");
        let advisor = format!("{graph}/advisor");
        for name in ["qa", "repro", "perf"] {
            assert!(has(&worker, name), "{worker} {name}");
            assert!(!has(&advisor, name), "{advisor} {name}");
        }
        assert!(has(&advisor, "review"), "{advisor} review");
        assert!(!has(&worker, "review"), "{worker} review");
    }
    for name in ["qa", "repro", "perf", "review"] {
        assert!(!has("squad/orchestrator", name), "orchestrator {name}");
    }
}

#[test]
fn trait_check_prints_the_description_and_writes_of_a_valid_file() {
    let (_tmp, root) = root();
    let file = root.join("download/index.md");
    write(&file, &trait_index("remote qa", Some("true"), "Q\n"));

    let out = run(&root, &["trait", "check", file.to_str().unwrap()]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        format!("remote qa\ttrue\t{}\n", file.display())
    );
}

#[test]
fn trait_check_reads_index_md_of_a_folder() {
    let (_tmp, root) = root();
    let dir = write_trait(&root, "qa", &trait_index("qa", None, "Q\n"));

    let out = run(&root, &["trait", "check", dir.to_str().unwrap()]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        format!("qa\tfalse\t{}\n", dir.join("index.md").display())
    );
}

#[test]
fn trait_check_fails_for_a_file_that_is_not_a_trait() {
    let (_tmp, root) = root();
    let file = root.join("download/index.md");
    write(&file, "<html>Not found</html>\n");

    let out = run(&root, &["trait", "check", file.to_str().unwrap()]);

    assert!(!out.status.success());
    assert_eq!(stdout(&out), "");
    assert_eq!(
        stderr(&out),
        format!(
            "error: {} is not a valid trait: no frontmatter with a `type:` key\n",
            file.display()
        )
    );
}

#[test]
fn trait_check_fails_for_a_missing_file() {
    let (_tmp, root) = root();
    let file = root.join("nothing.md");

    let out = run(&root, &["trait", "check", file.to_str().unwrap()]);

    assert!(!out.status.success());
    assert!(
        stderr(&out).starts_with(&format!("error: {} is not a valid trait: ", file.display())),
        "{}",
        stderr(&out)
    );
}

#[test]
fn trait_show_reads_a_path_and_skips_the_node_check() {
    let (_tmp, root) = root();
    let file = root.join("download/index.md");
    write(&file, &trait_index("remote", None, "R\n"));
    node(&root, "advisor", None);

    let out = run(
        &root,
        &[
            "trait",
            "show",
            file.to_str().unwrap(),
            "--node",
            "g/advisor",
        ],
    );

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(stdout(&out), format!("R\n\n{READ_ONLY}"));
}
