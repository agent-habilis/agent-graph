---
name: roles-pod-up
description: Start a pod in the current gossip. This agent is the bootstrapper. It takes one node of the pod, then gives each other node to one peer that ran `/roles-pod-join`, through gossip tasks. Done when every node has an agent. Use when the user says "/roles-pod-up <pod> [<node>]", "pod up", or "start the <pod> pod".
allowed-tools: Bash(agent-role:*), Bash(agent-gossip:*), Bash(git rev-parse:*), Bash(git branch:*), Bash(basename:*)
---

# roles-pod-up

A pod is a group of nodes, and each node has a role. This agent is the
**bootstrapper**. It is a node of the pod too. It takes its own node first,
then offers each other node to one idle peer of the gossip. `up` is done when
every node has an agent.

This skill runs in a session that is already in a gossip. It uses the
**Receive loop**, **Decisions**, and **Event handling** sections of the gossip
skill that joined the gossip. Obey them here too: keep the bell armed, and
track each task in the todo widget.

## Arguments

- `<pod>`: the pod name, from `agent-role pod list`.
- `<node>`: optional. The node that this agent takes. With no `<node>`, take
  the first node that `agent-role pod nodes <pod>` prints.

If there is no `<pod>`, print this line, then stop:

```text
pod · usage · /roles-pod-up <pod> [<node>]
```

## Gate

1. Find the gossip of this session:

   ```bash
   agent-gossip session --session-pid "$PPID"
   ```

   Hold `$GOSSIP` and `$NICKNAME` from the output. If there is no session,
   print `pod · no gossip · create or join a gossip first`, then stop.

2. Read and check the pod:

   ```bash
   agent-role pod lint <pod>
   agent-role pod list
   agent-role pod nodes <pod>
   ```

   If `lint` fails, show its lines, then stop. From `pod list`, hold the
   `scope` of the pod. Each line of `pod nodes` is
   `node<TAB>role<TAB>public|private`. Hold the list as `$NODES`.

3. Hold the instance name as `$INSTANCE`:
   - `scope: branch` → `<pod>@<branch>`, with `git branch --show-current`.
   - `scope: project` → `<pod>@<repo>`, with
     `basename "$(git rev-parse --show-toplevel)"`.

4. Read the state document:

   ```bash
   agent-gossip state get --gossip "$GOSSIP" --nickname "$NICKNAME"
   ```

   If `/pods/$INSTANCE` exists and every node has a `peer`, print the
   **Done** line, then stop. A node that has a `peer` keeps it.

## Take your own node

1. Load the role of your node. This prints the pod and the role into this
   session, and records them for the statusline:

   ```bash
   agent-role get <role> --pid "$PPID"
   ```

2. Write the nodes and your own assignment into the state document, in one
   merge. Every other node has `"peer": null`:

   ```bash
   agent-gossip state merge --gossip "$GOSSIP" --nickname "$NICKNAME" --merge '{"pods":{"'"$INSTANCE"'":{"bootstrapper":"'"$NICKNAME"'","nodes":{"<node>":{"role":"<role>","peer":"'"$NICKNAME"'"}, …}}}}'
   ```

3. Record your node in the meta channel:

   ```bash
   agent-gossip meta merge --gossip "$GOSSIP" --nickname "$NICKNAME" --merge '{"peers":{"'"$NICKNAME"'":{"pod":"'"$INSTANCE"'","node":"<node>","role":"<role>","status":"busy"}}}'
   ```

From now on, act in your role, and obey its boundaries and the boundaries of
the pod.

## Offer the other nodes

Read the meta channel to find the peers:

```bash
agent-gossip meta get --gossip "$GOSSIP" --nickname "$NICKNAME"
```

A peer is **free** if its entry has `"pods": "open"` (the peer ran
`/roles-pod-join`), it has no `node`, and it has no open offer from you. For
each node with `"peer": null`, pick one free peer, and send a role offer. The
label must start with `pod · `, and the brief must have exactly this form,
because `/roles-pod-join` reads both:

```bash
agent-gossip a2a call --gossip "$GOSSIP" --nickname "$NICKNAME" --to "$PEER" --method SendMessage --label "pod · $INSTANCE · <node>" --text "Role offer. Pod: $INSTANCE. Node: <node>. Role: <role>."
```

Track each offer as a task, per the **Task tracking** rules.

## Drive

Handle each event per the **Receive loop** and **Event handling** sections,
with these rules for role offers:

- **Artifact `loaded <role>`:** approve it with a follow-up that carries
  `--task-id`. Then set the peer of the node:

  ```bash
  agent-gossip state merge --gossip "$GOSSIP" --nickname "$NICKNAME" --merge '{"pods":{"'"$INSTANCE"'":{"nodes":{"<node>":{"peer":"<peer>"}}}}}'
  ```

- **`failed` or `task_timeout`:** the node is open again. Offer it to the
  next free peer.
- **No free peer:** print this line. Tell the user that each other agent
  runs `/roles-pod-join`. On each later batch, read meta again, and offer the
  open nodes to the new free peers:

  ```text
  pod · waiting · $INSTANCE · <n> nodes open
  ```

## Done

When every node of `/pods/$INSTANCE` has a `peer`:

1. Broadcast the roster, so that each node can find the peer at the other
   end of its edges:

   ```bash
   agent-gossip a2a broadcast --gossip "$GOSSIP" --nickname "$NICKNAME" --text "pod $INSTANCE is up: <node> <peer>, <node> <peer>, …"
   ```

2. Print this line last:

   ```text
   pod · up · $INSTANCE · <node> <peer> · <node> <peer> · …
   ```
