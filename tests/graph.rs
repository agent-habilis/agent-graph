use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const CODING: &str = r#"---
type: Graph
description: One worker and one advisor per project and branch.
scope: branch
---

```mermaid
flowchart LR
  %% the graph
  subgraph coding
    worker(("worker")):::public
    advisor(("advisor")):::private
    worker ask-review@-->|"ask for review"| advisor
    advisor review-result@-->|"review result"| worker
  end
  outside[["user or other graph"]]
  outside external@<--> worker
```

## Boundaries

- No file changes outside the git branch of the graph.
- No `git merge`.

## Edges

### ask-review

The worker sends the diff to the advisor.

### review-result

The advisor answers with findings.

### external

All outside messages go through the worker.
"#;

const NODE_INDEX: &str = "---\ntype: Node\ndescription: d\n---\n";

/// A temp root, canonicalized so paths match what the binary prints on
/// macOS, where `/var` is a symlink to `/private/var`.
fn root() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().canonicalize().unwrap();
    for name in ["worker", "advisor"] {
        write(
            &path
                .join(".agent-graph/coding/nodes")
                .join(name)
                .join("index.md"),
            NODE_INDEX,
        );
    }
    (tmp, path)
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Write `<name>/index.md` in `.agent-graph/` under `level` and return the
/// graph folder.
fn graph(level: &Path, name: &str, index: &str) -> PathBuf {
    let dir = level.join(".agent-graph").join(name);
    write(&dir.join("index.md"), index);
    dir
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

/// Lint the coding graph with `from` replaced by `to`, and expect one error
/// that contains `needle`.
fn assert_lint_rejects(from: &str, to: &str, needle: &str) {
    let (_tmp, root) = root();
    assert!(CODING.contains(from), "fixture has no `{from}`");
    graph(&root, "coding", &CODING.replacen(from, to, 1));

    let out = run(&root, &["graph", "lint"]);

    assert_eq!(out.status.code(), Some(1), "{}", stdout(&out));
    assert!(
        stdout(&out).contains(needle),
        "no `{needle}` in:\n{}{}",
        stdout(&out),
        stderr(&out)
    );
}

#[test]
fn graph_list_shows_graphs_from_cwd_and_parents() {
    let (_tmp, root) = root();
    let outer = graph(&root, "coding", CODING);
    let inner = graph(
        &root.join("x"),
        "docs",
        &CODING.replace("scope: branch", "scope: project"),
    );

    let out = run(&root.join("x/y"), &["graph", "list"]);

    assert!(out.status.success(), "{}", stderr(&out));
    let description = "One worker and one advisor per project and branch.";
    assert_eq!(
        stdout(&out),
        format!(
            "coding\t{description}\tbranch\t{}\ndocs\t{description}\tproject\t{}\n",
            outer.display(),
            inner.display()
        )
    );
}

#[test]
fn graph_get_prints_body_with_includes() {
    let (_tmp, root) = root();
    let dir = graph(
        &root,
        "coding",
        &CODING.replace("## Edges\n", "@extra.md\n\n## Edges\n"),
    );
    write(&dir.join("extra.md"), "Extra text.\n");

    let out = run(&root, &["graph", "get", "coding"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).starts_with("\n```mermaid\n"),
        "{}",
        stdout(&out)
    );
    assert!(
        stdout(&out).contains("Extra text.\n\n## Edges\n"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn graph_lint_accepts_coding_example() {
    let (_tmp, root) = root();
    graph(&root, "coding", CODING);

    let out = run(&root, &["graph", "lint"]);

    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert_eq!(stdout(&out), "");
}

#[test]
fn graph_lint_reports_path_and_line() {
    let (_tmp, root) = root();
    let dir = graph(
        &root,
        "coding",
        &CODING.replace(
            "outside external@<--> worker",
            "outside external@<--> advisor",
        ),
    );

    let out = run(&root, &["graph", "lint", "coding"]);

    assert_eq!(out.status.code(), Some(1));
    let line = CODING
        .lines()
        .position(|line| line.contains("outside external@"))
        .unwrap()
        + 1;
    assert!(
        stdout(&out).starts_with(&format!("{}:{line}: ", dir.join("index.md").display())),
        "{}",
        stdout(&out)
    );
}

#[test]
fn graph_lint_rejects_external_to_private_edge() {
    assert_lint_rejects(
        "outside external@<--> worker",
        "outside external@<--> advisor",
        "private node `advisor`",
    );
}

#[test]
fn graph_lint_rejects_two_public_nodes() {
    assert_lint_rejects(
        "advisor((\"advisor\")):::private",
        "advisor((\"advisor\")):::public",
        "more than one public node",
    );
}

#[test]
fn graph_lint_rejects_no_public_node() {
    assert_lint_rejects(
        "worker((\"worker\")):::public",
        "worker((\"worker\")):::private",
        "no public node",
    );
}

#[test]
fn graph_lint_rejects_unclassed_node() {
    assert_lint_rejects(
        "advisor((\"advisor\")):::private",
        "advisor((\"advisor\"))",
        "`advisor` is not public or private",
    );
}

#[test]
fn graph_lint_accepts_class_statement() {
    let (_tmp, root) = root();
    graph(
        &root,
        "coding",
        &CODING.replace(
            "advisor((\"advisor\")):::private",
            "advisor((\"advisor\"))\n    class advisor private\n    classDef private stroke-width:1px",
        ),
    );

    let out = run(&root, &["graph", "lint"]);

    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
}

#[test]
fn graph_lint_rejects_graph_to_graph_edge_off_public() {
    let second = r#"  subgraph docs
    writer(("worker")):::public
    editor(("advisor")):::private
    writer ask-edit@--> editor
  end
  worker cross@--> editor
```"#;
    let edges = "\n### ask-edit\n\nAsk.\n\n### cross\n\nCross.\n";
    let (_tmp, root) = root();
    graph(
        &root,
        "coding",
        &(CODING.replacen(
            "```\n\n## Boundaries",
            &format!("{second}\n\n## Boundaries"),
            1,
        ) + edges),
    );

    let out = run(&root, &["graph", "lint"]);

    assert_eq!(out.status.code(), Some(1), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("private node `editor`"),
        "{}",
        stdout(&out)
    );
    assert_eq!(stdout(&out).lines().count(), 1, "{}", stdout(&out));
}

#[test]
fn graph_lint_rejects_unnamed_edge() {
    assert_lint_rejects("worker ask-review@-->", "worker -->", "edge has no name");
}

#[test]
fn graph_lint_rejects_duplicate_edge_name() {
    assert_lint_rejects(
        "advisor review-result@-->",
        "advisor ask-review@-->",
        "edge name `ask-review` is used more than once",
    );
}

#[test]
fn graph_lint_rejects_edge_without_section() {
    assert_lint_rejects(
        "### review-result\n\nThe advisor answers with findings.\n\n",
        "",
        "edge `review-result` has no `### review-result` section",
    );
}

#[test]
fn graph_lint_rejects_section_without_edge() {
    assert_lint_rejects(
        "### external\n",
        "### external\n\nText.\n\n### ghost\n",
        "section `ghost` names no edge",
    );
}

#[test]
fn graph_lint_rejects_empty_edge_section() {
    assert_lint_rejects(
        "The advisor answers with findings.\n",
        "",
        "section `review-result` is empty",
    );
}

#[test]
fn graph_lint_rejects_unknown_node() {
    assert_lint_rejects(
        "advisor((\"advisor\"))",
        "advisor((\"reviewer\"))",
        "node `reviewer` not found",
    );
}

#[test]
fn graph_lint_rejects_bad_scope() {
    assert_lint_rejects("scope: branch", "scope: repo", "scope");
}

#[test]
fn graph_lint_rejects_unknown_mermaid_line() {
    assert_lint_rejects(
        "  end\n",
        "  end\n  click worker callback\n",
        "unknown line",
    );
}

#[test]
fn graph_lint_rejects_empty_boundaries() {
    assert_lint_rejects(
        "- No file changes outside the git branch of the graph.\n- No `git merge`.\n",
        "",
        "Boundaries has no `- ` items",
    );
}

#[test]
fn graph_lint_rejects_boundaries_with_prose() {
    assert_lint_rejects(
        "- No `git merge`.\n",
        "- No `git merge`.\nBe nice.\n",
        "Boundaries must hold only `- ` list items",
    );
}

#[test]
fn graph_lint_unknown_graph_exits_1() {
    let (_tmp, root) = root();

    let out = run(&root, &["graph", "lint", "nope"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
}

#[test]
fn graph_nodes_prints_node_node_and_class() {
    let (_tmp, root) = root();
    graph(&root, "coding", CODING);

    let out = run(&root, &["graph", "nodes", "coding"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "worker\tcoding/worker\tpublic\t1\t\nadvisor\tcoding/advisor\tprivate\t1\t\n"
    );
}

#[test]
fn graph_nodes_names_graph_local_node() {
    let (_tmp, root) = root();
    let dir = graph(
        &root,
        "coding",
        &CODING.replace("advisor((\"advisor\"))", "advisor((\"reviewer\"))"),
    );
    write(&dir.join("nodes/reviewer/index.md"), NODE_INDEX);

    let nodes = run(&root, &["graph", "nodes", "coding"]);
    let lint = run(&root, &["graph", "lint"]);

    assert_eq!(
        stdout(&nodes),
        "worker\tcoding/worker\tpublic\t1\t\nadvisor\tcoding/reviewer\tprivate\t1\t\n"
    );
    assert!(lint.status.success(), "{}{}", stdout(&lint), stderr(&lint));
}

#[test]
fn graph_lint_accepts_default_graphs() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    assert!(run(&root, &["graph", "init"]).status.success());

    let out = run(&root, &["graph", "lint"]);

    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
}

#[test]
fn graph_nodes_prints_count_of_each_node() {
    let (_tmp, root) = root();
    let dir = graph(&root, "coding", CODING);
    write(
        &dir.join("nodes/advisor/index.md"),
        "---\ntype: Node\ndescription: d\ncount: 3\n---\n",
    );

    let out = run(&root, &["graph", "nodes", "coding"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "worker\tcoding/worker\tpublic\t1\t\nadvisor\tcoding/advisor\tprivate\t3\t\n"
    );
}

#[test]
fn graph_lint_rejects_node_with_bad_count() {
    let (_tmp, root) = root();
    let dir = graph(&root, "coding", CODING);
    write(
        &dir.join("nodes/advisor/index.md"),
        "---\ntype: Node\ndescription: d\ncount: 0\n---\n",
    );

    let out = run(&root, &["graph", "lint"]);

    assert_eq!(out.status.code(), Some(1), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("node `advisor` is not valid: count is `0`"),
        "{}",
        stdout(&out)
    );
}

fn hash(cwd: &Path, name: &str) -> String {
    let out = run(cwd, &["graph", "hash", name]);
    assert!(out.status.success(), "{}", stderr(&out));
    stdout(&out)
}

#[test]
fn graph_hash_is_same_for_same_content() {
    let (_tmp, root) = root();
    let first = graph(&root.join("a"), "coding", CODING);
    write(&first.join("nodes/worker/index.md"), NODE_INDEX);
    std::thread::sleep(std::time::Duration::from_millis(20));
    let second = graph(&root.join("b"), "coding", CODING);
    write(&second.join("nodes/worker/index.md"), NODE_INDEX);

    let first_hash = hash(&root.join("a"), "coding");
    let second_hash = hash(&root.join("b"), "coding");

    assert_eq!(first_hash, second_hash);
    assert_eq!(first_hash.trim().len(), 64, "{first_hash}");
    assert!(
        first_hash
            .trim()
            .chars()
            .all(|char| char.is_ascii_hexdigit()),
        "{first_hash}"
    );
}

#[test]
fn graph_hash_changes_when_a_file_changes() {
    let (_tmp, root) = root();
    let dir = graph(&root, "coding", CODING);
    let before = hash(&root, "coding");

    write(
        &dir.join("nodes/worker/index.md"),
        "---\ntype: Node\ndescription: e\n---\n",
    );

    assert_ne!(hash(&root, "coding"), before);
}

#[test]
fn graph_hash_unknown_graph_exits_1() {
    let (_tmp, root) = root();

    let out = run(&root, &["graph", "hash", "nope"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
}

#[test]
fn graph_nodes_prints_model_preference() {
    let (_tmp, root) = root();
    let dir = graph(&root, "coding", CODING);
    write(
        &dir.join("nodes/advisor/index.md"),
        "---\ntype: Node\ndescription: d\nmodel: fable\n---\n",
    );

    let out = run(&root, &["graph", "nodes", "coding"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "worker\tcoding/worker\tpublic\t1\t\nadvisor\tcoding/advisor\tprivate\t1\tfable\n"
    );
}
