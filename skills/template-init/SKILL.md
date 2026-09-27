---
name: template-init
description: Create a starter `.agent-graph/` folder in the current project, with the default team template and its orchestrator, worker, and advisor roles. Does nothing if the folder exists. Use when the user says "/template-init", "init template", or "set up agent teams for this project".
allowed-tools: Bash(git rev-parse:*), Bash(agent-graph:*)
---

# template-init

Create `.agent-graph/` in the current project with
`agent-graph template init`.

The defaults are:

- `default`: 1 orchestrator (lead), 2 workers, and 1 advisor, per git branch.
  - `default/orchestrator`: divides the task and manages the work (model opus).
  - `default/worker`: writes the code for one part (count 2, model sonnet).
  - `default/advisor`: reviews the plan and the diff (model fable).

## Target

The target is the root of the git repository. If the cwd is not in a git
repository, the target is the cwd:

```sh
git rev-parse --show-toplevel 2>/dev/null || pwd
```

Hold the output as `$TARGET`.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

## Init

```sh
agent-graph template init "$TARGET"
```

The binary holds the default files. It prints each file that it writes. If
`$TARGET/.agent-graph` exists, as a folder or a file, it writes nothing and
prints `… already exists` to stderr. It exits 0 in both cases.

If the output says `already exists`, print this line, then stop:

```text
template · exists · $TARGET/.agent-graph
```

Else show the new roles:

```sh
cd "$TARGET" && agent-graph role list
```

Show the roles as a Markdown table with the columns name and description.
Print this line last:

```text
template · created · $TARGET/.agent-graph
```
