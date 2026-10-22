# `agent-graph` 🧑‍🧒‍🧒

A graph is a team of AI agents. Each agent takes one role in the team, for example orchestrator, worker, or advisor.
You write the roles in markdown. `agent-graph` starts the team, and the agents work together over [`agent-gossip`](https://github.com/agent-habilis/agent-gossip).

## Features

- **A team in one command** — `/graph-up squad` fills each role with an agent from your gossip.
- **Ready-made teams** — `squad` (orchestrator, workers, and advisor) and `pair` (worker and advisor).
- **Your own roles** — a role is a markdown file. Change it, or write a new one, and keep it in your project.
- **Traits** — routines that an agent runs in a fork of its session, for example a QA run. Only the report comes back, so your context stays clean.
- **Multi-model** — agents that run on different models work in the same graph. A role can name the model that it prefers.
- **Multi-harness** — one binary plugs into [Claude Code](https://claude.com/claude-code), [pi](https://pi.dev), Cursor, [Codex](https://developers.openai.com/codex/), and opencode.
- **Live view** — `/graph-topology` draws the running graph: who is in each role, and what each agent does.

## Installation

```sh
# Homebrew for macOS and Linux
brew install agent-habilis/tap/agent-graph && agent-graph plug

# From source everywhere else
cargo install --git https://github.com/agent-habilis/agent-graph && agent-graph plug
```

`plug` writes the skills into each agent that it finds on your machine.
Run `plug` again after each upgrade.

A graph runs inside a gossip, so you also need [`agent-gossip`](https://github.com/agent-habilis/agent-gossip).

## Quick start

1. **`/graph-init`**
   Run it in your project. It writes the default teams into `.agent-graph/`.
2. **`/gossip-create`**
   Run it in one agent. It gives you a gossip hash.
3. **`/gossip-join`**
   Run `/gossip-join <hash>` in each other agent, with the hash from step 2.
4. **`/graph-up squad`**
   Run it in the first agent. The first agent takes the lead role. It then offers each open role to one free agent of the gossip.

## Skills

`agent-graph plug` installs one skill for each operation. You call them as commands in your agent, shown here with the `/` prefix of Claude Code.
Codex uses `$graph-up`, and pi uses `/skill:graph-up`.

### `/graph-init`

Creates the `.agent-graph/` folder in your project, with two teams and four traits.
If the folder exists, it changes nothing.

- `squad` — an orchestrator, two workers, and an advisor.
- `pair` — a worker that talks to you, and an advisor.

### `/graph-list`

Lists the teams that your project can see.

### `/graph-up`

`/graph-up <graph>` starts a graph in the gossip that you are in. Your agent becomes the founder and takes the lead role.
Then it offers each other role to a free agent, and each agent loads its role. The command ends when every role has its agents.

### `/graph-node-invite`

`/graph-node-invite <role>` adds one more agent to a running graph, for example a third worker.
Any agent of the graph can run it.

### `/graph-topology`

Draws the running graph in the terminal: one box for each agent, with its model and status, and the links between the roles.
It also warns about a problem, for example a role with no agent.

### `/graph-down` and `/graph-leave`

`/graph-down` stops the whole graph. Every agent leaves its role.
`/graph-leave` takes only your agent out. The other agents keep working.

### `/graph-trait-run`

A trait is a routine, like a method of a role. `/graph-trait-run qa` runs the QA trait in a fork of your session.
The fork does the work and sends you a report. Your context stays as it was, plus the report.

- The default traits are `qa` (run the checks of the project), `repro` (write a failing test for a bug), `perf` (measure a change), and `review` (review a plan or a diff).
- An agent in a graph runs only the traits that its role lists.
- You can also give a URL. You see the description of the trait first and must confirm before the fork runs it.

### Roles outside a graph

A standalone role is one role with no team around it. It lives in `.agent-graph/nodes/`.

- `/graph-node-list` — list the standalone roles.
- `/graph-node-up <name>` — your agent takes the role.
- `/graph-node-down` — your agent drops the role.

## Links

- [agent-gossip](https://github.com/agent-habilis/agent-gossip)
- [License](LICENSE)
- [agent-habilis](https://agent-habilis.com)
- [Homebrew tap](https://github.com/agent-habilis/homebrew-tap)
