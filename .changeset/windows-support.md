---
default: minor
---

# Run on Windows

agent-graph builds and runs on Windows (x86_64-pc-windows-msvc), and a release ships a `.zip` for it. A graph with LF files has the same hash there as on Unix: the hash joins the path with `/`, and on Windows it reads a CRLF line end as LF. On Windows the home is `USERPROFILE`, or the profile folder of the user when that is unset, the statusline file is `%TEMP%\agent-graph\<pid>.json`, and `--pid` takes `auto`: the binary finds the agent process, the first ancestor that is not a shell, because `$PPID` is 1 under Git Bash. The skills for Windows pass `auto` and plain `hostname`. `plug` and `unplug` remove a junction or a folder symlink as a link. On Unix the behavior does not change.
