# Goals

## Vision

A browser-native vehicle simulation engine with a custom deterministic vehicle
model, contract-bound asset packs, and playable viewers, runnable from a static
host with no application server.

## Target users

- Developers and enthusiasts who want a web-native driving simulation they can
  run, inspect, and extend.
- Contributors working on physics, rendering, assets, multiplayer, or the web
  client.
- Players who want a quick, installable browser demo rather than a full game.

## Goals

- Deliver a runnable vertical slice: one car, one track, custom vehicle
  dynamics, wgpu rendering, audio, FFB, and deterministic replay.
- Keep the engine zero-backend and statically hostable.
- Keep asset packs versioned, validated, and contract-bound.
- Keep the simulation deterministic for a given build and input replay.
- Keep the implementation original and permissively licensed.
- Provide a small, coherent set of viewers and a relay multiplayer prototype.
- Keep the multi-agent workflow contract-bound and context-efficient.

## Success criteria

- `cargo fmt --check`, clippy, workspace tests, and the wasm check pass in CI.
- `npm test` and `npm run test:viewer` pass.
- The demos run on the Pages build with WebGPU and a WebGL2 fallback.
- Fixed-step replay is deterministic on the same build.
- The physics worker holds its configured tick rate; the main-thread fallback
  remains available.
- Asset packs validate against the current schema and round-trip through the
  compiler.
- The relay prototype supports up to four players on one car and track.
- No application server is required; a static host with COOP/COEP headers is
  sufficient.

## Non-goals

- A finished game or a complete Assetto Corsa replacement.
- Cross-platform deterministic physics.
- Universal force feedback across all wheelbases.
- AI opponents, damage, weather, or tire thermal/wear simulation.
- Hard real-time guarantees.
- Delegating vehicle handling to a general-purpose physics engine's vehicle
  controller.

## Principles and constraints

- Web-native and zero-backend.
- Permissive licensing; original engine implementation.
- Real mod assets are user-supplied and never committed.
- Contract first: define the interface before the implementation.
- Small, runnable vertical slices with measurable checkpoints.
- Deterministic fixed-step simulation and versioned asset packs.
- Context-bounded multi-agent workflow with an architect gate.

## Milestones

See `PROGRESS.md` for the current phase and `PLAN.md` for the active plan.

## Review

Reviewed at phase changes. Significant goal changes that affect scope or
architecture require an ADR in `docs/adr/`.
