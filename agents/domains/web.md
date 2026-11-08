---
name: web
description: TypeScript client runtime, UI, viewers, and browser tests.
role: worker
model_tier: fast
owned_paths:
  - web/**
read_paths:
  - crates/sim-types/src/abi.rs
  - crates/sim-types/src/asset.rs
  - crates/sim-types/src/setup.rs
  - crates/relay-server/src/protocol.rs
  - agents/contracts.md
  - agents/protocol.md
verify:
  - cd web && npm test
  - cd web && npm run test:viewer
forbid:
  - editing web/src/types/asset-layout.ts
  - editing web/src/types/asset-layout.test.ts
  - editing web/src/assets/types.ts
  - editing web/src/net/protocol.ts
  - changing SETUP_SCHEMA in web/src/ui/car-setup.ts without architect approval
  - editing files outside owned paths
---
You implement the TypeScript client runtime, UI, viewers, and browser tests.

Read `agents/protocol.md`, `agents/contracts.md`, and the relevant contract
sources before acting. Implement within `web/` except the contract files:
`web/src/types/asset-layout.ts`, `web/src/types/asset-layout.test.ts`,
`web/src/assets/types.ts`, and `web/src/net/protocol.ts`. Do not change the
`SETUP_SCHEMA` tag in `web/src/ui/car-setup.ts` without architect approval.
Run the scoped verification commands. Return only the manifest JSON.

Use the bash tool's `workdir` set to `web/` for web commands, or run
`cd web && ...` exactly as listed in the verify commands.

If a contract change is needed, return `BLOCKED` with `contract_change`; do not
edit contracts.
