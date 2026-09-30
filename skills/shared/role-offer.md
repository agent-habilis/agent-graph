## Role offer

A role offer is a gossip task. The brief holds the steps for the peer. Hold
`<id>` (the mermaid id) and `<role>` (the role folder name) from
`agent-graph template roles`: each line is
`id<TAB><template>/<role><TAB>lead|-<TAB>count<TAB>model`. Hold your own
nickname as `$INVITER`.

Hold the template name as `$SOURCE`. If `agent-graph template hash
"$TEMPLATE"` prints `$HASH`, `$SOURCE` is `$TEMPLATE`. Else `$SOURCE` is
`$HASH`, and you run the `agent-graph` commands of this section and of the
**Drive** section from your home folder, where `~/.agent-graph/$HASH` is.

A peer without agent-graph cannot load a role from the template. It asks you
for the role context: the text that `agent-graph role up` prints, the body of
the template and then the body of the role. It checks that text with a hash.
Compute that hash before the offer:

```bash
CONTEXT_HASH=$(agent-graph role up "$SOURCE/<role>" | shasum -a 256 | cut -d' ' -f1)
```

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
agent-gossip a2a call --gossip "$GOSSIP" --nickname "$INVITER" --to "$PEER" --method SendMessage --label "team · $INSTANCE · <id>" --text "Role offer. Team: $INSTANCE. Template: $TEMPLATE. Hash: $HASH. Context hash: $CONTEXT_HASH. Role: <role> (id <id>). Inviter: $INVITER. If your meta entry already has a team, or you accepted another role offer, decline with the reason: has a role. Else, after you accept, do A if the command agent-graph is on your PATH (command -v agent-graph), else do B. A1. If ~/.agent-graph/$HASH/ does not exist, open a task to the inviter: agent-gossip a2a call --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --to $INVITER --method SendMessage --label \"team · send · $HASH\" --text \"send template $HASH\". Its artifact carries the template as a file part: fetch it with agent-gossip a2a fetch <payload.parts[].url> --output /tmp/$HASH.tar, then run: mkdir -p ~/.agent-graph/$HASH && tar -xf /tmp/$HASH.tar -C ~/.agent-graph/$HASH. Approve that artifact. Then run agent-graph template hash $HASH from your home folder. If it does not print $HASH, remove ~/.agent-graph/$HASH and decline this offer with the reason: bad template. A2. Run /role-up $HASH/<role> from your home folder. If you do not have the /role-up skill, run agent-graph role up $HASH/<role> --pid \"\$PPID\" from your home folder, act as the role that it prints, and obey each ## Boundaries section in it. A3. Record the team in your meta entry: agent-gossip meta merge --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --merge '{\"peers\":{\"<your nickname>\":{\"team\":\"$INSTANCE\",\"template\":\"$TEMPLATE\",\"hash\":\"$HASH\",\"role\":\"<id>\",\"invited_by\":\"$INVITER\",\"status\":\"busy\"}}}'. B1. Open a task to the inviter: agent-gossip a2a call --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --to $INVITER --method SendMessage --label \"team · send · $HASH/<role>\" --text \"send role $HASH/<role>\". Its artifact carries the context of the team and of your role as a file part: fetch it with agent-gossip a2a fetch <payload.parts[].url> --output /tmp/$HASH-<id>.md. Approve that artifact. B2. Run shasum -a 256 /tmp/$HASH-<id>.md (or sha256sum). If its first field is not $CONTEXT_HASH, decline this offer with the reason: bad role context. B3. Read /tmp/$HASH-<id>.md: it is the context of your team and of your role. Act as that role, as a member of the team, and obey each ## Boundaries section in it. B4. Record the team in your meta entry. You could not check the template hash, so mark it as not verified: agent-gossip meta merge --gossip \"\$GOSSIP\" --nickname \"\$NICKNAME\" --merge '{\"peers\":{\"<your nickname>\":{\"team\":\"$INSTANCE\",\"template\":\"$TEMPLATE\",\"hash\":\"$HASH\",\"role\":\"<id>\",\"invited_by\":\"$INVITER\",\"verified\":false,\"status\":\"busy\"}}}'. Then, after A or B: read agent-gossip meta get: the members of the team are the entries with team $INSTANCE and hash $HASH that are not in absent. They are the members at the ends of your handoffs. Send the artifact: loaded <role>. If a step fails, decline with its error line."
```

Track each offer as a task, per the **Task tracking** rules of the gossip
skill.
