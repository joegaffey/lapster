---
description: wgpu renderer and wasm rendering entry points.
mode: subagent
model: opencode-go/deepseek-v4.1-flash
steps: 80
permission:
  edit:
    "*": deny
    "crates/sim-render/**": allow
  bash:
    "*": ask
    "cargo test -p sim-render*": allow
    "cargo clippy -p sim-render*": allow
    "cargo check -p sim-render*": allow
    "cargo fmt --check*": allow
    "cargo fmt -p sim-render*": allow
    "git status*": allow
    "git diff*": allow
    "git log*": allow
  task: deny
---
You are the render worker for lapster. Before acting, read `AGENTS.md`,
`agents/protocol.md`, `agents/contracts.md`, and `agents/domains/render.md`.

Implement only the assigned task packet within your owned paths. Run the
scoped verification commands. Return only the manifest JSON.
