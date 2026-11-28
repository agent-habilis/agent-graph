## Antigravity CLI

Read this section only if you run in the Antigravity CLI (`agy`). In any
other agent, skip it.

The Standard Sandbox Mode of `agy` blocks each read and each write under
`~/.agent-graph` with `EPERM`. It also blocks all network access. If the
`run_command` tool has the parameter `BypassSandbox`, set it to true in the
call of each of these commands. If `run_command` has no such parameter, the
command already runs outside the sandbox: run it as it is.

- each `agent-graph` command
- each `agent-gossip` command
- each command that reads or writes under `~/.agent-graph`

`agy` matches an Always Allow rule against the start of a command line, and a
shell variable such as `$NAME` or `${NAME}` defeats the match. So a command
line has literal values only. Each `$NAME` in a command of this skill
is a placeholder, not a shell variable. Before you run the command, write the
value of `$NAME` in its place. Do not set a shell variable.
