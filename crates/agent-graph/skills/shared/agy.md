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
- each command that reads or writes under `~/.agent-graph`, such as `mkdir`,
  `cp`, `tar`, `ls`, `rm`, and `cat`
- each `curl` command
