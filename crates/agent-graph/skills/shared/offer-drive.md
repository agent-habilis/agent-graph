## Drive

Handle each event per the **Receive loop** and **Event handling** sections of
the gossip skill, with these rules for node offers:

- **A task brief `send graph <hash>`:** if `<hash>` is `$HASH` and the peer
  has an open offer of yours, the task serves your offer. Accept it without
  the accept or decline question. Else decline it with the reason: no offer
  for this graph. Pack the graph with `agent-graph graph pack` and send the
  file as the result. `$SOURCE` names a local graph, or a received one.

  Run each line as its own command:

  ```bash
  agent-gossip a2a status --gossip "$GOSSIP" --nickname "$NICKNAME" --task-id "<task id>" --state working
  agent-graph graph pack "$SOURCE" --output /tmp/$HASH.tar
  agent-gossip a2a artifact --gossip "$GOSSIP" --nickname "$NICKNAME" --task-id "<task id>" --file /tmp/$HASH.tar --text "graph $HASH"
  ```

  When the peer approves, close that task with `--state completed`.
- **A task brief `send node <hash>/<node>`:** a peer without agent-graph asks
  for the context of the graph and of its node. If `<hash>` is `$HASH` and the
  peer has an open offer of yours for `<node>`, accept it without the accept or
  decline question. Else decline it with the reason: no offer for this node.
  Send the text that `agent-graph graph node-up` writes to a file. Its SHA-256
  is the `$CONTEXT_HASH` of your offer.

  Run each line as its own command:

  ```bash
  agent-gossip a2a status --gossip "$GOSSIP" --nickname "$NICKNAME" --task-id "<task id>" --state working
  agent-graph graph node-up "$SOURCE/<node>" --output /tmp/$HASH-<node>.md
  agent-gossip a2a artifact --gossip "$GOSSIP" --nickname "$NICKNAME" --task-id "<task id>" --file /tmp/$HASH-<node>.md --text "node $HASH/<node>"
  ```

  When the peer approves, close that task with `--state completed`.
- **Artifact `loaded <node>`:** approve it with a follow-up that carries
  `--task-id`. The new peer wrote its own meta entry. Write nothing.
- **`failed` or `task_timeout`:** the place is open again.
