---
name: graph-leave
description: Leave the node of this agent in its running graph. The agent stops acting as the node and as a peer of the graph, clears its gossip meta entry, and the pills leave the statusline. The graph keeps running with the other peers. Use when the user says "/graph-leave", "leave the graph", or "drop out of the graph".
allowed-tools: Bash(agent-graph:*), Bash(agent-gossip:*)
---

# graph-leave

A running graph is an instance of a graph in a gossip. `/graph-up` and
`/node-invite` add a peer to a node of it. This skill takes this agent out of
its node. The other peers stay. To stop the whole graph, the user runs
`/graph-down`.

<!-- include path="../shared/gossip-session.md" -->

## Leave

```sh
agent-graph graph node-down --pid "$PPID"
```

`$PPID` is the agent process. The command removes the record for the
statusline, and it also succeeds when this agent has no node. If the command
fails, show its `error:` line and stop.

Read your meta entry:

```bash
agent-gossip meta get --gossip "$GOSSIP" --nickname "$NICKNAME"
```

If your entry `/peers/$NICKNAME` has an `instance`, remove your place. Then
the graph does not count you, and you are a free peer again. If it has no
`instance`, write nothing:

```bash
agent-gossip meta merge --gossip "$GOSSIP" --nickname "$NICKNAME" --merge '{"peers":{"'"$NICKNAME"'":{"instance":null,"graph":null,"hash":null,"node":null,"joined_at":null,"invited_by":null,"verified":null,"status":"idle"}}}'
```

Then:

1. Stop acting as the node and as a peer of its graph. The boundaries of
   the node and of its graph do not apply any more.
2. Print only this line:

```text
graph · left · <instance>
```
