---
name: team-up
description: Start a team from a team template in the current gossip. This agent is the founder. It takes one role of the template, then offers each open place to one free peer of the gossip, through gossip tasks. A role with `count` n needs n members at the start. The team lives in the gossip meta. Done when every role has its count. Use when the user says "/team-up <template> [<role>] [--team <name>]", "team up", or "start the <template> team".
allowed-tools: Bash(agent-graph:*), Bash(agent-gossip:*), Bash(git rev-parse:*), Bash(git branch:*), Bash(basename:*), Bash(hostname:*), Bash(mkdir:*), Bash(cp:*), Bash(tar:*)
---

# team-up

A team template describes a team of agents. A team is one run of a template
in a gossip. Each role of the template needs `count` members (1 by default)
at the start. Later, a member can add more with `/role-invite`. All members
of one role have the same role name. Their gossip nicknames tell them apart.
This agent is the **founder**. It is a member of the team too. It takes its
own role first, then offers each open place to one free peer of the gossip.
`up` is done when every role has `count` filled places.

This skill runs in a session that is already in a gossip. It uses the
**Receive loop**, **Decisions**, **Event handling**, and **Task tracking**
sections of the gossip skill that joined the gossip. Obey them here too: keep
the bell armed, and track each task in the todo widget.

## Arguments

- `<template>`: the template name, from `agent-graph template list`.
- `<role>`: optional. The mermaid id of the role that this agent takes. With
  no `<role>`, take the first role that `agent-graph template roles
  <template>` prints.
- `--team <name>`: optional. The instance name of the team.

If there is no `<template>`, print this line, then stop:

```text
team · usage · /team-up <template> [<role>] [--team <name>]
```

<!-- include path="../shared/gossip-session.md" -->

<!-- include path="../shared/team-meta.md" -->

## Gate

1. Read and check the template:

   ```bash
   agent-graph template lint <template>
   agent-graph template list
   agent-graph template roles <template>
   agent-graph template hash <template>
   ```

   If `lint` fails, show its lines, then stop. From `template list`, hold the
   `scope` of the template. Hold the lines of `template roles` as `$ROLES`.
   `model` is the model that the role prefers, or empty. Hold `<template>` as
   `$TEMPLATE` and the hash as `$HASH`. The hash is the id of the template
   between peers.

2. Hold the instance name as `$INSTANCE`:
   - `--team <name>` → `<name>`.
   - `scope: branch` → `<template>@<branch>`, with `git branch --show-current`.
   - `scope: project` → `<template>@<repo>`, with
     `basename "$(git rev-parse --show-toplevel)"`.

   Check `$INSTANCE`, `$TEMPLATE`, and each role id per the **Team meta**
   section.

3. Read the meta document, and look at your own entry:
   - It has another `team`: print this line, then stop.

     ```text
     team · in team <team> · run /role-down first
     ```

   - It has `team` `$INSTANCE`: this is a resume. Take `$TEMPLATE` and
     `$HASH` from your entry, not from the local template, so that new offers
     carry the hash of the team. Go to **Offer the open places**.

4. If any live entry (not in `absent`) other than yours has `team`
   `$INSTANCE`, with any hash and with or without a `role`, print this line,
   then stop:

   ```text
   team · exists · $INSTANCE · use /role-invite from a member
   ```

## Take your own role

1. Load your role. This prints the template and the role into this session,
   and records them for the statusline:

   ```bash
   agent-graph role up <template>/<role> --pid "$PPID"
   ```

2. Record your place in your meta entry:

   ```bash
   agent-gossip meta merge --gossip "$GOSSIP" --nickname "$NICKNAME" --merge '{"peers":{"'"$NICKNAME"'":{"team":"'"$INSTANCE"'","template":"'"$TEMPLATE"'","hash":"'"$HASH"'","role":"<id>","status":"busy"}}}'
   ```

3. Do the **Second founder check**.

4. From now on, act as your role, and obey its boundaries and the boundaries
   of the template.

## Second founder check

Another peer can start the same team at the same time, and meta arrives with
a delay. Do this check after your own merge, and again on each later batch,
before each new offer.

Read the meta document. If another live entry has `team` `$INSTANCE` and no
`invited_by`, the smaller nickname keeps the team. If that is the other peer:

1. Cancel each open offer of yours with `--method CancelTask`.
2. Remove your place:

   ```bash
   agent-gossip meta merge --gossip "$GOSSIP" --nickname "$NICKNAME" --merge '{"peers":{"'"$NICKNAME"'":{"team":null,"template":null,"hash":null,"role":null,"status":"idle"}}}'
   agent-graph role down --pid "$PPID"
   ```

3. Print this line, then stop:

   ```text
   team · taken · $INSTANCE · by <nickname>
   ```

## Offer the open places

Do the **Second founder check** first. A role has `count` minus its filled
places open places, per the **Team meta** section. For each open place, pick
one free peer. If the role has a `model`, pick first a free peer whose meta
`model` contains it (case-insensitive, so `fable` matches
`claude-fable-5-1`). If no such peer is free, pick any free peer. Send the
offer per the **Role offer** section.

<!-- include path="../shared/role-offer.md" -->

<!-- include path="../shared/offer-drive.md" -->

After a `failed` or `task_timeout`, offer the place to the next free peer.
When no peer is free, print this line. On each later batch, read meta again,
and offer the open places to new free peers, for example a peer that just
joined the gossip:

```text
team · waiting · $INSTANCE · <n> places open
```

## Done

When each role of `$INSTANCE` has `count` filled places, and no offer is
open:

1. Broadcast the roster, so that each member can find the members at the
   other end of its handoffs:

   ```bash
   agent-gossip a2a broadcast --gossip "$GOSSIP" --nickname "$NICKNAME" --text "team $INSTANCE is up: <id> <member> <member>, <id> <member>, …"
   ```

2. Print this line last:

   ```text
   team · up · $INSTANCE · <id> <member> <member> · <id> <member> · …
   ```
