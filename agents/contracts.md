# Contract registry

Contracts are shared interfaces. The architect owns canonical sources and
their mirrors. Workers read contracts and never edit them.

| Contract | Canonical | Mirror | Consumers | Owner |
|---|---|---|---|---|
| SAB ABI v2 | `crates/sim-types/src/abi.rs` | `web/src/types/asset-layout.ts` | `sim-worker`, `web/src/sim/sab.ts`, `web/src/input` | architect |
| Asset schema v4 / pack format | `crates/sim-types/src/asset.rs` | `web/src/assets/types.ts` | `sim-asset`, `asset-compiler`, `sim-render`, `web/src/assets` | architect |
| Car tuning | `crates/sim-types/src/setup.rs` | `lapster/setup@1` schema tag in `web/src/ui/car-setup.ts` | `asset-compiler`, `sim-worker`, `sim-render`, `web/src/ui` | architect |
| Net protocol | `crates/relay-server/src/protocol.rs` | `web/src/net/protocol.ts` | `relay-server`, `web/src/net` | architect |
| Core public API | `crates/sim-core/src/lib.rs` re-exports | — | `sim-worker`, `asset-compiler` tests | physics |

## Rules

1. Only the architect edits canonical contracts and mirrors.
2. Exception: the isolated spike agent may edit contracts and mirrors in a
   `spike/<name>` worktree to test feasibility. Those edits are never pushed
   or merged; promotion goes through the strict workflow.
3. A worker that needs a contract change returns `BLOCKED` with
   `contract_change`.
4. Update the canonical source first, then every mirror, in the same change.
5. Bump the relevant version when the ABI or schema changes:
   - `SAB_LAYOUT_VERSION` in `crates/sim-types/src/abi.rs` and
     `web/src/types/asset-layout.ts`.
   - `SCHEMA_VERSION` in `crates/sim-types/src/asset.rs`.
   - `SETUP_SCHEMA` in `web/src/ui/car-setup.ts` when the setup JSON shape
     changes.
6. Run the workspace gate after a contract change.
7. A breaking contract change requires an ADR in `docs/adr/`. Reference it in
   the contract registry or PR.

## Mirror sync

The following pairs must stay identical in structure and field offsets:

- `crates/sim-types/src/abi.rs` ↔ `web/src/types/asset-layout.ts`
- `crates/relay-server/src/protocol.rs` ↔ `web/src/net/protocol.ts`
- `crates/sim-types/src/asset.rs` ↔ `web/src/assets/types.ts`

If a mirror drifts, stop and fix it before delegating.
