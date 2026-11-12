---
type: Graph
description: One worker and one advisor per git branch.
title: pair
scope: branch
icon: 󰀎
---

```mermaid
flowchart LR
  subgraph pair
    worker(("worker")):::lead
    advisor(("advisor"))
    worker ask-review@-->|"ask for review"| advisor
    advisor review-result@-->|"review result"| worker
  end
  outside[["user or other graph"]]
  outside external@<--> worker
```

## Boundaries

- No communication with nodes outside the graph, except through `external`.
- Graph management (`/graph-up`, `/graph-down`, `/graph-node-invite`,
  `/graph-leave`) is permitted with any gossip peer.
- No file changes outside the git branch of the graph.
- Reading files outside the working tree is permitted. No file changes
  outside the working tree.
- The two nodes work on the same host, in the same working tree.

## Questions for the user

The leader is the only node that talks to the user. Its meta entry has
`"leader":true`. The leader of your graph is the peer with `"leader":true`, the
same `instance` and `hash` as you, that is not in `absent`. Read the meta each
time you ask, because the leader can change.

A node that is not the leader never asks the user. When it needs an answer
from the user, it opens a gossip task to the leader with the label
`question · <short question>`. The text holds the question and the choices, if
any. Then it keeps working on what it can. The artifact of the task is the
answer: approve it on the batch in which it arrives. If the question belongs
to a task that the leader gave you, ask it with `input-required` on that task
instead.

If there is no live leader, do not ask the user. Keep working, and read the
meta again on each later batch. If you cannot go on without the answer, say so
in the result of your task.

The leader accepts a task labelled `question ·` at once. This replaces the
accept or decline question of gossip-task: do not ask the user whether to
accept. It asks the user the question with the question tool, or in chat if
the session has no such tool. Then it sends the answer as the artifact of the
task, and closes the task when the asker approves. For an `input-required`
question on a task of its own, it relays the answer on that task.

## Edges

### ask-review

The worker sends the plan, or the diff from the base commit, to the advisor.
Then it waits for `review-result`.

### review-result

The advisor sends its findings to the worker, one per line, the most severe
first. Each finding has a severity: `blocker`, `concern`, or `nit`.

### external

All messages from and to the user or other graphs go through the worker. The
worker gives the final result to the user.
