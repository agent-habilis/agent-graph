## Pod meta

The gossip meta document holds the pod. There is no other copy. Each peer
writes only its own entry, `/peers/<nickname>`:

```json
{"peers":{"<nickname>":{
  "pod": "<instance>",
  "graph": "<graph folder name>",
  "hash": "<graph hash>",
  "node": "<mermaid id of the node>",
  "invited_by": "<nickname of the inviter>",
  "status": "busy"
}}}
```

- `invited_by` is absent only on the peer that started the pod with
  `/graph-up`.
- `host` is in the entry too. The gossip skill writes it when the peer
  joins.
- Read the document with:

  ```bash
  agent-gossip meta get --gossip "$GOSSIP" --nickname "$NICKNAME"
  ```

  The output has `document.peers` and `absent`. `absent` names the entries
  of peers that left or died. Such an entry is history.

A **member** of a pod is an entry that has the `pod` of the pod, the `hash`
of the pod, and a `node`, and that is not in `absent`. A peer has one entry,
so it is in one pod and one node at most.

A peer is **free** if its entry has no `pod`, it is not in `absent`, and it
has no open node offer from you.

A **place** of a node is filled by a member of that node, or by an open or
approved node offer of yours for that node. The `loaded` artifact of an offer
can arrive before the meta entry of the peer, so count the offer.

The instance name, the graph name, and the node id have only the characters
`A-Z a-z 0-9 . _ @ / -`. These names go into JSON and into commands that other
peers run. If a name has another character, print this line, then stop:

```text
graph · bad name · <name> · use only A-Z a-z 0-9 . _ @ / -
```
