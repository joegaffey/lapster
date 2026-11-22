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
| [0003](0003-section-point-venue-graph.md) | Section-point venue graph with mid-section junctions | Accepted; spline clause superseded by ADR-0004 |
| [0004](0004-b-spline-control-points.md) | B-spline control-point venue curves | Accepted |
| [0005](0005-junction-road-geometry.md) | Junction road geometry | Accepted |
| [0006](0006-track-ed-hosting-and-track-pack.md) | Host track-ed as a submodule and share the track-pack compiler | Accepted |
| [0007](0007-promote-track-ed-as-sole-editor.md) | Promote track-ed as the sole track editor | Accepted |
| [0008](0008-defer-layouts-as-route-layer.md) | Defer layouts and treat them as a future route layer over track-ed sections | Accepted; implementation deferred |
| [0009](0009-agent-runtime-policy-and-editor-domain.md) | Agent runtime policy, editor domain and spike subagent | Accepted |
| [0010](0010-legacy-project-migration-window.md) | Migrate legacy projects with a one-release read-only window | Accepted |
| [0011](0011-race-layout-race-chunk.md) | Race layout chunk: layout as race route | Accepted |
| [0012](0012-contract-roots-and-agent-permissions.md) | Make contract ownership directory-based | Accepted |
| [0013](0013-ai-racing-and-online-contact.md) | AI racing and online contact direction | Accepted |
| [0014](0014-ai-opponents-in-physics-worker.md) | AI opponents in the physics worker | Accepted |
