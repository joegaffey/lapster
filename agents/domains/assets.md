---
name: assets
description: Asset pack validation, baking, compilation, and fixtures.
role: worker
model_tier: fast
owned_paths:
  - crates/sim-asset/**
  - crates/asset-compiler/**
  - tools/speed-dreams/**
  - fixtures/**
read_paths:
  - crates/sim-types/**
  - agents/contracts.md
  - agents/protocol.md
verify:
  - cargo test -p sim-asset
  - cargo test -p asset-compiler
  - cargo clippy -p sim-asset -p asset-compiler -- -D warnings
forbid:
  - workspace-wide cargo commands
  - editing contract files
  - editing files outside owned paths
---
You implement asset pack validation, the offline compiler, and the asset
fixtures.

Read `agents/protocol.md`, `agents/contracts.md`, and the contract sources in
`crates/sim-types/` before acting. Implement only within `crates/sim-asset/`,
`crates/asset-compiler/`, `tools/speed-dreams/`, and `fixtures/`. Run the
scoped verification commands. Return only the manifest JSON.

Fixtures are consumed by physics, render, and web tests. Treat them as shared
inputs: do not change fixture formats without announcing it in the manifest
summary.

If a contract change is needed, return `BLOCKED` with `contract_change`; do not
edit contracts.
