# agent-graph

A graph is a team of agents. A node is one position in the graph, and a peer fills it. Its text goes into the context of an agent and changes how the agent behaves.
Edges are the named relationships between nodes.
The same words apply on disk and at run time: a graph runs in a gossip as an instance, for example `squad@main`, and each agent in a node is a peer of the graph.
`agent-graph` finds graphs on disk, checks them, prints them, and draws running graphs.

## Layout

```
.agent-graph/
  squad/
    index.md        # the graph: mermaid block, boundaries, edges
    shared/         # optional files that the nodes can include
    nodes/
      worker/
        index.md    # the node "squad/worker"
        tone.md     # printed only if index.md includes it
  traits/
    qa/
      index.md      # the trait "qa"
```

Each graph folder and each node folder is an [OKF](https://cloud.google.com/blog/products/data-analytics/how-the-open-knowledge-format-can-improve-data-sharing) bundle.
A graph folder is self-contained: it holds its nodes, so you can share the graph as one folder.
Every node lives in a graph. A node has the name `<graph>/<node>`, for example `squad/worker`.

A graph `index.md` starts with this frontmatter:

```yaml
---
type: Graph                 # required
description: One orchestrator, two workers, and one advisor per git branch.   # required
scope: branch               # required: branch or project
title: Default graph        # optional, the graph name on the statusline
icon: 󰡉                     # optional, one Nerd Font glyph for the statusline
---
```

In the mermaid block, the node that talks to the user or to other graphs has the class `lead` (`orchestrator(("orchestrator")):::lead`). Each subgraph has exactly one lead. The other nodes need no class.

A node `index.md` starts with this frontmatter:

```yaml
---
type: Node                  # required
description: Challenge plans and point out risks.   # required, one line
title: Advisor              # optional
tags: [review, go]          # optional, inline list only
icon: 󰌵                     # optional, one Nerd Font glyph for the statusline
count: 2                    # optional, the number of peers in this node (default 1)
model: fable                # optional, the model the node prefers; graph-up offers it to a matching peer first
traits: [qa]                # optional, inline list only: the traits that the node can run
---
```

All peers of one node have the same node name. Their gossip nicknames tell them apart.

## Traits

A trait is like a method of a node: a markdown text that drives one routine, for example a QA run.
`/trait-run <name>` runs the trait in a fork of the session. The fork gets a copy of the context, runs the routine, and gives a report.
The session gets only the report, so its context stays as it was before the run.
`traits` is a reserved folder name in `.agent-graph/`: it holds traits, not a graph.

A trait `index.md` starts with this frontmatter:

```yaml
---
type: Trait                 # required
description: Run the checks of the project on the current change.   # required, one line
writes: false               # optional (default false); true lets the trait change tracked files and commit
---
```

`trait show` adds a `## Boundaries` section to a trait with `writes: false`: no changes to files that git tracks, and no commits.
When a node is loaded, `/trait-run <name>` runs only a trait that the node lists in `traits`.
`/trait-run <url>` downloads the trait into a folder in `/tmp`, checks it with `trait check`, and shows its description and `writes`.
The user must confirm before the fork runs it. A URL trait skips the node check. It is one file, so it cannot include other files.

## Discovery

`agent-graph` reads `.agent-graph/` in the current directory and in each parent directory, up to `/`.
If two directories have a graph with the same name, the graph nearer to the current directory wins, with all its nodes.

## Commands

```sh
agent-graph graph list                     # name<TAB>description<TAB>scope<TAB>path
agent-graph graph get squad                # the body of the graph, with includes expanded
agent-graph graph nodes squad              # id<TAB><graph>/<node><TAB>lead|-<TAB>count<TAB>model
agent-graph graph lint [squad]             # path:line: reason, for each error
agent-graph graph hash squad               # SHA-256 of the graph folder: the graph id between peers
agent-graph graph init [<dir>]             # write the default graphs into <dir>/.agent-graph/
agent-graph node list                      # name<TAB>description<TAB>tags<TAB>path
agent-graph node list --tag '^go$'         # only nodes with a tag that matches the regex
agent-graph node up squad/worker           # the body of the graph, then the body of the node
agent-graph node up squad/worker --pid N   # also write /tmp/agent-graph/N.json for the statusline
agent-graph node down --pid N              # remove /tmp/agent-graph/N.json
agent-graph trait list                     # name<TAB>description<TAB>writes<TAB>path
agent-graph trait check <path>             # description<TAB>writes<TAB>path of a trait file or folder, or an error
agent-graph trait show qa                  # the body of the trait, with includes expanded
agent-graph trait show qa --node pair/worker   # fail if the node does not list the trait
agent-graph trait show /tmp/x/index.md     # a trait file by path: no node check
agent-graph graph topology --meta F --peers F --me NICK  # draw the running graph from gossip meta JSON; --mermaid for the source
agent-graph plug [--agent A] [--path DIR]  # install the graph skills into each detected agent
agent-graph unplug [--agent A] [--path DIR] # remove the graph skills that plug installed
```

If a graph, node, or trait is not valid, `list` writes a warning to stderr and skips it.
The `--tag` regex is not anchored, so `go` also matches `mongo`.

## Defaults

`agent-graph graph init` writes the files in [`defaults/`](crates/agent-graph/defaults/) into `.agent-graph/`: the `squad` graph with its `orchestrator`, `worker`, and `advisor` nodes, and the `pair` graph with one `worker` (the lead) and one `advisor`.
It also writes four traits. The `worker` nodes list `qa` (run the checks), `repro` (write a failing test for a bug, `writes: true`), and `perf` (measure against the base commit). The `advisor` nodes list `review`, and their steps run it.
The build puts these files into the binary, so a user needs only the binary.
If `.agent-graph` exists, `init` writes nothing and exits 0.

## Includes

In `index.md`, a line that contains only `@<path>` is replaced with the body of that file, without its frontmatter.

- The path is relative to the file that contains the include.
- Included files can include other files. A cycle is an error.
- The path must stay inside the graph folder.
- The CLI ignores `@` lines inside fenced code blocks.

## Statusline

`node up --pid <pid>` writes the graph, the node, and their icons to `/tmp/agent-graph/<pid>.json`:

```json
{"graph":"squad","graph_icon":"󰡉","node":"worker","node_icon":"󱌢","pid":123}
```

`pid` is the Claude Code process. A statusline script gets the same pid as its parent process.
The `graph` key is the `title` of the graph, or its folder name when it has no title.
A graph or node without an `icon` has no icon key.

## Skills

The agent skills (`/graph-up`, `/graph-down`, `/node-up`, `/node-down`, `/node-invite`, `/graph-topology`, `/trait-run`, and more) are in the binary.
`agent-graph plug` writes them into each agent on this machine: Claude Code, pi, Codex, Cursor, and opencode.
An agent that is not on the machine is skipped.
Run `plug` again after each install of a new binary. It also removes the `template-*`, `role-*`, and `team-*` skills of a binary from before the rename back to graph and node.
`plug` replaces a symbolic link to an old skill folder, and it does not change the target of the link.

The skill sources in [`crates/agent-graph/skills/`](crates/agent-graph/skills/) are templates. See [`skills/README.md`](crates/agent-graph/skills/README.md).

## Exit codes

- `0`: success.
- `1`: error. For example, the node is not found, an include is not valid, the regex is not valid, or `graph lint` found errors.

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
| `run` | `cargo run --` with the rest forwarded (`cargo task run node list`) |
| `install` | `cargo install --path crates/agent-graph --force --locked` |
| `release` | Builds the release binary |
| `clean` | `cargo clean` plus the llvm-cov target dir |
