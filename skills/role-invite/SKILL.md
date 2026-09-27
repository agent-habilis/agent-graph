---
name: role-invite
description: Add one member to the running team of this agent. Any member of the team can do it. It offers one role of the template to a free peer of the gossip, through a gossip task, and the peer joins the team in that role. Use when the user says "/role-invite <role> [<peer>]", "invite a <role>", "add a <role> to the team", or "grow the team".
allowed-tools: Bash(agent-graph:*), Bash(agent-gossip:*), Bash(hostname:*), Bash(mkdir:*), Bash(cp:*), Bash(tar:*)
---

# role-invite

A team is one run of a team template in a gossip. `/team-up` fills each role
to its `count`. After that, any member can add a member to a role with this
skill. A role can have more members than its `count`. The new member is a
free peer of the gossip. It joins the team that this agent is a member of.

This skill runs in a session that is already in a gossip. It uses the
**Receive loop**, **Decisions**, **Event handling**, and **Task tracking**
sections of the gossip skill that joined the gossip. Obey them here too: keep
the bell armed, and track each task in the todo widget.

Team management is permitted with any peer of the gossip, also for a role
that has no `external` handoff.

## Arguments

- `<role>`: the mermaid id of the role, from `agent-graph template roles`.
- `<peer>`: optional. The nickname of the peer to invite.

If there is no `<role>`, print this line, then stop:

```text
team · usage · /role-invite <role> [<peer>]
```

<!-- include path="../shared/gossip-session.md" -->

<!-- include path="../shared/team-meta.md" -->

## Gate

1. Read the meta document. Your own entry must make you a member of its
   team. If it does not, print this line, then stop:

   ```text
   team · not in a team · start one with /team-up <template>
   ```

   From your entry, hold `team` as `$INSTANCE`, `template` as `$TEMPLATE`,
   and `hash` as `$HASH`. Check `<role>` per the **Team meta** section.

2. Find the template folder with `$HASH`. First try the local template
   `$TEMPLATE`:

   ```bash
   agent-graph template hash "$TEMPLATE"
   ```

   If it prints `$HASH`, use `$TEMPLATE` as the template name below. Else use
   `$HASH` as the template name, and run the commands from your home folder,
   where `~/.agent-graph/$HASH` is.

   ```bash
   agent-graph template roles <template name>
   ```

   If `<role>` is not an id in the output, print this line, then stop:

   ```text
   team · no role <role> in $TEMPLATE
   ```

   If the third column of `<role>` is `lead`, print this line, then stop. A
   team has one lead member, because the lead talks to the user:

   ```text
   team · <role> is the lead · one per team
   ```

3. Pick the peer:
   - With `<peer>`: the peer must be free, per the **Team meta** section. If
     it is not, print `team · <peer> is not free`, then stop.
   - Without `<peer>`: pick a free peer whose meta `model` contains the
     `model` of the role (case-insensitive). If no such peer is free, pick
     any free peer. If no peer is free, print this line, then stop:

     ```text
     team · no free peer · $INSTANCE · <role>
     ```

4. Send the offer per the **Role offer** section.

<!-- include path="../shared/role-offer.md" -->

<!-- include path="../shared/offer-drive.md" -->

After a `failed` or `task_timeout`: with `<peer>`, print
`team · <peer> declined · $INSTANCE · <role>`, then stop. Without `<peer>`,
offer the place to the next free peer, the same way as step 3.

## Done

When the peer's `loaded` artifact is approved:

1. A meta change rings no bell, so tell the team. Broadcast:

   ```bash
   agent-gossip a2a broadcast --gossip "$GOSSIP" --nickname "$NICKNAME" --text "team $INSTANCE · <peer> joined as <role>"
   ```

2. Print this line last:

```text
team · invited · $INSTANCE · <role> <peer>
```
