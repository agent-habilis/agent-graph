---
name: roles-init
description: Create a starter `.agent-roles/` folder in the current project, with the worker, advisor, and qa roles and a dev-team pod. Does nothing if the folder exists. Use when the user says "/roles-init", "init roles", or "set up agent roles for this project".
allowed-tools: Bash(git rev-parse:*), Bash(agent-role:*)
---

# roles-init

Create `.agent-roles/` in the current project with `agent-role init`.

The defaults are:

- `pods/dev-team`: 1 worker (public), 1 advisor, and 1 qa, per git branch.
  - `dev-team/worker`: writes the code on one branch.
  - `dev-team/advisor`: reviews the work of the worker.
  - `dev-team/qa`: runs the tests and tries the change as a user.

## Target

The target is the root of the git repository. If the cwd is not in a git
repository, the target is the cwd:

```sh
git rev-parse --show-toplevel 2>/dev/null || pwd
```

Hold the output as `$TARGET`.

## Preflight

Make sure that `agent-role` is on `PATH`:

```sh
agent-role --version
```

If the command is not found, tell the user to run `cargo task install` in the
`agent-role` repository. Then stop.

## Init

```sh
agent-role init "$TARGET"
```

The binary holds the default files. It prints each file that it writes. If
`$TARGET/.agent-roles` exists, as a folder or a file, it writes nothing and
prints `… already exists` to stderr. It exits 0 in both cases.

If the output says `already exists`, print this line, then stop:

```text
roles · exists · $TARGET/.agent-roles
```

Else show the new roles:

```sh
cd "$TARGET" && agent-role list
```

Show the roles as a Markdown table with the columns name and description.
Print this line last:

```text
roles · created · $TARGET/.agent-roles
```
