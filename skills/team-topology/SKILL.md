---
name: team-topology
description: Draw the live topology of the current team in the terminal, with the `agent-graph team topology` CLI and the gossip data from `agent-gossip`. It shows one box per member in its role, with its model and status, the handoffs of the template, and a warning line for each problem. Use when the user says "/team-topology", "show the team", "draw the topology", or "who is in the team".
allowed-tools: Bash(agent-graph:*), Bash(agent-gossip:*)
---

# team-topology

A team is one run of a team template in a gossip. The gossip meta document
holds the team: each member writes its team, template hash, and role into
its own entry. `agent-graph team topology` reads the meta document, the
roster, and the template file, and draws the team: one box per member, and
the handoffs of the template between the members. The output is the same for
the same input.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

## Arguments

`<instance>`: optional. The instance name of the team, for example
`default@main`. With no argument, the command draws the team of this agent.

<!-- include path="../shared/gossip-session.md" -->

## Draw

Give the two gossip documents to the command in one call:

```bash
agent-graph team topology [<instance>] --me "$NICKNAME" \
  --meta <(agent-gossip meta get --gossip "$GOSSIP" --nickname "$NICKNAME") \
  --peers <(agent-gossip peers --gossip "$GOSSIP" --nickname "$NICKNAME")
```

Show stdout in a `text` fence, without changes: it is a box drawing, and it
needs a monospace font. Then show each `warning:` line from stderr, below the
fence.

If the command fails, show its `error:` line, then stop. For
`is not in a team`, tell the user to start a team with `/team-up <template>`,
or to give the `<instance>`. To add a member to the team, a member runs
`/role-invite <role>`.
