# Architecture Decision Records

This directory records significant architecture decisions: the context, the
decision, the consequences, and the alternatives considered.

## When to write an ADR

Required for:

- Breaking contract, ABI, schema, or protocol changes.
- A new contract layer or mirror.
- A new domain or worker, or a change to the agent topology.
- Cross-domain architecture decisions.
- Security, performance, or data-format tradeoffs.
- Promoting a spike that changes architecture.

Not required for routine implementation, bug fixes, or minor refactors.

## Format

Use `template.md`. Name files `NNNN-kebab-title.md` with a sequential number.

## Lifecycle

- Status: proposed, accepted, rejected, deprecated, or superseded by ADR-NNNN.
- Accepted ADRs are immutable. Supersede with a new ADR; do not rewrite history.
- The architect owns this directory and accepts ADRs.
- Reference the ADR from the contract registry or PR when it affects an
  interface.

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | Accepted |
| [0002](0002-contract-bound-multi-agent-workflow.md) | Contract-bound multi-agent workflow | Accepted |
| [0003](0003-multi-layout-track-model.md) | Multi-layout track model | Superseded by 0004 |
| [0004](0004-venue-graph-shared-segments.md) | Venue graph, shared segments, and cross-section profiles | Superseded by 0005 |
| [0005](0005-loop-shared-control-points.md) | Loop and shared-control-point model | Superseded by 0006 |
| [0006](0006-drivable-venue-network.md) | Drivable venue network with canonical segments and loop-centric routes | Accepted |
