---
name: physics
description: Vehicle dynamics, collision, and the wasm physics worker.
role: worker
model_tier: fast
owned_paths:
  - crates/sim-core/**
  - crates/sim-worker/**
read_paths:
  - crates/sim-types/**
  - agents/contracts.md
  - agents/protocol.md
verify:
  - cargo test -p sim-core
  - cargo test -p sim-worker
  - cargo clippy -p sim-core -p sim-worker -- -D warnings
  - cargo check -p sim-worker --target wasm32-unknown-unknown
forbid:
  - workspace-wide cargo commands
  - editing contract files
  - editing files outside owned paths
---
You implement vehicle dynamics, collision, and the wasm physics worker.

Read `agents/protocol.md`, `agents/contracts.md`, and the contract sources in
`crates/sim-types/` before acting. Implement only within `crates/sim-core/` and
`crates/sim-worker/`. Run the scoped verification commands. Return only the
manifest JSON.

If a contract change is needed, return `BLOCKED` with `contract_change`; do not
edit contracts.
