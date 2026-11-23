# 0012. Make contract ownership directory-based so contract changes do not need agent-config edits

- Status: accepted
- Date: 2026-11-21
- Deciders: architect, @joegaffey
- Related: ADR-0011, ADR-0009, ADR-0002, `agents/contracts.md`,
  `agents/architect.md`

## Context

Adding the optional Race chunk in ADR-0011 required creating a new TypeScript
mirror at `web/src/assets/race.ts`. The architect adapter lists editable paths
explicitly, so the new file was not writable until the adapter was updated.
Because opencode does not hot-reload agent configuration, applying the change
also required restarting the shared `opencode.service` server, which interrupts
every other session and task on that server.

That is a process failure, not a one-off. Any new contract file on a path that
was not already enumerated has the same problem: edit the role card, edit the
adapter, restart the shared server, and hope no other task is running.

## Decision

Contract ownership is directory-based:

- Rust canonical contracts live under `crates/sim-types/**`. Protocol modules
  use `crates/relay-server/src/protocol*`.
- New TypeScript contract mirrors live under `web/src/contracts/**`.
- Existing mirrors remain at `web/src/assets/types.ts`,
  `web/src/types/asset-layout*`, and `web/src/net/protocol*` until they are
  migrated.
- The architect role card and adapter allow these roots by pattern:
  - `crates/sim-types/**`
  - `crates/relay-server/src/protocol*`
  - `web/src/types/asset-layout*`
  - `web/src/assets/types.ts`
  - `web/src/contracts/**`
  - `web/src/net/protocol*`
- Worker adapters deny `web/src/contracts/**` and the existing TS contract
  mirrors. Workers still never edit canonical contracts or mirrors.

Adding a file under one of these roots does **not** require an agent-config
change or a server restart. One restart is still required now to load the new
pattern-based permissions into the running server; after that, the roots are
generic.

ADR-0011's `web/src/assets/race.ts` mirror path becomes
`web/src/contracts/race.ts`. The Race chunk contract and its semantics are
unchanged.

## Consequences

### Positive

- New contract files under the defined roots are immediately writable by the
  architect.
- Contract changes no longer interrupt unrelated sessions on the shared
  server.
- Worker/architect separation is expressed by directory, not an ever-growing
  file list.
- The contract registry can describe roots and conventions instead of every
  new file.

### Negative / risks

- One restart is still needed to load the initial pattern-based permission
  change.
- `web/src/contracts/**` is a new convention; existing mirrors remain split
  until migrated.
- A contract file placed outside the roots still needs an explicit permission
  change. The contract registry must document the roots clearly.
- Pattern-based access is broader than per-file access; the architect must
  continue to avoid editing non-contract implementation code.

## Alternatives considered

- **Allow `web/**` for the architect.** Rejected: it would let the architect
  edit web implementation files and break domain ownership.
- **Keep per-file contract permissions.** Rejected: it caused the restart
  problem and will recur for every new mirror.
- **Generate adapter permissions from `agents/contracts.md`.** Rejected for
  now: more tooling and another moving part; directory roots solve the problem
  with less machinery.
- **Hot-reload agent configuration in the server.** Not available in the
  current runtime; the process must be restarted to load adapter changes.

## Follow-up

- Use `web/src/contracts/race.ts` for the RaceData mirror.
- Migrate the existing TS mirrors into `web/src/contracts/` when it is cheap to
  do so; keep re-exports for compatibility during the migration.
- Keep `agents/contracts.md` as the source of truth for contract roots and
  mirror conventions.
