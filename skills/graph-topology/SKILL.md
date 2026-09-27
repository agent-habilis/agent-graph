---
name: graph-topology
description: Draw the live topology of the current agent graph pod in the terminal, with the `agent-graph topology` CLI and the gossip data from `agent-gossip`. It shows one box per peer in its node, with its model and status, the edges of the graph, and a warning line for each problem. Use when the user says "/graph-topology", "show the pod", "draw the topology", or "who is in the pod".
allowed-tools: Bash(agent-graph:*), Bash(agent-gossip:*)
---

# graph-topology

A pod is a graph that runs in a gossip. `agent-graph topology` reads the
gossip documents and the graph file, and draws the pod: one box per peer that
fills a place, and the edges of the graph between the peers. The output is
the same for the same input.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

## Arguments

`<instance>`: optional. The pod, as `<graph>@<branch or repo>`. With no
argument, the command draws the pod of this agent.

## Gossip

Find the gossip of this session:

```bash
agent-gossip session --session-pid "$PPID"
```

Hold `$GOSSIP` and `$NICKNAME` from the output. If there is no session, print
this line, then stop:

```text
graph · no gossip · create or join a gossip first
```

## Draw

Give the three gossip documents to the command in one call:

```bash
agent-graph topology [<instance>] --me "$NICKNAME" \
  --state <(agent-gossip state get --gossip "$GOSSIP" --nickname "$NICKNAME") \
  --meta <(agent-gossip meta get --gossip "$GOSSIP" --nickname "$NICKNAME") \
  --peers <(agent-gossip peers --gossip "$GOSSIP" --nickname "$NICKNAME")
```

Show stdout in a `text` fence, without changes: it is a box drawing, and it
needs a monospace font. Then show each `warning:` line from stderr, below the
fence.

If the command fails, show its `error:` line, then stop. For
`is not in a pod`, tell the user to start a pod with `/graph-up <graph>`, or
to give the `<instance>`.
