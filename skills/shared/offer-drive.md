## Drive

Handle each event per the **Receive loop** and **Event handling** sections of
the gossip skill, with these rules for node offers:

- **A task brief `send graph <hash>`:** if `<hash>` is `$HASH` and the peer
  has an open offer of yours, the task serves your offer. Accept it without
  the accept or decline question. Else decline it with the reason: no offer
  for this graph. Pack the graph folder and send it as the result. The folder
  is the last column of `agent-graph graph list` for a local graph, or
  `~/.agent-graph/$HASH` for a received graph:

  ```bash
  agent-gossip a2a status --gossip "$GOSSIP" --nickname "$NICKNAME" --task-id "<task id>" --state working
  COPYFILE_DISABLE=1 tar -cf /tmp/$HASH.tar -C "<graph folder>" .
  agent-gossip a2a artifact --gossip "$GOSSIP" --nickname "$NICKNAME" --task-id "<task id>" --file /tmp/$HASH.tar --text "graph $HASH"
  ```

  When the peer approves, close that task with `--state completed`.
- **Artifact `loaded <node>`:** approve it with a follow-up that carries
  `--task-id`. The peer wrote its own meta entry. Write nothing.
- **`failed` or `task_timeout`:** the place is open again.
