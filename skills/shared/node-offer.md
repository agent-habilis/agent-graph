## Node offer

A node offer is a gossip task. The brief holds the steps for the peer. Hold
`<id>` (the mermaid id) and `<node>` (the node folder name) from
`agent-graph graph nodes`: each line is
`id<TAB><graph>/<node><TAB>public|private<TAB>count<TAB>model`. Hold your own
nickname as `$INVITER`.

Before the offer, look at the meta `host` of the peer. If it is your host
(`hostname -s`), and `~/.agent-graph/$HASH/` does not exist, copy the graph
folder there. Then the peer has the graph, and no transfer is necessary. The
folder is the last column of `agent-graph graph list` for a local graph:

```bash
mkdir -p ~/.agent-graph/$HASH && cp -R "<graph folder>/." ~/.agent-graph/$HASH/
```

Send one offer per place:

```bash
agent-gossip a2a call --gossip "$GOSSIP" --nickname "$INVITER" --to "$PEER" --method SendMessage --label "graph · $INSTANCE · <id>" --text "Node offer. Pod: $INSTANCE. Graph: $GRAPH. Hash: $HASH. Node: <node> (id <id>). Inviter: $INVITER. If your meta entry already has a pod, or you accepted another node offer, decline with the reason: has a node. Else, after you accept: 1. If ~/.agent-graph/$HASH/ does not exist, open a task to the inviter: agent-gossip a2a call --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --to $INVITER --method SendMessage --label \"graph · send · $HASH\" --text \"send graph $HASH\". Its artifact carries the graph as a file part: fetch it with agent-gossip a2a fetch <payload.parts[].url> --output /tmp/$HASH.tar, then run: mkdir -p ~/.agent-graph/$HASH && tar -xf /tmp/$HASH.tar -C ~/.agent-graph/$HASH. Approve that artifact. Then run agent-graph graph hash $HASH from your home folder. If it does not print $HASH, remove ~/.agent-graph/$HASH and decline this offer with the reason: bad graph. 2. Run /graph-node-up $HASH/<node> from your home folder. 3. Record the pod in your meta entry: agent-gossip meta merge --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --merge '{\"peers\":{\"<your nickname>\":{\"pod\":\"$INSTANCE\",\"graph\":\"$GRAPH\",\"hash\":\"$HASH\",\"node\":\"<id>\",\"invited_by\":\"$INVITER\",\"status\":\"busy\"}}}'. 4. Read agent-gossip meta get: the members of the pod are the entries with pod $INSTANCE and hash $HASH that are not in absent. They are the peers at the ends of your edges. 5. Send the artifact: loaded <node>. If /graph-node-up fails, decline with its error line."
```

Track each offer as a task, per the **Task tracking** rules of the gossip
skill.
