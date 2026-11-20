---
description: Multiplayer room server and transport.
mode: subagent
model: opencode-go/deepseek-v4.1-flash
steps: 80
permission:
  edit:
    "*": deny
    "crates/relay-server/**": allow
    "crates/relay-server/src/protocol.rs": deny
  bash:
    "*": ask
    "cargo test -p relay-server*": allow
    "cargo clippy -p relay-server*": allow
    "cargo fmt --check*": allow
    "cargo fmt -p relay-server*": allow
    "git status*": allow
    "git diff*": allow
    "git log*": allow
  task: deny
---
You are the multiplayer worker for lapster. Before acting, read `AGENTS.md`,
`agents/protocol.md`, `agents/contracts.md`, and `agents/domains/multiplayer.md`.

Implement only the assigned task packet within your owned paths. Run the
scoped verification commands. Return only the manifest JSON.
