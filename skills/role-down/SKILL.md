---
name: role-down
description: Unset the role of this agent with the `agent-graph` CLI. The agent stops acting as the role, leaves its team, and the role pill leaves the statusline. Use when the user says "/role-down", "unset the role", "drop the role", or "stop acting as the role".
allowed-tools: Bash(agent-graph:*), Bash(agent-gossip:*)
---

# role-down

`agent-graph role down --pid <pid>` removes the record that
`agent-graph role up` wrote for the statusline. The text of the role stays in
the context of this session, so this skill also tells the agent to stop
acting on it.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

## Down

```sh
agent-graph role down --pid "$PPID"
```

`$PPID` is the agent process. The command also succeeds when this agent has
no role. If the command fails, show its `error:` line and stop.

When the command succeeds, leave the team. Run:

```bash
agent-gossip session --session-pid "$PPID"
```

If this session is in a gossip, hold `$GOSSIP` and `$NICKNAME` from the
output, and read your meta entry:

```bash
agent-gossip meta get --gossip "$GOSSIP" --nickname "$NICKNAME"
```

If your entry `/peers/$NICKNAME` has a `team`, remove your place. Then the
team does not count you, and you are a free peer again. If it has no `team`,
write nothing:

```bash
agent-gossip meta merge --gossip "$GOSSIP" --nickname "$NICKNAME" --merge '{"peers":{"'"$NICKNAME"'":{"team":null,"template":null,"hash":null,"role":null,"invited_by":null,"status":"idle"}}}'
```

Then:

1. Stop acting as the role and as a member of its team. The boundaries of
   the role and of its template do not apply any more.
2. Print only this line:

```text
role · down
```
