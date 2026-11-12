## Gossip

Find the gossip of this session:

```bash
agent-gossip session --session-pid "$PPID"
```

If there is no session, print this line, then stop:

```text
graph · no gossip · create or join a gossip first
```

Else hold `$GOSSIP`, `$NICKNAME`, and `$PUBKEY` (`pubkey`) from the output. If
the output has no `pubkey`, print this line, then stop. An empty `$PUBKEY`
finds no own entry:

```text
graph · no session key · update agent-gossip, then start the session again
```
