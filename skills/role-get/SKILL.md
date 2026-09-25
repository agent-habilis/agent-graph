---
name: role-get
description: Load one agent role into this session with the `agent-role` CLI, so that the agent acts in that role and obeys its boundaries. Use when the user says "/role-get <name>", "load the <name> role", or "act as the <name> role".
allowed-tools: Bash(agent-role:*)
---

# role-get

A role is a text that changes how an agent behaves. `agent-role get` prints
the body of the role, with its `@file` includes expanded.

## Preflight

Make sure that `agent-role` is on `PATH`:

```sh
agent-role --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-role` repository. Then stop.

## Arguments

The role name. If there is no name, print:

```text
role · usage · /role-get <name>
```

Then stop. To see the names, the user runs `/role-list`.

## Get

```sh
agent-role get <name>
```

Run it in the foreground. The tool result puts the body of the role into this
session. If the command fails, show its `error:` line and stop.

When the command succeeds:

1. Act in this role until the user loads another role or clears the session.
2. Obey each item in the `## Boundaries` section of the role, if the role has
   one. A boundary is a rule that you must not break. If a request from the
   user breaks a boundary, say which boundary it breaks, and ask before you
   continue.
3. Print only this line. Do not summarize the role:

```text
role · loaded · <name>
```
