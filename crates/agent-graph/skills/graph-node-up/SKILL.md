---
name: graph-node-up
description: Load one standalone node into this session with the `agent-graph` CLI, so that the agent acts as that node and obeys its boundaries. A standalone node lives in `.agent-graph/nodes/`, outside of any graph. Use when the user says "/graph-node-up --node-template <node> [--name <name>]", "load the <node> node", or "act as the <node> node".
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

- `--node-template <node>`: the standalone node template, from
  `/graph-node-list`.
- `--name <name>`: optional. The name of this agent as the node, shown in
  the statusline. The default is a short hash, like a git short hash.

If there is no `--node-template`, print:

```text
node · usage · /graph-node-up --node-template <node> [--name <name>]
```

Then stop. To see the names, the user runs `/graph-node-list`.

If the name has a `/`, it is a node of a graph. Print this line, then stop:

```text
node · <name> is a node of a graph · use /graph-up or /graph-node-invite
```

## Up

Hold the name as `$NAME`: the `--name` value, or else the output of
`agent-graph name`. Then run:

```sh
<!-- include path="../shared/pid-flag.md" command="agent-graph node up <node> --name \"$NAME\"" -->
```

Run it in the foreground. The tool result puts the body of the node into
<!-- include path="../shared/agent-pid.md" before="this session. " after=" The CLI records the node for the" -->
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
node · loaded · <name> (<node>)
```
