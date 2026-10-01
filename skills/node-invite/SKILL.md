---
name: node-invite
description: Add one peer to the running graph of this agent. Any peer of the graph can do it. It offers one node of the graph to a free peer of the gossip, through a gossip task, and the peer joins the graph in that node. Use when the user says "/node-invite <node> [<peer>]", "invite a <node>", "add a <node> to the graph", or "grow the graph".
allowed-tools: Bash(agent-graph:*), Bash(agent-gossip:*), Bash(hostname:*), Bash(mkdir:*), Bash(cp:*), Bash(tar:*)
---

# node-invite

A running graph is an instance of a graph in a gossip. `/graph-up` fills each node
to its `count`. After that, any peer can add a peer to a node with this
skill. A node can have more peers than its `count`. The new peer is a
free peer of the gossip. It joins the graph that this agent is a peer of.

This skill runs in a session that is already in a gossip. It uses the
**Receive loop**, **Decisions**, **Event handling**, and **Task tracking**
sections of the gossip skill that joined the gossip. Obey them here too: keep
the bell armed, and track each task in the todo widget.

Graph management is permitted with any peer of the gossip, also for a node
that has no `external` edge.

## Arguments

- `<node>`: the mermaid id of the node, from `agent-graph graph nodes`.
- `<peer>`: optional. The nickname of the peer to invite.

If there is no `<node>`, print this line, then stop:

```text
graph · usage · /node-invite <node> [<peer>]
```

<!-- include path="../shared/gossip-session.md" -->

<!-- include path="../shared/graph-meta.md" -->

## Gate

1. Read the meta document. Your own entry must make you a peer of its
   graph. If it does not, print this line, then stop:

   ```text
   graph · not in a graph · start one with /graph-up <graph>
   ```

   From your entry, hold `instance` as `$INSTANCE`, `graph` as `$GRAPH`,
   and `hash` as `$HASH`. Check `<node>` per the **Graph meta** section.

2. Find the graph folder with `$HASH`. First try the local graph
   `$GRAPH`:

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

   If the third column of `<node>` is `lead`, print this line, then stop. A
   graph has one lead peer, because the lead talks to the user:

   ```text
   graph · <node> is the lead · one per graph
   ```

3. Pick the peer:
   - With `<peer>`: the peer must be free, per the **Graph meta** section. If
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

1. A meta change rings no bell, so tell the graph. Broadcast:

   ```bash
   agent-gossip a2a broadcast --gossip "$GOSSIP" --nickname "$NICKNAME" --text "graph $INSTANCE · <peer> joined as <node>"
   ```

2. Print this line last:

```text
graph · invited · $INSTANCE · <node> <peer>
```
