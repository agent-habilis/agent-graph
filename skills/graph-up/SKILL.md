---
name: graph-up
description: Start an agent graph as a pod in the current gossip. This agent is the bootstrapper. It takes one node of the graph, then offers each open place to one free peer of the gossip, through gossip tasks. A node with `count` n needs n agents at the start. The pod lives in the gossip meta. Done when every node has its count. Use when the user says "/graph-up <graph> [<node>] [--pod <name>]", "graph up", or "start the <graph> graph".
allowed-tools: Bash(agent-graph:*), Bash(agent-gossip:*), Bash(git rev-parse:*), Bash(git branch:*), Bash(basename:*), Bash(hostname:*), Bash(mkdir:*), Bash(cp:*), Bash(tar:*)
---

# graph-up

A graph is a team of agents. A pod is one run of a graph in a gossip. Each
node of the graph needs `count` agents (1 by default) at the start. Later, a
member can add more with `/graph-node-invite`. All agents of one node have
the same node name. Their gossip nicknames tell them apart. This agent is the
**bootstrapper**. It is a member of the pod too. It takes its own node first,
then offers each open place to one free peer of the gossip. `up` is done when
every node has `count` filled places.

This skill runs in a session that is already in a gossip. It uses the
**Receive loop**, **Decisions**, **Event handling**, and **Task tracking**
sections of the gossip skill that joined the gossip. Obey them here too: keep
the bell armed, and track each task in the todo widget.

## Arguments

- `<graph>`: the graph name, from `agent-graph graph list`.
- `<node>`: optional. The mermaid id of the node that this agent takes. With
  no `<node>`, take the first node that `agent-graph graph nodes <graph>`
  prints.
- `--pod <name>`: optional. The instance name of the pod.

If there is no `<graph>`, print this line, then stop:

```text
graph · usage · /graph-up <graph> [<node>] [--pod <name>]
```

<!-- include path="../shared/gossip-session.md" -->

<!-- include path="../shared/pod-meta.md" -->

## Gate

1. Read and check the graph:

   ```bash
   agent-graph graph lint <graph>
   agent-graph graph list
   agent-graph graph nodes <graph>
   agent-graph graph hash <graph>
   ```

   If `lint` fails, show its lines, then stop. From `graph list`, hold the
   `scope` of the graph. Hold the lines of `graph nodes` as `$NODES`. `model`
   is the model that the node prefers, or empty. Hold `<graph>` as `$GRAPH`
   and the hash as `$HASH`. The hash is the id of the graph between peers.

2. Hold the instance name as `$INSTANCE`:
   - `--pod <name>` → `<name>`.
   - `scope: branch` → `<graph>@<branch>`, with `git branch --show-current`.
   - `scope: project` → `<graph>@<repo>`, with
     `basename "$(git rev-parse --show-toplevel)"`.

   Check `$INSTANCE`, `$GRAPH`, and each node id per the **Pod meta**
   section.

3. Read the meta document, and look at your own entry:
   - It has another `pod`: print this line, then stop.

     ```text
     graph · in pod <pod> · run /graph-node-down first
     ```

   - It has `pod` `$INSTANCE`: this is a resume. Take `$GRAPH` and `$HASH`
     from your entry, not from the local graph, so that new offers carry
     the hash of the pod. Go to **Offer the open places**.

4. If any live entry (not in `absent`) other than yours has `pod`
   `$INSTANCE`, with any hash and with or without a `node`, print this line,
   then stop:

   ```text
   graph · pod exists · $INSTANCE · use /graph-node-invite from a member
   ```

## Take your own node

1. Load your node. This prints the graph and the node into this session, and
   records them for the statusline:

   ```bash
   agent-graph node up <graph>/<node> --pid "$PPID"
   ```

2. Record your place in your meta entry:

   ```bash
   agent-gossip meta merge --gossip "$GOSSIP" --nickname "$NICKNAME" --merge '{"peers":{"'"$NICKNAME"'":{"pod":"'"$INSTANCE"'","graph":"'"$GRAPH"'","hash":"'"$HASH"'","node":"<id>","status":"busy"}}}'
   ```

3. Do the **Second starter check**.

4. From now on, act as your node, and obey its boundaries and the boundaries
   of the graph.

## Second starter check

Another peer can start the same pod at the same time, and meta arrives with a
delay. Do this check after your own merge, and again on each later batch,
before each new offer.

Read the meta document. If another live entry has `pod` `$INSTANCE` and no
`invited_by`, the smaller nickname keeps the pod. If that is the other peer:

1. Cancel each open offer of yours with `--method CancelTask`.
2. Remove your place:

   ```bash
   agent-gossip meta merge --gossip "$GOSSIP" --nickname "$NICKNAME" --merge '{"peers":{"'"$NICKNAME"'":{"pod":null,"graph":null,"hash":null,"node":null,"status":"idle"}}}'
   agent-graph node down --pid "$PPID"
   ```

3. Print this line, then stop:

   ```text
   graph · pod taken · $INSTANCE · by <nickname>
   ```

## Offer the open places

Do the **Second starter check** first. A node has `count` minus its filled
places open places, per the **Pod meta** section. For each open place, pick one free peer. If the node has a `model`,
pick first a free peer whose meta `model` contains it (case-insensitive, so
`fable` matches `claude-fable-5-1`). If no such peer is free, pick any free
peer. Send the offer per the **Node offer** section.

<!-- include path="../shared/node-offer.md" -->

<!-- include path="../shared/offer-drive.md" -->

After a `failed` or `task_timeout`, offer the place to the next free peer.
When no peer is free, print this line. On each later batch, read meta again,
and offer the open places to new free peers, for example a peer that just
joined the gossip:

```text
graph · waiting · $INSTANCE · <n> places open
```

## Done

When each node of `$INSTANCE` has `count` filled places, and no offer is
open:

1. Broadcast the roster, so that each node can find the peers at the other
   end of its edges:

   ```bash
   agent-gossip a2a broadcast --gossip "$GOSSIP" --nickname "$NICKNAME" --text "graph $INSTANCE is up: <id> <peer> <peer>, <id> <peer>, …"
   ```

2. Print this line last:

   ```text
   graph · up · $INSTANCE · <id> <peer> <peer> · <id> <peer> · …
   ```
