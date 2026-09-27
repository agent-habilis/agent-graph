---
name: graph-node-invite
description: Add one agent to the running pod of this agent. Any member of the pod can do it. It offers one node of the graph to a free peer of the gossip, through a gossip task, and the peer joins the pod as that node. Use when the user says "/graph-node-invite <node> [<peer>]", "invite a <node>", "add a <node> to the pod", or "grow the pod".
allowed-tools: Bash(agent-graph:*), Bash(agent-gossip:*), Bash(hostname:*), Bash(mkdir:*), Bash(cp:*), Bash(tar:*)
---

# graph-node-invite

A pod is one run of a graph in a gossip. `/graph-up` fills each node to its
`count`. After that, any member can add an agent to a node with this skill.
A node can have more agents than its `count`. The new agent is a free peer
of the gossip. It joins the pod that this agent is a member of.

This skill runs in a session that is already in a gossip. It uses the
**Receive loop**, **Decisions**, **Event handling**, and **Task tracking**
sections of the gossip skill that joined the gossip. Obey them here too: keep
the bell armed, and track each task in the todo widget.

Pod management is permitted with any peer of the gossip, also for a node that
has no `external` edge.

## Arguments

- `<node>`: the mermaid id of the node, from `agent-graph graph nodes`.
- `<peer>`: optional. The nickname of the peer to invite.

If there is no `<node>`, print this line, then stop:

```text
graph · usage · /graph-node-invite <node> [<peer>]
```

<!-- include path="../shared/gossip-session.md" -->

<!-- include path="../shared/pod-meta.md" -->

## Gate

1. Read the meta document. Your own entry must make you a member of its
   pod. If it does not, print this line, then stop:

   ```text
   graph · not in a pod · start one with /graph-up <graph>
   ```

   From your entry, hold `pod` as `$INSTANCE`, `graph` as `$GRAPH`, and
   `hash` as `$HASH`. Check `<node>` per the **Pod meta** section.

2. Find the graph folder with `$HASH`. First try the local graph `$GRAPH`:

   ```bash
   agent-graph graph hash "$GRAPH"
   ```

   If it prints `$HASH`, use `$GRAPH` as the graph name below. Else use
   `$HASH` as the graph name, and run the commands from your home folder,
   where `~/.agent-graph/$HASH` is.

   ```bash
   agent-graph graph nodes <graph name>
   ```

   If `<node>` is not an id in the output, print this line, then stop:

   ```text
   graph · no node <node> in $GRAPH
   ```

   If the class of `<node>` is `public`, print this line, then stop. A pod has one public
   node agent, because that node talks to the user:

   ```text
   graph · <node> is public · one per pod
   ```

3. Pick the peer:
   - With `<peer>`: the peer must be free, per the **Pod meta** section. If
     it is not, print `graph · <peer> is not free`, then stop.
   - Without `<peer>`: pick a free peer whose meta `model` contains the
     `model` of the node (case-insensitive). If no such peer is free, pick
     any free peer. If no peer is free, print this line, then stop:

     ```text
     graph · no free peer · $INSTANCE · <node>
     ```

4. Send the offer per the **Node offer** section.

<!-- include path="../shared/node-offer.md" -->

<!-- include path="../shared/offer-drive.md" -->

After a `failed` or `task_timeout`: with `<peer>`, print
`graph · <peer> declined · $INSTANCE · <node>`, then stop. Without `<peer>`,
offer the place to the next free peer, the same way as step 3.

## Done

When the peer's `loaded` artifact is approved:

1. A meta change rings no bell, so tell the pod. Broadcast:

   ```bash
   agent-gossip a2a broadcast --gossip "$GOSSIP" --nickname "$NICKNAME" --text "graph $INSTANCE · <peer> joined as <node>"
   ```

2. Print this line last:

```text
graph · invited · $INSTANCE · <node> <peer>
```
