use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const CODING: &str = r#"---
type: TeamTemplate
description: One worker and one advisor per project and branch.
scope: branch
---

```mermaid
flowchart LR
  %% the team
  subgraph coding
    worker(("worker")):::lead
    advisor(("advisor"))
    worker ask-review@-->|"ask for review"| advisor
    advisor review-result@-->|"review result"| worker
  end
  outside[["user or other team"]]
  outside external@<--> worker
```

## Boundaries

- No file changes outside the git branch of the team.
- No `git merge`.

## Handoffs

### ask-review

The worker sends the diff to the advisor.

### review-result

The advisor answers with findings.

### external

All outside messages go through the worker.
"#;

const ROLE_INDEX: &str = "---\ntype: Role\ndescription: d\n---\n";

/// A temp root, canonicalized so paths match what the binary prints on
/// macOS, where `/var` is a symlink to `/private/var`.
fn root() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().canonicalize().unwrap();
    for name in ["worker", "advisor"] {
        write(
            &path
                .join(".agent-graph/coding/roles")
                .join(name)
                .join("index.md"),
            ROLE_INDEX,
        );
    }
    (tmp, path)
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Write `<name>/index.md` in `.agent-graph/` under `level` and return the
/// template folder.
fn template(level: &Path, name: &str, index: &str) -> PathBuf {
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

/// Lint the coding template with `from` replaced by `to`, and expect one error
/// that contains `needle`.
fn assert_lint_rejects(from: &str, to: &str, needle: &str) {
    let (_tmp, root) = root();
    assert!(CODING.contains(from), "fixture has no `{from}`");
    template(&root, "coding", &CODING.replacen(from, to, 1));

    let out = run(&root, &["template", "lint"]);

    assert_eq!(out.status.code(), Some(1), "{}", stdout(&out));
    assert!(
        stdout(&out).contains(needle),
        "no `{needle}` in:\n{}{}",
        stdout(&out),
        stderr(&out)
    );
}

#[test]
fn template_list_shows_templates_from_cwd_and_parents() {
    let (_tmp, root) = root();
    let outer = template(&root, "coding", CODING);
    let inner = template(
        &root.join("x"),
        "docs",
        &CODING.replace("scope: branch", "scope: project"),
    );

    let out = run(&root.join("x/y"), &["template", "list"]);

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
fn template_get_prints_body_with_includes() {
    let (_tmp, root) = root();
    let dir = template(
        &root,
        "coding",
        &CODING.replace("## Handoffs\n", "@extra.md\n\n## Handoffs\n"),
    );
    write(&dir.join("extra.md"), "Extra text.\n");

    let out = run(&root, &["template", "get", "coding"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).starts_with("\n```mermaid\n"),
        "{}",
        stdout(&out)
    );
    assert!(
        stdout(&out).contains("Extra text.\n\n## Handoffs\n"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn template_lint_accepts_coding_example() {
    let (_tmp, root) = root();
    template(&root, "coding", CODING);

    let out = run(&root, &["template", "lint"]);

    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
    assert_eq!(stdout(&out), "");
}

#[test]
fn template_lint_reports_path_and_line() {
    let (_tmp, root) = root();
    let dir = template(
        &root,
        "coding",
        &CODING.replace(
            "outside external@<--> worker",
            "outside external@<--> advisor",
        ),
    );

    let out = run(&root, &["template", "lint", "coding"]);

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
fn template_lint_rejects_external_handoff_to_a_role_that_is_not_the_lead() {
    assert_lint_rejects(
        "outside external@<--> worker",
        "outside external@<--> advisor",
        "at role `advisor`, which is not the lead",
    );
}

#[test]
fn template_lint_rejects_two_leads() {
    assert_lint_rejects(
        "advisor((\"advisor\"))\n",
        "advisor((\"advisor\")):::lead\n",
        "more than one lead",
    );
}

#[test]
fn template_lint_rejects_no_lead() {
    assert_lint_rejects(
        "worker((\"worker\")):::lead",
        "worker((\"worker\"))",
        "has no lead",
    );
}

#[test]
fn template_lint_accepts_class_statement() {
    let (_tmp, root) = root();
    template(
        &root,
        "coding",
        &CODING.replace(
            "worker((\"worker\")):::lead",
            "worker((\"worker\"))\n    class worker lead\n    classDef lead stroke-width:3px",
        ),
    );

    let out = run(&root, &["template", "lint"]);

    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
}

#[test]
fn template_lint_accepts_a_style_class_on_a_role() {
    let (_tmp, root) = root();
    template(
        &root,
        "coding",
        &CODING.replace(
            "advisor((\"advisor\"))\n",
            "advisor((\"advisor\")):::quiet\n",
        ),
    );

    let out = run(&root, &["template", "lint"]);

    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
}

#[test]
fn template_lint_rejects_team_to_team_handoff_off_the_lead() {
    let second = r#"  subgraph docs
    writer(("worker")):::lead
    editor(("advisor"))
    writer ask-edit@--> editor
  end
  worker cross@--> editor
```"#;
    let handoffs = "\n### ask-edit\n\nAsk.\n\n### cross\n\nCross.\n";
    let (_tmp, root) = root();
    template(
        &root,
        "coding",
        &(CODING.replacen(
            "```\n\n## Boundaries",
            &format!("{second}\n\n## Boundaries"),
            1,
        ) + handoffs),
    );

    let out = run(&root, &["template", "lint"]);

    assert_eq!(out.status.code(), Some(1), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("at role `editor`, which is not the lead"),
        "{}",
        stdout(&out)
    );
    assert_eq!(stdout(&out).lines().count(), 1, "{}", stdout(&out));
}

#[test]
fn template_lint_rejects_unnamed_handoff() {
    assert_lint_rejects("worker ask-review@-->", "worker -->", "handoff has no name");
}

#[test]
fn template_lint_rejects_duplicate_handoff_name() {
    assert_lint_rejects(
        "advisor review-result@-->",
        "advisor ask-review@-->",
        "handoff name `ask-review` is used more than once",
    );
}

#[test]
fn template_lint_rejects_handoff_without_section() {
    assert_lint_rejects(
        "### review-result\n\nThe advisor answers with findings.\n\n",
        "",
        "handoff `review-result` has no `### review-result` section",
    );
}

#[test]
fn template_lint_rejects_section_without_handoff() {
    assert_lint_rejects(
        "### external\n",
        "### external\n\nText.\n\n### ghost\n",
        "section `ghost` names no handoff",
    );
}

#[test]
fn template_lint_rejects_empty_handoff_section() {
    assert_lint_rejects(
        "The advisor answers with findings.\n",
        "",
        "section `review-result` is empty",
    );
}

#[test]
fn template_lint_rejects_unknown_role() {
    assert_lint_rejects(
        "advisor((\"advisor\"))",
        "advisor((\"reviewer\"))",
        "role `reviewer` not found",
    );
}

#[test]
fn template_lint_rejects_bad_scope() {
    assert_lint_rejects("scope: branch", "scope: repo", "scope");
}

#[test]
fn template_lint_rejects_unknown_mermaid_line() {
    assert_lint_rejects(
        "  end\n",
        "  end\n  click worker callback\n",
        "unknown line",
    );
}

#[test]
fn template_lint_rejects_empty_boundaries() {
    assert_lint_rejects(
        "- No file changes outside the git branch of the team.\n- No `git merge`.\n",
        "",
        "Boundaries has no `- ` items",
    );
}

#[test]
fn template_lint_rejects_boundaries_with_prose() {
    assert_lint_rejects(
        "- No `git merge`.\n",
        "- No `git merge`.\nBe nice.\n",
        "Boundaries must hold only `- ` list items",
    );
}

#[test]
fn template_lint_unknown_template_exits_1() {
    let (_tmp, root) = root();

    let out = run(&root, &["template", "lint", "nope"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
}

#[test]
fn template_roles_prints_id_role_and_class() {
    let (_tmp, root) = root();
    template(&root, "coding", CODING);

    let out = run(&root, &["template", "roles", "coding"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "worker\tcoding/worker\tlead\t1\t\nadvisor\tcoding/advisor\t-\t1\t\n"
    );
}

#[test]
fn template_roles_names_template_local_role() {
    let (_tmp, root) = root();
    let dir = template(
        &root,
        "coding",
        &CODING.replace("advisor((\"advisor\"))", "advisor((\"reviewer\"))"),
    );
    write(&dir.join("roles/reviewer/index.md"), ROLE_INDEX);

    let roles = run(&root, &["template", "roles", "coding"]);
    let lint = run(&root, &["template", "lint"]);

    assert_eq!(
        stdout(&roles),
        "worker\tcoding/worker\tlead\t1\t\nadvisor\tcoding/reviewer\t-\t1\t\n"
    );
    assert!(lint.status.success(), "{}{}", stdout(&lint), stderr(&lint));
}

#[test]
fn template_lint_accepts_default_templates() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    assert!(run(&root, &["template", "init"]).status.success());

    let out = run(&root, &["template", "lint"]);

    assert!(out.status.success(), "{}{}", stdout(&out), stderr(&out));
}

#[test]
fn template_roles_prints_count_of_each_role() {
    let (_tmp, root) = root();
    let dir = template(&root, "coding", CODING);
    write(
        &dir.join("roles/advisor/index.md"),
        "---\ntype: Role\ndescription: d\ncount: 3\n---\n",
    );

    let out = run(&root, &["template", "roles", "coding"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "worker\tcoding/worker\tlead\t1\t\nadvisor\tcoding/advisor\t-\t3\t\n"
    );
}

#[test]
fn template_lint_rejects_role_with_bad_count() {
    let (_tmp, root) = root();
    let dir = template(&root, "coding", CODING);
    write(
        &dir.join("roles/advisor/index.md"),
        "---\ntype: Role\ndescription: d\ncount: 0\n---\n",
    );

    let out = run(&root, &["template", "lint"]);

    assert_eq!(out.status.code(), Some(1), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("role `advisor` is not valid: count is `0`"),
        "{}",
        stdout(&out)
    );
}

fn hash(cwd: &Path, name: &str) -> String {
    let out = run(cwd, &["template", "hash", name]);
    assert!(out.status.success(), "{}", stderr(&out));
    stdout(&out)
}

#[test]
fn template_hash_is_same_for_same_content() {
    let (_tmp, root) = root();
    let first = template(&root.join("a"), "coding", CODING);
    write(&first.join("roles/worker/index.md"), ROLE_INDEX);
    std::thread::sleep(std::time::Duration::from_millis(20));
    let second = template(&root.join("b"), "coding", CODING);
    write(&second.join("roles/worker/index.md"), ROLE_INDEX);

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
fn template_hash_changes_when_a_file_changes() {
    let (_tmp, root) = root();
    let dir = template(&root, "coding", CODING);
    let before = hash(&root, "coding");

    write(
        &dir.join("roles/worker/index.md"),
        "---\ntype: Role\ndescription: e\n---\n",
    );

    assert_ne!(hash(&root, "coding"), before);
}

#[test]
fn template_hash_unknown_template_exits_1() {
    let (_tmp, root) = root();

    let out = run(&root, &["template", "hash", "nope"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).starts_with("error: "), "{}", stderr(&out));
}

#[test]
fn template_roles_prints_model_preference() {
    let (_tmp, root) = root();
    let dir = template(&root, "coding", CODING);
    write(
        &dir.join("roles/advisor/index.md"),
        "---\ntype: Role\ndescription: d\nmodel: fable\n---\n",
    );

    let out = run(&root, &["template", "roles", "coding"]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(
        stdout(&out),
        "worker\tcoding/worker\tlead\t1\t\nadvisor\tcoding/advisor\t-\t1\tfable\n"
    );
}
