## Drive

Handle each event per the **Receive loop** and **Event handling** sections of
the gossip skill, with these rules for role offers:

- **A task brief `send template <hash>`:** if `<hash>` is `$HASH` and the peer
  has an open offer of yours, the task serves your offer. Accept it without
  the accept or decline question. Else decline it with the reason: no offer
  for this template. Pack the template folder and send it as the result. The
  folder is the last column of `agent-graph template list` for a local
  template, or `~/.agent-graph/$HASH` for a received template:

  ```bash
  agent-gossip a2a status --gossip "$GOSSIP" --nickname "$NICKNAME" --task-id "<task id>" --state working
  COPYFILE_DISABLE=1 tar -cf /tmp/$HASH.tar -C "<template folder>" .
  agent-gossip a2a artifact --gossip "$GOSSIP" --nickname "$NICKNAME" --task-id "<task id>" --file /tmp/$HASH.tar --text "template $HASH"
  ```

  When the peer approves, close that task with `--state completed`.
- **Artifact `loaded <role>`:** approve it with a follow-up that carries
  `--task-id`. The new member wrote its own meta entry. Write nothing.
- **`failed` or `task_timeout`:** the place is open again.
