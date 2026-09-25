# agent-role

A role is a fragment of text that you put into the context of an agent. The role changes how the agent behaves.
`agent-role` finds roles on disk and prints them.

## Layout

```
.agent-role/
  roles/
    advisor/
      index.md      # entry point
      tone.md       # printed only if index.md includes it
```

Each role folder is an [OKF](https://cloud.google.com/blog/products/data-analytics/how-the-open-knowledge-format-can-improve-data-sharing) bundle.
The folder name is the role name.

`index.md` starts with this frontmatter:

```yaml
---
type: Role                  # required
description: Challenge plans and point out risks.   # required, one line
title: Advisor              # optional
tags: [review, go]          # optional, inline list only
---
```

## Discovery

`agent-role` reads `.agent-role/roles/` in the current directory and in each parent directory, up to `/`.
If two directories have a role with the same name, the role nearer to the current directory wins.

## Commands

```sh
agent-role list                # name<TAB>description<TAB>tags<TAB>path
agent-role list --tag '^go$'   # only roles with a tag that matches the regex
agent-role get advisor         # the body of index.md, with includes expanded
```

If a role is not valid, `list` writes a warning to stderr and skips the role.
The `--tag` regex is not anchored, so `go` also matches `mongo`.

## Includes

In `index.md`, a line that contains only `@<path>` is replaced with the body of that file, without its frontmatter.

- The path is relative to the file that contains the include.
- Included files can include other files. A cycle is an error.
- The path must stay inside the role folder.
- The CLI ignores `@` lines inside fenced code blocks.

## Exit codes

- `0`: success.
- `1`: error. For example, the role is not found, an include is not valid, or the regex is not valid.

## Development

All development commands go through `cargo task <name>`.
`cargo task` with no argument lists the tasks.

| Task | What it does |
|---|---|
| `test` | `cargo test --workspace --no-fail-fast` |
| `ci` | The full gate: `fmt --check`, clippy, tests |
| `lint` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `fmt` | `cargo fmt --all` |
| `coverage` | `cargo llvm-cov`, installed on demand |
| `run` | `cargo run --` with the rest forwarded (`cargo task run list`) |
| `install` | `cargo install --force --locked` from the repo root |
| `release` | Builds the release binary |
| `clean` | `cargo clean` plus the llvm-cov target dir |
