# 0001. Record architecture decisions

- Status: accepted
- Date: 2026-11-08
- Deciders: @joegaffey, architect
- Related: `AGENTS.md`, `agents/architect.md`

## Context

Significant architecture decisions were recorded only in chat, commit
messages, and design notes. It was hard to recover why an interface or
workflow was chosen and what alternatives were rejected.

## Decision

Record significant architecture decisions as ADRs in `docs/adr/`, using the
MADR-lite template. The architect owns the directory and accepts ADRs. Accepted
ADRs are immutable and are superseded rather than rewritten.

## Consequences

### Positive

- Decisions are durable and reviewable.
- Contract changes and spike promotions can reference a decision record.
- New contributors and agents can recover context without chat history.

### Negative / risks

- Small process overhead for architecture changes.
- ADRs can go stale if not superseded.

## Alternatives considered

- Keep decisions in commit messages: hard to discover and compare.
- Keep decisions in `SPEC.md`: mixes requirements with rationale.
- Use a wiki: not versioned with the code.

## Follow-up

- Write ADR 0002 for the multi-agent workflow.
- Reference ADRs from contract changes and spike promotions.
