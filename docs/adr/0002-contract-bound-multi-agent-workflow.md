# 0002. Contract-bound multi-agent workflow

- Status: accepted
- Date: 2026-11-08
- Deciders: @joegaffey, architect
- Related: `AGENTS.md`, `agents/contracts.md`, `agents/protocol.md`,
  `docs/spikes/README.md`

## Context

Long development sessions suffer from context-window bloat, inconsistent
domain ownership, and undocumented interface drift. We want parallel domain
work without losing contract discipline.

## Decision

Adopt a contract-bound multi-agent workflow:

- The architect owns canonical contracts, mirrors, durable state, and
  integration.
- Domain workers are short-lived subagents. They own one domain, read contracts,
  never edit them, and return a compact manifest.
- A reviewer subagent is optional and advisory.
- A spike agent is optional for isolated prototype and architecture-spike work.
  It may test interface changes, writes `docs/spikes/<name>.md`, commits
  locally, and never pushes. The architect reviews and signs off.
- The architect gates integration: one full gate per batch, commits, pushes
  feature branches, and merges to `dev`. `main` is protected and requires CI,
  an architect comment, and human approval.
- Significant decisions are recorded as ADRs.

## Consequences

### Positive

- Domain ownership is explicit and enforced by permissions.
- Worker context stays small and disposable.
- Contract drift is caught before integration.
- Decisions are durable.

### Negative / risks

- Permission rules are guardrails, not a sandbox; scripts and build commands
  can bypass edit scopes.
- The architect can become a bottleneck.
- Spike and reviewer modes add optional agents to maintain.

## Alternatives considered

- A single general-purpose agent: simpler, but no domain isolation or durable
  contract ownership.
- One long-lived agent per domain: more context, more drift, harder to discard.
- No architect gate: faster locally, but weaker integration safety.

## Follow-up

- Keep the contract registry current.
- Use ADRs for breaking contract changes and spike promotions.
