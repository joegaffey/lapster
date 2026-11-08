# lapster — multi-agent development

This repository uses a contract-bound multi-agent workflow. Read this file,
then the canonical layer under `agents/`.

## Read order

1. `agents/contracts.md` — canonical contracts and mirrors.
2. `agents/protocol.md` — task packet, manifest, lifecycle.
3. Your role card:
   - Architect: `agents/architect.md`
   - Worker: `agents/domains/<domain>.md`

## Topology

```text
architect (primary)
  -> contracts (architect-owned)
    -> physics | render | assets | multiplayer | web (short-lived workers)
  -> reviewer (optional, read-only)
```

- **Architect** owns contracts, mirrors, docs, workspace/build/CI config, and
  delegation. It does not edit domain implementation.
- **Workers** are short-lived: one task, one session, then terminated. They
  read contracts, never edit them, and return a manifest only.

## Contract registry

| Contract | Canonical | Mirror | Owner |
|---|---|---|---|
| SAB ABI v2 | `crates/sim-types/src/abi.rs` | `web/src/types/asset-layout.ts` | architect |
| Asset schema v3 / pack format | `crates/sim-types/src/asset.rs` | `web/src/assets/types.ts` | architect |
| Car tuning | `crates/sim-types/src/setup.rs` | `lapster/setup@1` schema tag in `web/src/ui/car-setup.ts` | architect |
| Net protocol | `crates/relay-server/src/protocol.rs` | `web/src/net/protocol.ts` | architect |
| Core public API | `crates/sim-core/src/lib.rs` re-exports | — | physics |

Full detail: `agents/contracts.md`.

## Domains

| Worker | Owned paths | Verify |
|---|---|---|
| `physics` | `crates/sim-core/**`, `crates/sim-worker/**` | `cargo test -p sim-core -p sim-worker`, clippy, wasm check |
| `render` | `crates/sim-render/**` | `cargo test -p sim-render`, clippy, wasm check |
| `assets` | `crates/sim-asset/**`, `crates/asset-compiler/**`, `tools/speed-dreams/**`, `fixtures/**` | `cargo test -p sim-asset -p asset-compiler`, clippy |
| `multiplayer` | `crates/relay-server/**` except `protocol.rs` | `cargo test -p relay-server`, clippy |
| `web` | `web/**` except the three contract files and generated outputs | `cd web && npm test && npm run test:viewer` |

## Review

`reviewer` is an optional read-only subagent. The architect may dispatch it
after workers and before commit for high-risk batches. It returns findings by
severity; the architect decides. See `agents/reviewer.md`.

## Context protocol

Architect sends one task packet per worker. Workers return only a manifest.

Task packet:

```text
DOMAIN
GOAL
CONTRACTS (read-only)
OWNED PATHS
VERIFY
ACCEPTANCE
FORBIDDEN
RETURN: manifest JSON only
```

Manifest:

```json
{
  "status": "COMPLETED | FAILED | BLOCKED",
  "domain": "<domain>",
  "contracts": ["<path>"],
  "contract_change": null,
  "tests": { "cmd": "<command>", "passed": true },
  "files": ["<path>"],
  "summary": "<1-2 sentences>"
}
```

Full detail: `agents/protocol.md`.

## Durable state

`PROGRESS.md`, `SPEC.md`, `PLAN.md`, and `README.md` are the resumable project
state. Do not rely on chat history.

## Branching and gate

- `main` is protected: PR + `gate` CI + architect review comment + @joegaffey
  approval.
- `dev` is the integration branch: the architect may push directly and merge
  PRs.
- `feat/<name>` branches: the architect pushes and opens a PR to `dev` (or
  `main` for hotfixes).
- Workers never commit or push. The architect commits after the full gate.

The architect runs the full gate once per batch and posts a PR comment. See
`agents/architect.md`.

## Commands

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p sim-types -p sim-asset -p sim-core -p sim-render -p sim-worker \
  --target wasm32-unknown-unknown

cd web
npm test
npm run test:viewer
npm run bench:physics
```

## Context exclusions

Never read: `target/`, `web/node_modules/`, `tools/speed-dreams/node_modules/`,
`web/pkg/`, `web/.cache/`, `dist/`, `fixtures/real/`, `web/test-results/`,
`web/playwright-report/`, `**/*.kn5`, `**/*.log`, `.env*`,
`web/src/version.generated.ts`, `web/version.json`, generated
`web/{drive,garage,multiplayer,track-viewer,style-spike}/main.js`,
`web/worker/physics-worker.js`, `web/audio/combustion-worklet.js`,
`Cargo.lock`, `package-lock.json`.

## Tooling

Only opencode has committed adapters: `.opencode/agents/*.md` and
`opencode.json`. Other assistants use this file and `agents/` directly.

Portable pattern:
https://gist.github.com/joegaffey/27366c537daf1b9e5799e2c71bdac4f7
