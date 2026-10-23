---
name: graph-init
description: Create a starter `.agent-graph/` folder in the current project, with the squad1 and squad2 graphs (leader, workers, advisor), the pair graph (worker, advisor), the standalone worker and advisor nodes, and the qa, repro, perf, and review traits. Does nothing if the folder exists. Use when the user says "/graph-init", "init graph", or "set up agent graphs for this project".
allowed-tools: Bash(git rev-parse:*), Bash(agent-graph:*)
---

# graph-init

Create `.agent-graph/` in the current project with
`agent-graph graph init`.

The defaults are:

- `squad1`: 1 leader, 2 workers, and 1 advisor, per git branch.
  - `squad1/leader`: divides the task and manages the work (model opus).
  - `squad1/worker`: writes the code for one part (count 2, model sonnet).
  - `squad1/advisor`: reviews the plan and the diff (model fable).
- `squad2`: the same nodes as `squad1`, with other models.
  - `squad2/leader`: divides the task and manages the work (model sonnet).
  - `squad2/worker`: writes the code for one part (count 2, model haiku).
  - `squad2/advisor`: reviews the plan and the diff (model opus).
- `pair`: 1 worker (lead) and 1 advisor, per git branch.
  - `pair/worker`: writes the code, gets a review, and talks to the user (model sonnet).
  - `pair/advisor`: reviews the plan and the diff (model opus).
- Standalone nodes, outside of any graph:
  - `worker`: writes the code for a task and reports the result (model sonnet).
  - `advisor`: reviews a plan or a diff (model fable).
- Traits:
  - `qa`: runs the checks of the project and reports each result (workers).
  - `repro`: writes a test that fails because of a bug (workers, writes).
  - `perf`: measures the change against the base commit (workers).
  - `review`: reviews a plan or a diff (advisors).

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

Else show the new graphs and standalone nodes:

```sh
cd "$TARGET" && agent-graph graph list && agent-graph node list
```

Show the nodes as a Markdown table with the columns name and description.
Print this line last:

```text
graph · created · $TARGET/.agent-graph
```
