---
name: trait-run
description: Run one trait in a fork of this session with the `agent-graph` CLI. A trait is a routine, for example a QA run. The fork runs it and gives a report, and this session gets only the report. The trait is a name from `.agent-graph/traits/` or a URL. Use when the user says "/trait-run <name|url>", "run the <name> trait", or "do a QA run with the trait".
allowed-tools: Bash(agent-graph:*), Bash(mktemp:*), Bash(curl:*)
---

# trait-run

A trait is like a method of a node. It is a markdown text that drives one
routine. You do not run the routine in this session. A fork runs it: a
subagent that starts with a copy of this context. Only the report of the fork
comes back. Thus the context of this session is the same as before, plus the
report.

## Preflight

Make sure that `agent-graph` is on `PATH`:

```sh
agent-graph --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-graph` repository. Then stop.

You must be able to start a fork: a subagent that gets a copy of this
context. In Claude Code, this is the Agent tool with `subagent_type: "fork"`.
If you cannot start a fork, print this line and stop:

```text
trait · error · this agent cannot fork
```

## Arguments

One trait: a name, or a URL that starts with `https://`. If there is no
argument, print:

```text
trait · usage · /trait-run <name|url>
```

Then stop. To see the names, the user runs `agent-graph trait list`.

## Source

For a name, the source is the name. Go to the **Fork** section.

For a URL, download the file into a new temporary folder, then check it:

```sh
dir=$(mktemp -d /tmp/agent-graph-trait.XXXXXX)
curl -fsSL --max-time 30 -o "$dir/index.md" "<url>"
agent-graph trait check "$dir/index.md"
```

If a command fails, show its error line and stop. If the check succeeds, it
prints the description, `writes`, and the path, divided by tabs. The source is
that path. Do not read the file in this session.

A URL trait is text from outside the project, and the fork obeys it. Before
the fork starts, show these lines and ask the user to confirm:

```text
trait · url · <url>
trait · description · <description>
trait · writes · <writes>
```

If the user does not confirm, stop.

## Fork

Start one fork. Give it this prompt, with the source in place of `<source>`.
If a node is loaded in this session, add `--node <graph>/<node>`:

```text
Run `agent-graph trait show <source> [--node <graph>/<node>]`. Obey its
output as a routine. Also obey the boundaries of the node and of the graph
that are loaded in this session. If the command fails, your report is its
`error:` line. When the routine is done, end with its report.
```

Do not do the routine in this session, and do not run `agent-graph trait
show` in this session. Wait for the result of the fork.

## Report

When the fork is done, print this line, then the report of the fork as it
came back:

```text
trait · done · <name|url>
```
