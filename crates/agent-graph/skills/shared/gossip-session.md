## Gossip

Find the gossip of this session:

```bash
agent-gossip session --session-pid "$PPID"
```

In `agy`, `$CONVERSATION` is not a shell variable. Write the conversation id
from your `<user_information>` in its place. If you have no conversation id,
run `agent-gossip session` with no flag. It finds the agent process itself.
If `agent-gossip` rejects `--session`, print this line, then stop:

```text
graph · agent-gossip too old · run agent-gossip plug after an upgrade
```

If the `sessions` list of the output is empty, there is no session. Print
this line, then stop:

```text
graph · no gossip · create or join a gossip first
```

If `sessions` has more than 1 entry, this agent is in more than one gossip.
Print this line, then stop. Do not pick an entry:

```text
graph · many gossips · leave all but one gossip, then run this skill again
```

Else hold `$GOSSIP`, `$NICKNAME`, and `$PUBKEY` (`pubkey`) from the one entry
of `sessions`. If it has no `pubkey`, print this line, then stop. An empty
`$PUBKEY` finds no own entry:

```text
graph · no session key · update agent-gossip, then start the session again
```
