---
default: major
---

# Prefix every skill with graph-

The skills `/node-up`, `/node-down`, `/node-list`, `/node-invite`, and `/trait-run` are now `/graph-node-up`, `/graph-node-down`, `/graph-node-list`, `/graph-node-invite`, and `/graph-trait-run`. The old slash commands stop working.

Run `agent-graph plug` again after the upgrade. It removes the old skill folders and installs the new ones.
