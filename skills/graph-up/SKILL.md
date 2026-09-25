---
name: graph-up
description: Start an agent graph in the current gossip. This agent is the bootstrapper. It takes one node of the graph, then offers each open place to one peer that ran `/graph-join`, through gossip tasks. A node with `count` n needs n agents. Done when every node has its count. Use when the user says "/graph-up <graph> [<node>]", "graph up", or "start the <graph> graph".
allowed-tools: Bash(agent-graph:*), Bash(agent-gossip:*), Bash(git rev-parse:*), Bash(git branch:*), Bash(basename:*)
---

# graph-up

A graph is a team of agents. Each node of the graph needs `count` agents (1
by default). All agents of one node have the same node name. Their gossip
nicknames tell them apart. This agent is the **bootstrapper**. It is a member
of the graph too. It takes its own node first, then offers each open place to
one free peer of the gossip. `up` is done when every node has `count` peers.

This skill runs in a session that is already in a gossip. It uses the
**Receive loop**, **Decisions**, and **Event handling** sections of the gossip
skill that joined the gossip. Obey them here too: keep the bell armed, and
track each task in the todo widget.

## Arguments

- `<graph>`: the graph name, from `agent-graph graph list`.
- `<node>`: optional. The mermaid id of the node that this agent takes. With
  no `<node>`, take the first node that `agent-graph graph nodes <graph>`
  prints.

If there is no `<graph>`, print this line, then stop:

```text
graph · usage · /graph-up <graph> [<node>]
```

## Gate

1. Find the gossip of this session:

   ```bash
   agent-gossip session --session-pid "$PPID"
   ```

   Hold `$GOSSIP` and `$NICKNAME` from the output. If there is no session,
   print `graph · no gossip · create or join a gossip first`, then stop.

2. Read and check the graph:

   ```bash
   agent-graph graph lint <graph>
   agent-graph graph list
   agent-graph graph nodes <graph>
   ```

   If `lint` fails, show its lines, then stop. From `graph list`, hold the
   `scope` of the graph. Each line of `graph nodes` is
   `id<TAB><graph>/<node><TAB>public|private<TAB>count`. Hold the list as
   `$NODES`.

3. Hold the instance name as `$INSTANCE`:
   - `scope: branch` → `<graph>@<branch>`, with `git branch --show-current`.
   - `scope: project` → `<graph>@<repo>`, with
     `basename "$(git rev-parse --show-toplevel)"`.

4. Read the state document:

   ```bash
   agent-gossip state get --gossip "$GOSSIP" --nickname "$NICKNAME"
   ```

   If `/graphs/$INSTANCE` exists and each node has `count` peers, print the
   **Done** line, then stop. A peer that is in a `peers` list keeps its
   place.

## Take your own node

1. Load your node. This prints the graph and the node into this session, and
   records them for the statusline:

   ```bash
   agent-graph node up <graph>/<node> --pid "$PPID"
   ```

2. Write the nodes and your own place into the state document, in one merge.
   Each node has its `node` name, its `count`, and a `peers` list. Your own
   node has `["$NICKNAME"]`. Every other node has `[]`:

   ```bash
   agent-gossip state merge --gossip "$GOSSIP" --nickname "$NICKNAME" --merge '{"graphs":{"'"$INSTANCE"'":{"bootstrapper":"'"$NICKNAME"'","nodes":{"<id>":{"node":"<graph>/<node>","count":<count>,"peers":["'"$NICKNAME"'"]}, …}}}}'
   ```

3. Record your node in the meta channel:

   ```bash
   agent-gossip meta merge --gossip "$GOSSIP" --nickname "$NICKNAME" --merge '{"peers":{"'"$NICKNAME"'":{"graph":"'"$INSTANCE"'","node":"<id>","status":"busy"}}}'
   ```

From now on, act as your node, and obey its boundaries and the boundaries of
the graph.

## Offer the open places

A node has `count` minus the length of `peers` open places. Read the meta
channel to find the peers:

```bash
agent-gossip meta get --gossip "$GOSSIP" --nickname "$NICKNAME"
```

A peer is **free** if its entry has `"graphs": "open"` (the peer ran
`/graph-join`), it has no `node`, and it has no open offer from you. For each
open place, pick one free peer, and send a node offer. The label must start
with `graph · `, and the brief must have exactly this form, because
`/graph-join` reads both:

```bash
agent-gossip a2a call --gossip "$GOSSIP" --nickname "$NICKNAME" --to "$PEER" --method SendMessage --label "graph · $INSTANCE · <id>" --text "Node offer. Graph: $INSTANCE. Id: <id>. Node: <graph>/<node>."
```

Track each offer as a task, per the **Task tracking** rules.

## Drive

Handle each event per the **Receive loop** and **Event handling** sections,
with these rules for node offers:

- **Artifact `loaded <graph>/<node>`:** approve it with a follow-up that
  carries `--task-id`. Then add the peer to the `peers` list of the node. A
  merge replaces a list, so write the full list:

  ```bash
  agent-gossip state merge --gossip "$GOSSIP" --nickname "$NICKNAME" --merge '{"graphs":{"'"$INSTANCE"'":{"nodes":{"<id>":{"peers":["<peer>", …]}}}}}'
  ```

- **`failed` or `task_timeout`:** the place is open again. Offer it to the
  next free peer.
- **No free peer:** print this line. Tell the user that each other agent
  runs `/graph-join`. On each later batch, read meta again, and offer the
  open places to the new free peers:

  ```text
  graph · waiting · $INSTANCE · <n> places open
  ```

## Done

When each node of `/graphs/$INSTANCE` has `count` peers:

1. Broadcast the roster, so that each node can find the peers at the other
   end of its edges:

   ```bash
   agent-gossip a2a broadcast --gossip "$GOSSIP" --nickname "$NICKNAME" --text "graph $INSTANCE is up: <id> <peer> <peer>, <id> <peer>, …"
   ```

2. Print this line last:

   ```text
   graph · up · $INSTANCE · <id> <peer> <peer> · <id> <peer> · …
   ```
