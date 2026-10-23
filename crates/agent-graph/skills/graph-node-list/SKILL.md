---
name: graph-node-list
description: List the standalone nodes that the current project can see, with the `agent-graph` CLI. Standalone nodes are OKF bundles in `.agent-graph/nodes/` folders from the cwd up to `/`. Nodes of a graph are not listed. Use when the user says "/graph-node-list", "list nodes", or "what nodes are there".
allowed-tools: Bash(agent-graph:*)
---

# graph-node-list

A standalone node lives in `.agent-graph/nodes/<node>/`, outside of any
graph. It is self-contained: its text does not point at other nodes. Its name
is the bare folder name, for example `worker`. `agent-graph` finds standalone
nodes in `.agent-graph/nodes/` in the current directory and in each parent
directory, up to `/`. If two directories have a node with the same name, the
node nearer to the current directory wins.

This skill lists only standalone nodes. The nodes of a graph have the name
`<graph>/<node>`. To see them, the user runs
`agent-graph graph nodes <graph>`.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

## Arguments

An optional tag regex. With no argument, list all standalone nodes.

## List

```sh
agent-graph node list                 # all standalone nodes
agent-graph node list --tag '<regex>' # only nodes with a tag that matches
```

Each output line is `name<TAB>description<TAB>tags<TAB>path`. Show the nodes
as a Markdown table with the columns name, description, and tags. Do not show
the path unless the user asks for it.

If the output is empty, print:

```text
node · none · no nodes in .agent-graph/nodes/ from the cwd up to /
```

The CLI writes a `warning:` line to stderr for each node that is not valid.
Show each warning to the user after the table.
