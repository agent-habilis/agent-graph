---
type: TeamTemplate
description: One orchestrator, two workers, and one advisor per git branch.
title: Default team
scope: branch
icon: 󰡉
---

```mermaid
flowchart LR
  subgraph default
    orchestrator(("orchestrator")):::lead
    worker(("worker"))
    advisor(("advisor"))
    orchestrator assign@-->|"assign a part"| worker
    worker part-result@-->|"part result"| orchestrator
    orchestrator ask-review@-->|"ask for review"| advisor
    advisor review-result@-->|"review result"| orchestrator
  end
  outside[["user or other team"]]
  outside external@<--> orchestrator
```

## Boundaries

- No communication with roles outside the team, except through `external`.
- Team management (`/team-up`, `/role-invite`, `/role-down`) is permitted
  with any gossip peer.
- No file changes outside the git branch of the team.
- All roles work on the same host, in the same working tree. The workers
  share one git index.

## Handoffs

### assign

The orchestrator sends a part of the task to a worker. The message gives the
path of the repository, the branch, the part, and the files that the worker
can change. Each file has one owner. Shared files, such as `Cargo.toml` or a
`mod.rs`, also get one owner.

A small task can have only one part. If a part needs the result of another
part, the orchestrator sends it after the other part is complete.

The orchestrator also uses this handoff to send a repair request or an answer to
a question of the worker.

### part-result

When a worker finishes its part, it sends the commits and a short summary to
the orchestrator.

The worker also uses this handoff to ask a question, to ask for a file that is
not its file, or to report an error in a file that is not its file.

### ask-review

The orchestrator sends the plan, or the diff from the base commit, to the
advisor. Then it waits for `review-result`.

### review-result

The advisor sends its findings to the orchestrator, one per line, the most
severe first. Each finding has a severity: `blocker`, `concern`, or `nit`.

### external

All messages from and to the user or other teams go through the
orchestrator. The orchestrator gives the final result to the user.
