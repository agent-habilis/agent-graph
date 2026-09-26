---
name: graph-node-down
description: Unset the node of this agent with the `agent-graph` CLI. The agent stops acting as the node, and the node pill leaves the statusline. Use when the user says "/graph-node-down", "unset the node", "drop the node", or "stop acting as the node".
allowed-tools: Bash(agent-graph:*)
---

# graph-node-down

`agent-graph node down --pid <pid>` removes the record that
`agent-graph node up` wrote for the statusline. The text of the node stays in
the context of this session, so this skill also tells the agent to stop
acting on it.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

## Down

```sh
agent-graph node down --pid "$PPID"
```

`$PPID` is the agent process. The command also succeeds when this agent has
no node. If the command fails, show its `error:` line and stop.

When the command succeeds:

1. Stop acting as the node and as a member of its graph. The boundaries of
   the node and of its graph do not apply any more.
2. Print only this line:

```text
node · down
```
