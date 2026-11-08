# Agent protocol

## Roles

- **Architect** (primary): owns contracts, mirrors, docs, workspace/build/CI
  config. Delegates implementation. Verifies results.
- **Worker** (subagent): owns one domain. Short-lived. One task per session.
- Optional reviewer: see `agents/review-protocol.md`.
- Optional spike: see `agents/spike-protocol.md`.

## Task packet

The architect sends exactly one packet per worker task:

```text
DOMAIN: <domain>
GOAL: <one concern>
CONTRACTS (read-only): <paths>
OWNED PATHS: <paths>
VERIFY: <exact commands>
ACCEPTANCE: <observable criteria>
FORBIDDEN: <actions>
RETURN: manifest JSON only
```

## Manifest

The worker returns only this JSON object:

```json
{
  "status": "COMPLETED | FAILED | BLOCKED",
  "domain": "<domain>",
  "contracts": ["<path>"],
  "contract_change": null,
  "tests": { "cmd": "<command>", "passed": true },
  "files": ["<path>"],
  "summary": "<1-2 sentences>",
  "debt": [
    { "area": "<domain>", "summary": "<...>", "severity": "P0 | P1 | P2 | P3", "reason": "<...>" }
  ]
}
```

`debt` is optional. Workers identify debt; only the architect records it in
`docs/tech-debt.md`.

Contract change request:

```json
{
  "contract_change": {
    "canonical": "<path>",
    "mirrors": ["<path>"],
    "reason": "<why>",
    "breaking": true
  }
}
```

Rules:

- No logs, diffs, or full file contents in the manifest.
- `status` is `BLOCKED` when a contract change or human decision is required.
- `tests.passed` must reflect the actual scoped verification.

## Lifecycle

1. Architect updates canonical contracts and mirrors first, if needed.
2. Architect sends one task packet.
3. Worker reads contracts and its role card, implements, verifies.
4. Worker returns the manifest and terminates. The session is not reused.
5. Architect re-runs verification and updates durable state.
6. Optional: dispatch the reviewer; blocking findings return to the owning
   worker.
7. Architect commits after the gate and optional review.
8. Contract changes requested by workers return to the architect.

## Batch review and gate

- Workers never commit or push.
- The architect batches one or more worker tasks on a feature branch.
- The architect reviews the combined diff and runs the full gate once per
  batch, not per task.
- Optional: dispatch the reviewer after workers and before commit; treat
  findings as advisory.
- The architect commits and pushes the feature branch.
- For browser or UI changes, the architect validates the dev preview before
  merging to `main`.
- PRs to `main` require `gate` CI and @joegaffey approval.
- The architect may merge to `dev`; never to `main`.
- If the gate fails, send a focused packet to the owning worker.

## Context rules

- Load only what the current decision needs.
- Use `GOALS.md` to prioritize work.
- Prefer `agents/contracts.md` and role cards over reading source.
- Never read generated, vendor, build, cache, fixture, lockfile, or secret
  paths.
- Do not rely on chat history; use `PROGRESS.md`, `SPEC.md`, `PLAN.md`.

## Escalation

- Ambiguous requirement: `BLOCKED`, list options.
- Breaking contract change: `BLOCKED` with `contract_change`.
- Cross-domain conflict: architect resolves; workers do not negotiate
  directly.

## Definition of done

- Scoped verify command passes.
- No contract files edited by a worker.
- Manifest returned.
- If the reviewer was used, findings are resolved or explicitly accepted.
- If a spike was used, the report exists, the architect signed off, and the
  spike branch was discarded, tested, shared, or promoted.
- If the change is a breaking contract change, a new contract layer, a topology
  change, or an architecture-changing spike promotion, an accepted ADR exists.
- New debt is recorded in `docs/tech-debt.md`; accepted debt has a review date.
- Architect has re-run verification and updated durable state.
