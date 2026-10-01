## Node offer

A node offer is a gossip task. The brief holds the steps for the peer. Hold
`<id>` (the mermaid id) and `<node>` (the node folder name) from
`agent-graph graph nodes`: each line is
`id<TAB><graph>/<node><TAB>lead|-<TAB>count<TAB>model`. Hold your own
nickname as `$INVITER`.

Hold the graph name as `$SOURCE`. If `agent-graph graph hash
"$GRAPH"` prints `$HASH`, `$SOURCE` is `$GRAPH`. Else `$SOURCE` is
`$HASH`, and you run the `agent-graph` commands of this section and of the
**Drive** section from your home folder, where `~/.agent-graph/$HASH` is.

A peer without agent-graph cannot load a node from the graph. It asks you
for the node context: the text that `agent-graph node up` prints, the body of
the graph and then the body of the node. It checks that text with a hash.
Compute that hash before the offer:

```bash
CONTEXT_HASH=$(agent-graph node up "$SOURCE/<node>" | shasum -a 256 | cut -d' ' -f1)
```

Before the offer, look at the meta `host` of the peer. If it is your host
(`hostname -s`), and `~/.agent-graph/$HASH/` does not exist, copy the graph
folder there. Then the peer has the graph, and no transfer is necessary.
The folder is the last column of `agent-graph graph list` for a local
graph:

```bash
mkdir -p ~/.agent-graph/$HASH && cp -R "<graph folder>/." ~/.agent-graph/$HASH/
```

Send one offer per place:

```bash
agent-gossip a2a call --gossip "$GOSSIP" --nickname "$INVITER" --to "$PEER" --method SendMessage --label "graph · $INSTANCE · <id>" --text "Node offer. Instance: $INSTANCE. Graph: $GRAPH. Hash: $HASH. Context hash: $CONTEXT_HASH. Node: <node> (id <id>). Inviter: $INVITER. If your meta entry already has an instance, or you accepted another node offer, decline with the reason: has a node. Else, after you accept, do A if the command agent-graph is on your PATH (command -v agent-graph), else do B. A1. If ~/.agent-graph/$HASH/ does not exist, open a task to the inviter: agent-gossip a2a call --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --to $INVITER --method SendMessage --label \"graph · send · $HASH\" --text \"send graph $HASH\". Its artifact carries the graph as a file part: fetch it with agent-gossip a2a fetch <payload.parts[].url> --output /tmp/$HASH.tar, then run: mkdir -p ~/.agent-graph/$HASH && tar -xf /tmp/$HASH.tar -C ~/.agent-graph/$HASH. Approve that artifact. Then run agent-graph graph hash $HASH from your home folder. If it does not print $HASH, remove ~/.agent-graph/$HASH and decline this offer with the reason: bad graph. A2. Run /node-up $HASH/<node> from your home folder. If you do not have the /node-up skill, run agent-graph node up $HASH/<node> --pid \"\$PPID\" from your home folder, act as the node that it prints, and obey each ## Boundaries section in it. A3. Record the graph in your meta entry: agent-gossip meta merge --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --merge '{\"peers\":{\"<your nickname>\":{\"instance\":\"$INSTANCE\",\"graph\":\"$GRAPH\",\"hash\":\"$HASH\",\"node\":\"<id>\",\"invited_by\":\"$INVITER\",\"status\":\"busy\"}}}'. B1. Open a task to the inviter: agent-gossip a2a call --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --to $INVITER --method SendMessage --label \"graph · send · $HASH/<node>\" --text \"send node $HASH/<node>\". Its artifact carries the context of the graph and of your node as a file part: fetch it with agent-gossip a2a fetch <payload.parts[].url> --output /tmp/$HASH-<id>.md. Approve that artifact. B2. Run shasum -a 256 /tmp/$HASH-<id>.md (or sha256sum). If its first field is not $CONTEXT_HASH, decline this offer with the reason: bad node context. B3. Read /tmp/$HASH-<id>.md: it is the context of your graph and of your node. Act as that node, as a peer of the graph, and obey each ## Boundaries section in it. B4. Record the graph in your meta entry. You could not check the graph hash, so mark it as not verified: agent-gossip meta merge --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --merge '{\"peers\":{\"<your nickname>\":{\"instance\":\"$INSTANCE\",\"graph\":\"$GRAPH\",\"hash\":\"$HASH\",\"node\":\"<id>\",\"invited_by\":\"$INVITER\",\"verified\":false,\"status\":\"busy\"}}}'. Then, after A or B: read agent-gossip meta get: the peers of the graph are the entries with instance $INSTANCE and hash $HASH that are not in absent. They are the peers at the ends of your edges. If a broadcast later says graph $INSTANCE is down, follow it: leave your node and clear your meta entry. Send the artifact: loaded <node>. If a step fails, decline with its error line."
```

Track each offer as a task, per the **Task tracking** rules of the gossip
skill.
