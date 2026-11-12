## Graph meta

The gossip meta document holds the running graph. There is no other copy. The
document has one entry for each member, keyed by the session key of the member
(`pubkey`, lowercase hex). Your own entry is `document[$PUBKEY]`. Each peer
writes only its own entry. A merge goes to the top level of the entry, with no
wrapper key:

```json
{
  "instance": "<instance>",
  "graph": "<graph folder name>",
  "hash": "<graph hash>",
  "node": "<mermaid id of the node>",
  "invited_by": "<nickname of the inviter>",
  "joined_at": <epoch seconds>,
  "status": "busy"
}
```

A restart of the daemon gives a new key. The old entry stays in `absent` for
up to 1 hour, and the peer reads as free. The new entry is empty. After a
restart, a peer of a graph merges the whole entry again: `instance`, `graph`,
`hash`, `node`, `joined_at`, `status`, and each of `invited_by`, `verified`,
and `leader` that it has. Without `invited_by`, the peer reads as a founder.

The nickname of a peer is the field `.nickname` of its entry, not its key. Two
peers can share a nickname. Never join an entry to a roster row by the
nickname: match the key of the entry to the `pubkey` of the row.

- `invited_by` is absent only on the founder: the peer that started the
  graph with `/graph-up`.
- `joined_at` is the time at which the peer took its node, in seconds since the
  epoch. A reader that numbers the peers of one node sorts them by it, then by
  nickname. A peer without it sorts after the peers that have it.
- `leader` is `true` on the peer in the lead node, the only peer that talks
  to the user. It is absent on every other peer. Peers send their questions
  for the user to that peer.
- `verified` is `false` on a peer without agent-graph. It joined with the
  node context that its inviter sent, and it could not check the graph
  hash. `agent-graph graph topology` warns about such a peer. On a peer
  that checked the hash, `verified` is absent.
- `nickname`, `model`, `harness`, `host`, and `cwd` are in the entry too. The
  gossip skill writes them when the peer joins.
- Read the document with:

  ```bash
  agent-gossip meta get --gossip "$GOSSIP" --nickname "$NICKNAME"
  ```

  The output has `document` and `absent`. `absent` lists the keys of the
  entries of peers that left or died. Such an entry is history. Your own key
  is never in `absent`.

A **peer of a graph** is an entry that has the `instance` of the graph, the
`hash` of the graph, and a `node`, and whose key is not in `absent`. A peer has one
entry, so it is in one graph and one node at most.

A peer is **free** if its entry has no `instance`, its key is not in `absent`, and it
has no open node offer from you.

A **place** of a node is filled by a peer in that node, or by an open or
approved node offer of yours for that node. The `loaded` artifact of an offer
can arrive before the meta entry of the peer, so count the offer.

The instance name is a short hash by default, or the `--name` of `/graph-up`.
The instance name, the graph name, and the node id have only the
characters `A-Z a-z 0-9 . _ @ / -`. These names go into JSON and into
commands that other peers run. If a name has another character, print this
line, then stop:

```text
graph · bad name · <name> · use only A-Z a-z 0-9 . _ @ / -
```
