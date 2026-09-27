---
name: template-list
description: List the team templates that the current project can see, with the `agent-graph` CLI. Templates are OKF bundles in `.agent-graph/` folders from the cwd up to `/`. Use when the user says "/template-list", "list templates", or "what team templates are there".
allowed-tools: Bash(agent-graph:*)
---

# template-list

`agent-graph` finds team templates in `.agent-graph/` in the current
directory and in each parent directory, up to `/`. If two directories have a
template with the same name, the template nearer to the current directory
wins.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

## List

```sh
agent-graph template list
```

Each output line is `name<TAB>description<TAB>scope<TAB>path`. Show the
templates as a Markdown table with the columns name, description, and scope.
Do not show the path unless the user asks for it.

If the output is empty, print:

```text
template · none · no .agent-graph/ folder from the cwd up to /
```

The CLI writes a `warning:` line to stderr for each template that is not
valid. Show each warning to the user after the table.
