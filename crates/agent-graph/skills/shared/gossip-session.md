## Gossip

Find the gossip of this session:

```bash
agent-gossip session --session-pid "$PPID"
```

Hold `$GOSSIP`, `$NICKNAME`, and `$PUBKEY` (`pubkey`) from the output. If there is no session, print
this line, then stop:

```text
graph · no gossip · create or join a gossip first
```
