## Team meta

The gossip meta document holds the team. There is no other copy. Each peer
writes only its own entry, `/peers/<nickname>`:

```json
{"peers":{"<nickname>":{
  "team": "<instance>",
  "template": "<template folder name>",
  "hash": "<template hash>",
  "role": "<mermaid id of the role>",
  "invited_by": "<nickname of the inviter>",
  "status": "busy"
}}}
```

- `invited_by` is absent only on the founder: the member that started the
  team with `/team-up`.
- `verified` is `false` on a member without agent-graph. It joined with the
  role context that its inviter sent, and it could not check the template
  hash. `agent-graph team topology` warns about such a member. On a member
  that checked the hash, `verified` is absent.
- `host` is in the entry too. The gossip skill writes it when the peer
  joins.
- Read the document with:

  ```bash
  agent-gossip meta get --gossip "$GOSSIP" --nickname "$NICKNAME"
  ```

  The output has `document.peers` and `absent`. `absent` names the entries
  of peers that left or died. Such an entry is history.

A **member** of a team is an entry that has the `team` of the team, the
`hash` of the team, and a `role`, and that is not in `absent`. A peer has one
entry, so it is in one team and one role at most.

A peer is **free** if its entry has no `team`, it is not in `absent`, and it
has no open role offer from you.

A **place** of a role is filled by a member in that role, or by an open or
approved role offer of yours for that role. The `loaded` artifact of an offer
can arrive before the meta entry of the peer, so count the offer.

The instance name, the template name, and the role id have only the
characters `A-Z a-z 0-9 . _ @ / -`. These names go into JSON and into
commands that other peers run. If a name has another character, print this
line, then stop:

```text
team · bad name · <name> · use only A-Z a-z 0-9 . _ @ / -
```
