---
name: role-up
description: Load one role of a team template into this session with the `agent-graph` CLI, so that the agent acts as that role and obeys its boundaries and the boundaries of its template. Use when the user says "/role-up <template>/<role>", "load the <role> role", or "act as the <role> role".
allowed-tools: Bash(agent-graph:*)
---

# role-up

A team template describes a team of agents. A role is one position in a
template. Its text changes how an agent behaves.
`agent-graph role up <template>/<role>` prints the body of the template
first, then the body of the role, with their `@file` includes expanded.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

## Arguments

The role name, as `<template>/<role>`. If there is no name, print:

```text
role · usage · /role-up <template>/<role>
```

Then stop. To see the names, the user runs `/role-list`.

## Up

```sh
agent-graph role up <template>/<role> --pid "$PPID"
```

Run it in the foreground. The tool result puts the body of the template and
of the role into this session. `$PPID` is the agent process. The CLI records
the template and the role for the statusline under this pid. If the command
fails, show its `error:` line and stop.

When the command succeeds:

1. Act as this role, as a member of its team, until the user loads another
   role or clears the session.
2. Obey each item in each `## Boundaries` section of the role and of its
   template. A boundary is a rule that you must not break. If a request from
   the user breaks a boundary, say which boundary it breaks, and ask before
   you continue.
3. Print only these lines. Do not summarize the role:

```text
template · loaded · <template>
role · loaded · <role>
```
