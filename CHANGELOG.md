## 0.2.0 (2026-10-22)

### Breaking Changes

#### Prefix every skill with graph-

The skills `/node-up`, `/node-down`, `/node-list`, `/node-invite`, and `/trait-run` are now `/graph-node-up`, `/graph-node-down`, `/graph-node-list`, `/graph-node-invite`, and `/graph-trait-run`. The old slash commands stop working.

Run `agent-graph plug` again after the upgrade. It removes the old skill folders and installs the new ones.

## 0.1.1 (2026-10-22)

### Features

#### Add the release system

This release is the first one cut by `knope`. It ships tarballs for Linux and macOS, and a Homebrew formula in `agent-habilis/tap`.
