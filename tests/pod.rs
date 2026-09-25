use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const CODING: &str = r#"---
type: Pod
description: One worker and one advisor per project and branch.
scope: branch
---

```mermaid
flowchart LR
  %% the pod
  subgraph coding
    worker(("worker")):::public
    advisor(("advisor")):::private
    worker ask-review@-->|"ask for review"| advisor
    advisor review-result@-->|"review result"| worker
  end
  outside[["user or other pod"]]
  outside external@<--> worker
```

## Boundaries

- No file changes outside the git branch of the pod.
- No `git merge`.

## Edges

### ask-review

The worker sends the diff to the advisor.

### review-result

The advisor answers with findings.

### external

All outside messages go through the worker.
"#;

/// A temp root, canonicalized so paths match what the binary prints on
/// macOS, where `/var` is a symlink to `/private/var`.
fn root() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().canonicalize().unwrap();
    for name in ["worker", "advisor"] {
        write(
            &path.join(".agent-roles/roles").join(name).join("index.md"),
            "---\ntype: Role\ndescription: d\n---\n",
        );
    }
    (tmp, path)
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Write `pods/<name>/index.md` under `level` and return the pod folder.
fn pod(level: &Path, name: &str, index: &str) -> PathBuf {
    let dir = level.join(".agent-roles/pods").join(name);
    write(&dir.join("index.md"), index);
    dir
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

/// Lint the coding pod with `from` replaced by `to`, and expect one error
/// that contains `needle`.
fn assert_lint_rejects(from: &str, to: &str, needle: &str) {
    let (_tmp, root) = root();
    assert!(CODING.contains(from), "fixture has no `{from}`");
    pod(&root, "coding", &CODING.replacen(from, to, 1));

    let out = run(&root, &["pod", "lint"]);

    assert_eq!(out.status.code(), Some(1), "{}", stdout(&out));
    assert!(
        stdout(&out).contains(needle),
        "no `{needle}` in:\n{}{}",
        stdout(&out),
        stderr(&out)
    );
}

#[test]
fn pod_list_shows_pods_from_cwd_and_parents() {
    let (_tmp, root) = root();
    let outer = pod(&root, "coding", CODING);
    let inner = pod(
        &root.join("x"),
        "docs",
        &CODING.replace("scope: branch", "scope: project"),
    );

    let out = run(&root.join("x/y"), &["pod", "list"]);

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
fn pod_get_prints_body_with_includes() {
    let (_tmp, root) = root();
    let dir = pod(
        &root,
        "coding",
        &CODING.replace("## Edges\n", "@extra.md\n\n## Edges\n"),
    );
    write(&dir.join("extra.md"), "Extra text.\n");

    let out = run(&root, &["pod", "get", "coding"]);

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
fn pod_lint_accepts_coding_example() {
    let (_tmp, root) = root();
    pod(&root, "coding", CODING);

    let out = run(&root, &["pod", "lint"]);

    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert_eq!(stdout(&out), "");
}

#[test]
fn pod_lint_reports_path_and_line() {
    let (_tmp, root) = root();
    let dir = pod(
        &root,
        "coding",
        &CODING.replace(
            "outside external@<--> worker",
            "outside external@<--> advisor",
        ),
    );

    let out = run(&root, &["pod", "lint", "coding"]);

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
fn pod_lint_rejects_external_to_private_edge() {
    assert_lint_rejects(
        "outside external@<--> worker",
        "outside external@<--> advisor",
        "private node `advisor`",
    );
}

#[test]
fn pod_lint_rejects_two_public_nodes() {
    assert_lint_rejects(
        "advisor((\"advisor\")):::private",
        "advisor((\"advisor\")):::public",
        "more than one public node",
    );
}

#[test]
fn pod_lint_rejects_no_public_node() {
    assert_lint_rejects(
        "worker((\"worker\")):::public",
        "worker((\"worker\")):::private",
        "no public node",
    );
}

#[test]
fn pod_lint_rejects_unclassed_node() {
    assert_lint_rejects(
        "advisor((\"advisor\")):::private",
        "advisor((\"advisor\"))",
        "`advisor` is not public or private",
    );
}

#[test]
fn pod_lint_accepts_class_statement() {
    let (_tmp, root) = root();
    pod(
        &root,
        "coding",
        &CODING.replace(
            "advisor((\"advisor\")):::private",
            "advisor((\"advisor\"))\n    class advisor private\n    classDef private stroke-width:1px",
        ),
    );

    let out = run(&root, &["pod", "lint"]);

    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
}

#[test]
fn pod_lint_rejects_pod_to_pod_edge_off_public() {
    let second = r#"  subgraph docs
    writer(("worker")):::public
    editor(("advisor")):::private
    writer ask-edit@--> editor
  end
  worker cross@--> editor
```"#;
    let edges = "\n### ask-edit\n\nAsk.\n\n### cross\n\nCross.\n";
    let (_tmp, root) = root();
    pod(
        &root,
        "coding",
        &(CODING.replacen(
            "```\n\n## Boundaries",
            &format!("{second}\n\n## Boundaries"),
            1,
        ) + edges),
    );

    let out = run(&root, &["pod", "lint"]);

    assert_eq!(out.status.code(), Some(1), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("private node `editor`"),
        "{}",
        stdout(&out)
    );
    assert_eq!(stdout(&out).lines().count(), 1, "{}", stdout(&out));
}

#[test]
fn pod_lint_rejects_unnamed_edge() {
    assert_lint_rejects("worker ask-review@-->", "worker -->", "edge has no name");
}

#[test]
fn pod_lint_rejects_duplicate_edge_name() {
    assert_lint_rejects(
        "advisor review-result@-->",
        "advisor ask-review@-->",
        "edge name `ask-review` is used more than once",
    );
}

#[test]
fn pod_lint_rejects_edge_without_section() {
    assert_lint_rejects(
        "### review-result\n\nThe advisor answers with findings.\n\n",
        "",
        "edge `review-result` has no `### review-result` section",
    );
}

#[test]
fn pod_lint_rejects_section_without_edge() {
    assert_lint_rejects(
        "### external\n",
        "### external\n\nText.\n\n### ghost\n",
        "section `ghost` names no edge",
    );
}

#[test]
fn pod_lint_rejects_empty_edge_section() {
    assert_lint_rejects(
        "The advisor answers with findings.\n",
        "",
        "section `review-result` is empty",
    );
}

#[test]
fn pod_lint_rejects_unknown_role() {
    assert_lint_rejects(
        "advisor((\"advisor\"))",
        "advisor((\"reviewer\"))",
        "role `reviewer` not found",
    );
}

#[test]
fn pod_lint_rejects_bad_scope() {
    assert_lint_rejects("scope: branch", "scope: repo", "scope");
}

#[test]
fn pod_lint_rejects_unknown_mermaid_line() {
    assert_lint_rejects(
        "  end\n",
        "  end\n  click worker callback\n",
        "unknown line",
    );
}

#[test]
fn pod_lint_rejects_empty_boundaries() {
    assert_lint_rejects(
        "- No file changes outside the git branch of the pod.\n- No `git merge`.\n",
        "",
        "Boundaries has no `- ` items",
    );
}

#[test]
fn pod_lint_rejects_boundaries_with_prose() {
    assert_lint_rejects(
        "- No `git merge`.\n",
        "- No `git merge`.\nBe nice.\n",
        "Boundaries must hold only `- ` list items",
    );
}

#[test]
fn pod_lint_unknown_pod_exits_1() {
    let (_tmp, root) = root();

    let out = run(&root, &["pod", "lint", "nope"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
}

#[test]
fn pod_nodes_prints_node_role_and_class() {
    let (_tmp, root) = root();
    pod(&root, "coding", CODING);

    let out = run(&root, &["pod", "nodes", "coding"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "worker\tworker\tpublic\nadvisor\tadvisor\tprivate\n"
    );
}

#[test]
fn pod_nodes_names_pod_local_role() {
    let (_tmp, root) = root();
    let dir = pod(
        &root,
        "coding",
        &CODING.replace("advisor((\"advisor\"))", "advisor((\"reviewer\"))"),
    );
    write(
        &dir.join("roles/reviewer/index.md"),
        "---\ntype: Role\ndescription: d\n---\n",
    );

    let nodes = run(&root, &["pod", "nodes", "coding"]);
    let lint = run(&root, &["pod", "lint"]);

    assert_eq!(
        stdout(&nodes),
        "worker\tworker\tpublic\nadvisor\tcoding/reviewer\tprivate\n"
    );
    assert!(lint.status.success(), "{}{}", stdout(&lint), stderr(&lint));
}

#[test]
fn pod_lint_accepts_default_pods() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    assert!(run(&root, &["init"]).status.success());

    let out = run(&root, &["pod", "lint"]);

    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
}
