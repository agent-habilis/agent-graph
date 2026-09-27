# skills/ — sources, not what ships

These are the sources of the agent skills that the binary embeds.
`build.rs` renders them into one self-contained `SKILL.md` per skill, in
`$OUT_DIR/skills`. `agent-graph plug` installs that rendered tree. An
installed skill never tells the agent to read a second file.

This is the pattern of agent-gossip (`skills/README.md` there). The renderer
is the `slot-template` crate, copied from agent-gossip.

Rules:

- **`skills/<prefix>-*/SKILL.md` is the source and the only emitted file.**
  Only folders that start with `template-`, `role-`, or `team-` are skills.
  `shared/` holds the partials, and it is never emitted.
- The directives are HTML comments, so a source stays valid markdown:

  ```markdown
  <!-- include path="../shared/role-offer.md" inviter="{NICKNAME}" -->
  <!-- slot name="inviter" -->
  ```

  `include` (alone on its line) splices the file at `path`, relative to the
  file that holds the line, with exactly the other keys as its args. `slot`
  is the point that such an arg fills. `\"` and `\\` are the only escapes.
- The render is strict: a slot with no arg, an arg with no slot, and a
  malformed directive stop the build.
- Partials use `##`/`###` headings. Refer to another part by its section
  name ("see the **Drive** section"), never by a file path.
- A skill that runs in a gossip names the sections of the gossip skill that
  joined it (**Receive loop**, **Decisions**, **Event handling**). It does
  not copy them: agent-gossip owns that text.
