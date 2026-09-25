---
name: graph-init
description: Create a starter `.agent-graph/` folder in the current project, with the dev-team graph and its worker, advisor, and qa nodes. Does nothing if the folder exists. Use when the user says "/graph-init", "init graph", or "set up agent graphs for this project".
allowed-tools: Bash(git rev-parse:*), Bash(agent-graph:*)
---

# graph-init

Create `.agent-graph/` in the current project with `agent-graph graph init`.

The defaults are:

- `dev-team`: 1 worker (public), 1 advisor, and 1 qa, per git branch.
  - `dev-team/worker`: writes the code on one branch.
  - `dev-team/advisor`: reviews the work of the worker.
  - `dev-team/qa`: runs the tests and tries the change as a user.

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
agent-graph graph init "$TARGET"
```

The binary holds the default files. It prints each file that it writes. If
`$TARGET/.agent-graph` exists, as a folder or a file, it writes nothing and
prints `… already exists` to stderr. It exits 0 in both cases.

If the output says `already exists`, print this line, then stop:

```text
graph · exists · $TARGET/.agent-graph
```

Else show the new nodes:

```sh
cd "$TARGET" && agent-graph node list
```

Show the nodes as a Markdown table with the columns name and description.
Print this line last:

```text
graph · created · $TARGET/.agent-graph
```
