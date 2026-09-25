---
name: graph-list
description: List the agent graphs that the current project can see, with the `agent-graph` CLI. Graphs are OKF bundles in `.agent-graph/` folders from the cwd up to `/`. Use when the user says "/graph-list", "list graphs", or "what graphs are there".
allowed-tools: Bash(agent-graph:*)
---

# graph-list

`agent-graph` finds graphs in `.agent-graph/` in the current directory and in
each parent directory, up to `/`. If two directories have a graph with the
same name, the graph nearer to the current directory wins.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

## List

```sh
agent-graph graph list
```

Each output line is `name<TAB>description<TAB>scope<TAB>path`. Show the
graphs as a Markdown table with the columns name, description, and scope. Do
not show the path unless the user asks for it.

If the output is empty, print:

```text
graph · none · no .agent-graph/ folder from the cwd up to /
```

The CLI writes a `warning:` line to stderr for each graph that is not valid.
Show each warning to the user after the table.
