---
name: graph-node-up
description: Load one standalone node into this session with the `agent-graph` CLI, so that the agent acts as that node and obeys its boundaries. A standalone node lives in `.agent-graph/nodes/`, outside of any graph. Use when the user says "/graph-node-up <node>", "load the <node> node", or "act as the <node> node".
allowed-tools: Bash(agent-graph:*)
---

# graph-node-up

A node is one position that an agent can take. Its text changes how the
agent behaves. A standalone node lives in `.agent-graph/nodes/<node>/`,
outside of any graph. It is self-contained: its text does not point at other
nodes. `agent-graph node up <node>` prints its body, with its `@file`
includes expanded.

This skill loads only standalone nodes. A node of a graph, `<graph>/<node>`,
joins a running graph through `/graph-up` or `/graph-node-invite`.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

## Arguments

The node name, `<node>`. If there is no name, print:

```text
node · usage · /graph-node-up <node>
```

Then stop. To see the names, the user runs `/graph-node-list`.

If the name has a `/`, it is a node of a graph. Print this line, then stop:

```text
node · <name> is a node of a graph · use /graph-up or /graph-node-invite
```

## Up

```sh
agent-graph node up <node> --pid "$PPID"
```

Run it in the foreground. The tool result puts the body of the node into
this session. `$PPID` is the agent process. The CLI records the node for the
statusline under this pid. If the command fails, show its `error:` line and
stop.

When the command succeeds:

1. Act as this node until the user loads another node or clears the
   session.
2. Obey each item in each `## Boundaries` section of the node. A boundary is
   a rule that you must not break. If a request from the user breaks a
   boundary, say which boundary it breaks, and ask before you continue.
3. Print only this line. Do not summarize the node:

```text
node · loaded · <node>
```
