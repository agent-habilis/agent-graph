# agent-graph

A graph is a team of agents. A node is one position in the graph, and a peer fills it. Its text goes into the context of an agent and changes how the agent behaves.
Edges are the named relationships between nodes.
The same words apply on disk and at run time: a graph runs in a gossip as an instance, for example `default@main`, and each agent in a node is a peer of the graph.
`agent-graph` finds graphs on disk, checks them, prints them, and draws running graphs.

## Layout

```
.agent-graph/
  default/
    index.md        # the graph: mermaid block, boundaries, edges
    shared/         # optional files that the nodes can include
    nodes/
      worker/
        index.md    # the node "default/worker"
        tone.md     # printed only if index.md includes it
```

Each graph folder and each node folder is an [OKF](https://cloud.google.com/blog/products/data-analytics/how-the-open-knowledge-format-can-improve-data-sharing) bundle.
A graph folder is self-contained: it holds its nodes, so you can share the graph as one folder.
Every node lives in a graph. A node has the name `<graph>/<node>`, for example `default/worker`.

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
---
```

All peers of one node have the same node name. Their gossip nicknames tell them apart.

## Discovery

`agent-graph` reads `.agent-graph/` in the current directory and in each parent directory, up to `/`.
If two directories have a graph with the same name, the graph nearer to the current directory wins, with all its nodes.

## Commands

```sh
agent-graph graph list                     # name<TAB>description<TAB>scope<TAB>path
agent-graph graph get default              # the body of the graph, with includes expanded
agent-graph graph nodes default            # id<TAB><graph>/<node><TAB>lead|-<TAB>count<TAB>model
agent-graph graph lint [default]           # path:line: reason, for each error
agent-graph graph hash default             # SHA-256 of the graph folder: the graph id between peers
agent-graph graph init [<dir>]             # write the default graph into <dir>/.agent-graph/
agent-graph node list                      # name<TAB>description<TAB>tags<TAB>path
agent-graph node list --tag '^go$'         # only nodes with a tag that matches the regex
agent-graph node up default/worker         # the body of the graph, then the body of the node
agent-graph node up default/worker --pid N # also write /tmp/agent-graph/N.json for the statusline
agent-graph node down --pid N              # remove /tmp/agent-graph/N.json
agent-graph graph topology --meta F --peers F --me NICK  # draw the running graph from gossip meta JSON; --mermaid for the source
agent-graph plug [--agent A] [--path DIR]  # install the graph skills into each detected agent
agent-graph unplug [--agent A] [--path DIR] # remove the graph skills that plug installed
```

If a graph or node is not valid, `list` writes a warning to stderr and skips it.
The `--tag` regex is not anchored, so `go` also matches `mongo`.

## Defaults

`agent-graph graph init` writes the files in [`defaults/`](defaults/) into `.agent-graph/`: the `default` graph with its `orchestrator`, `worker`, and `advisor` nodes.
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
{"graph":"default","graph_icon":"󰡉","node":"worker","node_icon":"󱌢","pid":123}
```

`pid` is the Claude Code process. A statusline script gets the same pid as its parent process.
The `graph` key is the `title` of the graph, or its folder name when it has no title.
A graph or node without an `icon` has no icon key.

## Skills

The agent skills (`/graph-up`, `/graph-down`, `/node-up`, `/node-down`, `/node-invite`, `/graph-topology`, and more) are in the binary.
`agent-graph plug` writes them into each agent on this machine: Claude Code, pi, Codex, Cursor, and opencode.
An agent that is not on the machine is skipped.
Run `plug` again after each install of a new binary. It also removes the `template-*`, `role-*`, and `team-*` skills of a binary from before the rename back to graph and node.
`plug` replaces a symbolic link to an old skill folder, and it does not change the target of the link.

The skill sources in [`skills/`](skills/) are templates. See [`skills/README.md`](skills/README.md).

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
| `install` | `cargo install --force --locked` from the repo root |
| `release` | Builds the release binary |
| `clean` | `cargo clean` plus the llvm-cov target dir |
