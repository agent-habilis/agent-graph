# agent-graph

A team template describes a team of agents. A role is one position in the template. Its text goes into the context of an agent and changes how the agent behaves.
Handoffs are the named channels between roles.
A team is one run of a template in a gossip, and each agent in a role is a member of the team.
`agent-graph` finds team templates on disk, checks them, prints them, and draws running teams.

## Layout

```
.agent-graph/
  default/
    index.md        # the team template: mermaid block, boundaries, handoffs
    shared/         # optional files that the roles can include
    roles/
      worker/
        index.md    # the role "default/worker"
        tone.md     # printed only if index.md includes it
```

Each template folder and each role folder is an [OKF](https://cloud.google.com/blog/products/data-analytics/how-the-open-knowledge-format-can-improve-data-sharing) bundle.
A template folder is self-contained: it holds its roles, so you can share the template as one folder.
Every role lives in a template. A role has the name `<template>/<role>`, for example `default/worker`.

A template `index.md` starts with this frontmatter:

```yaml
---
type: TeamTemplate          # required
description: One orchestrator, two workers, and one advisor per git branch.   # required
scope: branch               # required: branch or project
title: Default team         # optional, the template name on the statusline
icon: 󰡉                     # optional, one Nerd Font glyph for the statusline
---
```

In the mermaid block, the role that talks to the user or to other teams has the class `lead` (`orchestrator(("orchestrator")):::lead`). A team has exactly one lead. The other roles need no class.

A role `index.md` starts with this frontmatter:

```yaml
---
type: Role                  # required
description: Challenge plans and point out risks.   # required, one line
title: Advisor              # optional
tags: [review, go]          # optional, inline list only
icon: 󰌵                     # optional, one Nerd Font glyph for the statusline
count: 2                    # optional, the number of members in this role (default 1)
model: fable                # optional, the model the role prefers; team-up offers it to a matching peer first
---
```

All members of one role have the same role name. Their gossip nicknames tell them apart.

## Discovery

`agent-graph` reads `.agent-graph/` in the current directory and in each parent directory, up to `/`.
If two directories have a template with the same name, the template nearer to the current directory wins, with all its roles.

## Commands

```sh
agent-graph template list                  # name<TAB>description<TAB>scope<TAB>path
agent-graph template get default           # the body of the template, with includes expanded
agent-graph template roles default         # id<TAB><template>/<role><TAB>lead|-<TAB>count<TAB>model
agent-graph template lint [default]        # path:line: reason, for each error
agent-graph template hash default          # SHA-256 of the template folder: the template id between peers
agent-graph template init [<dir>]          # write the default template into <dir>/.agent-graph/
agent-graph role list                      # name<TAB>description<TAB>tags<TAB>path
agent-graph role list --tag '^go$'         # only roles with a tag that matches the regex
agent-graph role up default/worker         # the body of the template, then the body of the role
agent-graph role up default/worker --pid N # also write /tmp/agent-graph/N.json for the statusline
agent-graph role down --pid N              # remove /tmp/agent-graph/N.json
agent-graph team topology --meta F --peers F --me NICK  # draw the running team from gossip meta JSON; --mermaid for the source
agent-graph plug [--agent A] [--path DIR]  # install the team skills into each detected agent
agent-graph unplug [--agent A] [--path DIR] # remove the team skills that plug installed
```

If a template or role is not valid, `list` writes a warning to stderr and skips it.
The `--tag` regex is not anchored, so `go` also matches `mongo`.

## Defaults

`agent-graph template init` writes the files in [`defaults/`](defaults/) into `.agent-graph/`: the `default` template with its `orchestrator`, `worker`, and `advisor` roles.
The build puts these files into the binary, so a user needs only the binary.
If `.agent-graph` exists, `init` writes nothing and exits 0.

## Includes

In `index.md`, a line that contains only `@<path>` is replaced with the body of that file, without its frontmatter.

- The path is relative to the file that contains the include.
- Included files can include other files. A cycle is an error.
- The path must stay inside the template folder.
- The CLI ignores `@` lines inside fenced code blocks.

## Statusline

`role up --pid <pid>` writes the template, the role, and their icons to `/tmp/agent-graph/<pid>.json`:

```json
{"graph":"default","graph_icon":"󰡉","node":"worker","node_icon":"󱌢","pid":123}
```

The keys are still `graph` and `node`, because agent-statusline reads them.
`pid` is the Claude Code process. A statusline script gets the same pid as its parent process.
The `graph` key is the `title` of the template, or its folder name when it has no title.
A template or role without an `icon` has no icon key.

## Skills

The agent skills (`/team-up`, `/role-invite`, `/team-topology`, and more) are in the binary.
`agent-graph plug` writes them into each agent on this machine: Claude Code, pi, Codex, Cursor, and opencode.
An agent that is not on the machine is skipped.
Run `plug` again after each install of a new binary. It also removes the `graph-*` skills of a binary from before the rename.
`plug` replaces a symbolic link to an old skill folder, and it does not change the target of the link.

The skill sources in [`skills/`](skills/) are templates. See [`skills/README.md`](skills/README.md).

## Exit codes

- `0`: success.
- `1`: error. For example, the role is not found, an include is not valid, the regex is not valid, or `template lint` found errors.

## Development

All development commands go through `cargo task <name>`.
`cargo task` with no argument lists the tasks.

| Task | What it does |
|---|---|
| `test` | `cargo test --workspace --no-fail-fast` |
| `ci` | The full gate: `fmt --check`, clippy, tests |
| `lint` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `fmt` | `cargo fmt --all` |
| `coverage` | `cargo llvm-cov`, installed on demand |
| `run` | `cargo run --` with the rest forwarded (`cargo task run role list`) |
| `install` | `cargo install --force --locked` from the repo root |
| `release` | Builds the release binary |
| `clean` | `cargo clean` plus the llvm-cov target dir |
