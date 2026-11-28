## Node offer

A node offer is a gossip task. The brief holds the steps for the peer. Hold
`<id>` (the mermaid id) and `<node>` (the node folder name) from `agent-graph
graph nodes`: each line is
`id<TAB><graph>/<node><TAB>lead|-<TAB>count<TAB>model`. Write `yes` for `Lead:`
in the brief if the third column is `lead`, else `no`. Hold your own nickname as
`$INVITER`.

Hold the graph name as `$SOURCE`. If `agent-graph graph hash
"$GRAPH"` prints `$HASH`, `$SOURCE` is `$GRAPH`. Else `$SOURCE` is
`$HASH`, and you add `--dir ~` to each `agent-graph` command of this section
and of the **Drive** section, so that discovery starts in your home folder,
where `~/.agent-graph/graphs/$HASH` is.

A peer without agent-graph cannot load a node from the graph. It asks you for
the node context: the text that `agent-graph graph node-up` prints, the body of
the graph and then the body of the node. It checks that text with a hash.
Compute that hash before the offer. The command prints it. Hold it as
`$CONTEXT_HASH`:

```bash
agent-graph graph node-hash "$SOURCE/<node>"
```

Before the offer, look at the meta `host` of the peer. If it is your host
(`hostname -s`), and `~/.agent-graph/graphs/$HASH/` does not exist, copy the
graph folder there. Then the peer has the graph, and no transfer is necessary.
The folder is the last column of `agent-graph graph list` for a local graph.
The command prints `$HASH`:

```bash
agent-graph graph install "<graph folder>"
```

Send one offer per place:

```bash
agent-gossip a2a call --gossip "$GOSSIP" --nickname "$INVITER" --to "$PEER" --method SendMessage --label "graph · $INSTANCE · <id>" --text "Node offer. Instance: $INSTANCE. Graph: $GRAPH. Hash: $HASH. Context hash: $CONTEXT_HASH. Node: <node> (id <id>). Lead: <yes or no>. Inviter: $INVITER. In A3 and B4, replace <leader> with \"leader\":true, if Lead is yes, else with nothing. If your meta entry (document[\$PUBKEY], with \$PUBKEY from agent-gossip session --session-pid \"\$PPID\") already has an instance, or you accepted another node offer, decline with the reason: has a node. Else, after you accept, do A if the command agent-graph is on your PATH (command -v agent-graph), else do B. If agent-graph graph install is an unknown command, or --pid auto is not accepted, decline with the reason: agent-graph too old, run agent-graph plug after an upgrade. In agy, run each command of these steps with BypassSandbox true if run_command has that parameter: each agent-graph command, each agent-gossip command, and each command that reads or writes under ~/.agent-graph. Without it the command already runs outside the sandbox. In agy, write --pid auto in place of --pid \"\$PPID\". In agy, \$CONVERSATION is not a shell variable: write your conversation id in its place. In agy, run agent-gossip session --session \$CONVERSATION in place of agent-gossip session --session-pid \"\$PPID\", or agent-gossip session with no flag if you have no conversation id. If agent-gossip rejects --session, decline with the reason: agent-gossip too old, run agent-gossip plug after an upgrade. A1. If ~/.agent-graph/graphs/$HASH/ does not exist, open a task to the inviter: agent-gossip a2a call --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --to $INVITER --method SendMessage --label \"graph · send · $HASH\" --text \"send graph $HASH\". Its artifact carries the graph as a file part, and <file url> is the url of that part (payload.parts[].url): fetch it with agent-gossip a2a fetch <file url> --output /tmp/$HASH.tar, then run: agent-graph graph install --tar /tmp/$HASH.tar --expect $HASH. Approve that artifact. If the install fails, decline this offer with the reason: bad graph. A2. Run agent-graph graph node-up $HASH/<node> --dir ~ --name $INSTANCE --pid \"\$PPID\", act as the node that it prints, and obey each ## Boundaries section in it. A3. Record the graph in your meta entry: agent-gossip meta merge --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --merge '{\"instance\":\"$INSTANCE\",\"graph\":\"$GRAPH\",\"hash\":\"$HASH\",\"node\":\"<id>\",<leader>\"invited_by\":\"$INVITER\",\"joined_at\":<epoch seconds>,\"status\":\"busy\"}'. Replace <epoch seconds> with the number that the command date +%s prints. Run that command on its own, before the merge. B1. Open a task to the inviter: agent-gossip a2a call --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --to $INVITER --method SendMessage --label \"graph · send · $HASH/<node>\" --text \"send node $HASH/<node>\". Its artifact carries the context of the graph and of your node as a file part, and <file url> is the url of that part (payload.parts[].url): fetch it with agent-gossip a2a fetch <file url> --output /tmp/$HASH-<id>.md. Approve that artifact. B2. Run shasum -a 256 /tmp/$HASH-<id>.md (or sha256sum). If its first field is not $CONTEXT_HASH, decline this offer with the reason: bad node context. B3. Read /tmp/$HASH-<id>.md: it is the context of your graph and of your node. Act as that node, as a peer of the graph, and obey each ## Boundaries section in it. B4. Record the graph in your meta entry. You could not check the graph hash, so mark it as not verified: agent-gossip meta merge --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --merge '{\"instance\":\"$INSTANCE\",\"graph\":\"$GRAPH\",\"hash\":\"$HASH\",\"node\":\"<id>\",<leader>\"invited_by\":\"$INVITER\",\"joined_at\":<epoch seconds>,\"verified\":false,\"status\":\"busy\"}'. Replace <epoch seconds> with the number that the command date +%s prints. Run that command on its own, before the merge. Then, after A or B: read agent-gossip meta get: the peers of the graph are the entries of document with instance $INSTANCE and hash $HASH whose keys are not in absent. The nickname of a peer is the field nickname of its entry. They are the peers at the ends of your edges. If a broadcast later says graph $INSTANCE is down, follow it: leave your node and clear your meta entry. Send the artifact: loaded <node>. If a step fails, decline with its error line."
```

Track each offer as a task, per the **Task tracking** rules of the gossip
skill.
