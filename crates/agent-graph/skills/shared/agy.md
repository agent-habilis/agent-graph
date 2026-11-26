## Antigravity CLI

Read this section only if you run in the Antigravity CLI (`agy`). In any
other agent, skip it.

The Standard Sandbox Mode of `agy` blocks the write to `~/.agent-graph` with
`EPERM`. It also blocks the `agent-gossip` calls. Set `BypassSandbox` to true
in the `run_command` call of each `agent-graph` command and each
`agent-gossip` command.
