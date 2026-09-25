---
name: role-list
description: List the agent roles that the current project can see, with the `agent-role` CLI. Roles are OKF bundles in `.agent-roles/roles/` folders from the cwd up to `/`. Use when the user says "/role-list", "list roles", or "what roles are there".
allowed-tools: Bash(agent-role:*)
---

# role-list

`agent-role` finds roles in `.agent-roles/roles/` in the current directory
and in each parent directory, up to `/`. If two directories have a role with
the same name, the role nearer to the current directory wins.

## Preflight

Make sure that `agent-role` is on `PATH`:

```sh
agent-role --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-role` repository. Then stop.

## Arguments

An optional tag regex. With no argument, list all roles.

## List

```sh
agent-role list                 # all roles
agent-role list --tag '<regex>' # only roles with a tag that matches
```

Each output line is `name<TAB>description<TAB>tags<TAB>path`. Show the roles
as a Markdown table with the columns name, description, and tags. Do not show
the path unless the user asks for it.

If the output is empty, print:

```text
role · none · no .agent-roles/roles/ folder from the cwd up to /
```

The CLI writes a `warning:` line to stderr for each role that is not valid.
Show each warning to the user after the table.
