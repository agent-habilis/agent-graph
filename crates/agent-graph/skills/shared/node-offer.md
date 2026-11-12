## Node offer

A node offer is a gossip task. The brief holds the steps for the peer. Hold
`<id>` (the mermaid id) and `<node>` (the node folder name) from
`agent-graph graph nodes`: each line is
`id<TAB><graph>/<node><TAB>lead|-<TAB>count<TAB>model`. Hold your own
nickname as `$INVITER`.

Hold the graph name as `$SOURCE`. If `agent-graph graph hash
"$GRAPH"` prints `$HASH`, `$SOURCE` is `$GRAPH`. Else `$SOURCE` is
`$HASH`, and you run the `agent-graph` commands of this section and of the
**Drive** section from your home folder, where `~/.agent-graph/graphs/$HASH` is.

A peer without agent-graph cannot load a node from the graph. It asks you
for the node context: the text that `agent-graph graph node-up` prints, the body of
the graph and then the body of the node. It checks that text with a hash.
Compute that hash before the offer:

```bash
CONTEXT_HASH=$(agent-graph graph node-up "$SOURCE/<node>" | shasum -a 256 | cut -d' ' -f1)
```

Before the offer, look at the meta `host` of the peer. If it is your host
<!-- include path="your-host.md" -->
folder there. Then the peer has the graph, and no transfer is necessary.
The folder is the last column of `agent-graph graph list` for a local
graph:

```bash
mkdir -p ~/.agent-graph/graphs/$HASH && cp -R "<graph folder>/." ~/.agent-graph/graphs/$HASH/
```

Send one offer per place:

```bash
<!-- include path="offer-brief.md" -->
```

Track each offer as a task, per the **Task tracking** rules of the gossip
skill.
