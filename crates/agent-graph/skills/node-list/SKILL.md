---
name: node-list
description: List the nodes of the graphs that the current project can see, with the `agent-graph` CLI. Graphs are OKF bundles in `.agent-graph/` folders from the cwd up to `/`, and each graph holds its nodes in `<graph>/nodes/`. Use when the user says "/node-list", "list nodes", or "what nodes are there".
allowed-tools: Bash(agent-graph:*)
---

# node-list

`agent-graph` finds graphs in `.agent-graph/` in the current
directory and in each parent directory, up to `/`. Each graph holds its
nodes in `<graph>/nodes/`. A node has the name `<graph>/<node>`. If two
directories have a graph with the same name, the graph nearer to the
current directory wins, with all its nodes.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

## Arguments

An optional tag regex. With no argument, list all nodes.

## List

```sh
agent-graph node list                 # all nodes
agent-graph node list --tag '<regex>' # only nodes with a tag that matches
```

Each output line is `name<TAB>description<TAB>tags<TAB>path`. Show the nodes
as a Markdown table with the columns name, description, and tags. Do not show
the path unless the user asks for it.

If the output is empty, print:

```text
node · none · no nodes in .agent-graph/ from the cwd up to /
```

The CLI writes a `warning:` line to stderr for each node that is not valid.
Show each warning to the user after the table.
