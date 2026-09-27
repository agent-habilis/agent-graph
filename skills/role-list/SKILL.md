---
name: role-list
description: List the roles of the team templates that the current project can see, with the `agent-graph` CLI. Templates are OKF bundles in `.agent-graph/` folders from the cwd up to `/`, and each template holds its roles in `<template>/roles/`. Use when the user says "/role-list", "list roles", or "what roles are there".
allowed-tools: Bash(agent-graph:*)
---

# role-list

`agent-graph` finds team templates in `.agent-graph/` in the current
directory and in each parent directory, up to `/`. Each template holds its
roles in `<template>/roles/`. A role has the name `<template>/<role>`. If two
directories have a template with the same name, the template nearer to the
current directory wins, with all its roles.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

## Arguments

An optional tag regex. With no argument, list all roles.

## List

```sh
agent-graph role list                 # all roles
agent-graph role list --tag '<regex>' # only roles with a tag that matches
```

Each output line is `name<TAB>description<TAB>tags<TAB>path`. Show the roles
as a Markdown table with the columns name, description, and tags. Do not show
the path unless the user asks for it.

If the output is empty, print:

```text
role · none · no roles in .agent-graph/ from the cwd up to /
```

The CLI writes a `warning:` line to stderr for each role that is not valid.
Show each warning to the user after the table.
