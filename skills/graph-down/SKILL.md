---
name: graph-down
description: Stop the running graph of this agent. Any peer of the graph can do it. It tells every peer of the graph to leave its node, through one gossip broadcast, then leaves its own node. Use when the user says "/graph-down [<instance>]", "graph down", "stop the graph", or "tear down the graph".
allowed-tools: Bash(agent-graph:*), Bash(agent-gossip:*)
---

# graph-down

A running graph is an instance of a graph in a gossip. `/graph-up` starts
it, and `/node-down` takes one peer out of it. This skill takes every peer
out of it. A peer writes only its own meta entry, so this agent cannot remove
the places of the other peers. It broadcasts the end of the graph, and each
peer leaves its node by itself.

This skill runs in a session that is already in a gossip. It uses the
**Receive loop** and **Event handling** sections of the gossip skill that
joined the gossip. Obey them here too: keep the bell armed.

## Arguments

`<instance>`: optional. The instance name of the graph, for example
`squad@main`. With no argument, take down the graph of this agent.

<!-- include path="../shared/gossip-session.md" -->

<!-- include path="../shared/graph-meta.md" -->

## Gate

Read the meta document. Hold `$INSTANCE`:

- With `<instance>`: `<instance>`. Check it per the **Graph meta** section.
- Without it: the `instance` of your own entry. If your entry has no
  `instance`, print this line, then stop:

  ```text
  graph · not in a graph · give the <instance>
  ```

Hold as `$PEERS` the nicknames of the live entries (not in `absent`) that
have `instance` `$INSTANCE`, other than yours. If `$PEERS` is empty and your
entry does not have `instance` `$INSTANCE`, print this line, then stop:

```text
graph · no graph · $INSTANCE
```

## Down

1. Cancel each open node offer of yours for `$INSTANCE` with
   `--method CancelTask`.

2. If `$PEERS` is not empty, tell them. One broadcast reaches each peer:

   ```bash
   agent-gossip a2a broadcast --gossip "$GOSSIP" --nickname "$NICKNAME" --text "graph $INSTANCE is down · each peer of $INSTANCE: run /node-down. Without the /node-down skill, run agent-graph node down --pid \"\$PPID\" if agent-graph is on your PATH, then clear your meta entry: agent-gossip meta merge --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --merge '{\"peers\":{\"<your nickname>\":{\"instance\":null,\"graph\":null,\"hash\":null,\"node\":null,\"invited_by\":null,\"verified\":null,\"status\":\"idle\"}}}'. Then stop acting as your node."
   ```

3. If your own entry has `instance` `$INSTANCE`, leave your node:

   ```bash
   agent-graph node down --pid "$PPID"
   agent-gossip meta merge --gossip "$GOSSIP" --nickname "$NICKNAME" --merge '{"peers":{"'"$NICKNAME"'":{"instance":null,"graph":null,"hash":null,"node":null,"invited_by":null,"verified":null,"status":"idle"}}}'
   ```

   Then stop acting as the node and as a peer of its graph. The boundaries
   of the node and of its graph do not apply any more.

4. Print this line last:

   ```text
   graph · down · $INSTANCE · <n> peers told to leave
   ```

The meta entries of the other peers clear as each peer runs `/node-down`.
To check, run `/graph-topology $INSTANCE`: when no peer is left, it fails
with `has no peers`.
