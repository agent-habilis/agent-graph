---
default: minor
---

# Support the Antigravity CLI, and run agent-graph commands without a shell

The skills now work in `agy`. Its sandbox, its missing working folder, and its Always Allow rule shaped the changes. A command line of an `agy` skill holds no shell construct and no shell variable, so a rule can match it. The skills of the other agents keep `$PPID` and the quote juggle.

New commands and flags:

- `--pid auto` finds the agent process without `$PPID`.
- `--dir <path>` starts discovery in a folder. A leading `~` is the home folder.
- `graph node-hash`, `graph pack`, `graph install <folder>` and `graph install --tar <file> --expect <hash>`, `graph node-up --output <file>`, `trait fetch <url>`, and `graph topology --gossip` replace commands that needed `curl`, `tar`, `cp`, or a pipe.

A name that starts with `._` is not part of a graph now. `graph hash`, `graph pack` and `graph install` skip it, and an include of one is an error. A graph that has such a file has another hash than before, and an older agent-graph computes the old one. A peer on an older version fails the hash check of an offer for that graph.

`trait fetch` trusts the certificate store of the operating system, as `curl` does.

Run `agent-graph plug` again after the upgrade. The `agy` skills need `agent-gossip` with `session --session`: update both tools.
