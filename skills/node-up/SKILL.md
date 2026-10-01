---
name: node-up
description: Load one node of a graph into this session with the `agent-graph` CLI, so that the agent acts as that node and obeys its boundaries and the boundaries of its graph. Use when the user says "/node-up <graph>/<node>", "load the <node> node", or "act as the <node> node".
allowed-tools: Bash(agent-graph:*)
---

# node-up

A graph is a team of agents. A node is one position in a
graph. Its text changes how an agent behaves.
`agent-graph node up <graph>/<node>` prints the body of the graph
first, then the body of the node, with their `@file` includes expanded.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

## Arguments

The node name, as `<graph>/<node>`. If there is no name, print:

```text
node · usage · /node-up <graph>/<node>
```

Then stop. To see the names, the user runs `/node-list`.

## Up

```sh
agent-graph node up <graph>/<node> --pid "$PPID"
```

Run it in the foreground. The tool result puts the body of the graph and
of the node into this session. `$PPID` is the agent process. The CLI records
the graph and the node for the statusline under this pid. If the command
fails, show its `error:` line and stop.

When the command succeeds:

1. Act as this node, as a peer of its graph, until the user loads another
   node or clears the session. If a gossip broadcast says
   `graph <instance> is down` for the instance in your meta entry, run
   `/node-down`.
2. Obey each item in each `## Boundaries` section of the node and of its
   graph. A boundary is a rule that you must not break. If a request from
   the user breaks a boundary, say which boundary it breaks, and ask before
   you continue.
3. Print only these lines. Do not summarize the node:

```text
graph · loaded · <graph>
node · loaded · <node>
```
