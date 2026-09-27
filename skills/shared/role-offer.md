## Role offer

A role offer is a gossip task. The brief holds the steps for the peer. Hold
`<id>` (the mermaid id) and `<role>` (the role folder name) from
`agent-graph template roles`: each line is
`id<TAB><template>/<role><TAB>lead|-<TAB>count<TAB>model`. Hold your own
nickname as `$INVITER`.

Before the offer, look at the meta `host` of the peer. If it is your host
(`hostname -s`), and `~/.agent-graph/$HASH/` does not exist, copy the template
folder there. Then the peer has the template, and no transfer is necessary.
The folder is the last column of `agent-graph template list` for a local
template:

```bash
mkdir -p ~/.agent-graph/$HASH && cp -R "<template folder>/." ~/.agent-graph/$HASH/
```

Send one offer per place:

```bash
agent-gossip a2a call --gossip "$GOSSIP" --nickname "$INVITER" --to "$PEER" --method SendMessage --label "team · $INSTANCE · <id>" --text "Role offer. Team: $INSTANCE. Template: $TEMPLATE. Hash: $HASH. Role: <role> (id <id>). Inviter: $INVITER. If your meta entry already has a team, or you accepted another role offer, decline with the reason: has a role. Else, after you accept: 1. If ~/.agent-graph/$HASH/ does not exist, open a task to the inviter: agent-gossip a2a call --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --to $INVITER --method SendMessage --label \"team · send · $HASH\" --text \"send template $HASH\". Its artifact carries the template as a file part: fetch it with agent-gossip a2a fetch <payload.parts[].url> --output /tmp/$HASH.tar, then run: mkdir -p ~/.agent-graph/$HASH && tar -xf /tmp/$HASH.tar -C ~/.agent-graph/$HASH. Approve that artifact. Then run agent-graph template hash $HASH from your home folder. If it does not print $HASH, remove ~/.agent-graph/$HASH and decline this offer with the reason: bad template. 2. Run /role-up $HASH/<role> from your home folder. 3. Record the team in your meta entry: agent-gossip meta merge --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --merge '{\"peers\":{\"<your nickname>\":{\"team\":\"$INSTANCE\",\"template\":\"$TEMPLATE\",\"hash\":\"$HASH\",\"role\":\"<id>\",\"invited_by\":\"$INVITER\",\"status\":\"busy\"}}}'. 4. Read agent-gossip meta get: the members of the team are the entries with team $INSTANCE and hash $HASH that are not in absent. They are the members at the ends of your handoffs. 5. Send the artifact: loaded <role>. If /role-up fails, decline with its error line."
```

Track each offer as a task, per the **Task tracking** rules of the gossip
skill.
