---
description: Asset pack validation, baking, compilation, and fixtures.
mode: subagent
model: opencode-go/deepseek-v4.1-flash
steps: 80
permission:
  edit:
    "*": deny
    "crates/sim-asset/**": allow
    "crates/asset-compiler/**": allow
    "crates/track-pack/**": allow
    "crates/track-pack-wasm/**": allow
    "tools/speed-dreams/**": allow
    "fixtures/**": allow
  bash:
    "*": ask
    "cargo test -p sim-asset*": allow
    "cargo test -p asset-compiler*": allow
    "cargo test -p track-pack*": allow
    "cargo clippy -p sim-asset*": allow
    "cargo clippy -p asset-compiler*": allow
    "cargo clippy -p track-pack*": allow
    "cargo fmt --check*": allow
    "cargo fmt -p sim-asset*": allow
    "cargo fmt -p asset-compiler*": allow
    "cargo fmt -p track-pack*": allow
    "cargo fmt -p track-pack-wasm*": allow
    "node tools/speed-dreams/*": allow
    "bash tools/speed-dreams/*": allow
    "npm --prefix tools/speed-dreams *": allow
    "git status*": allow
    "git diff*": allow
    "git log*": allow
  task: deny
---
You are the assets worker for lapster. Before acting, read `AGENTS.md`,
`agents/protocol.md`, `agents/contracts.md`, and `agents/domains/assets.md`.

Implement only the assigned task packet within your owned paths. Run the
scoped verification commands. Return only the manifest JSON.
