# 0014. AI opponents in the physics worker

- Status: accepted
- Date: 2026-11-22
- Deciders: architect, @joegaffey
- Related: ADR-0011, ADR-0013, `GOALS.md`,
  `docs/features/ai-opponents.md`, `docs/features/race-manager.md`

## Context

ADR-0013 approved AI opponents as the batch after the race manager. The race
layout and `RaceState` are already implemented, but the runtime is still
single-vehicle: the physics worker owns one `sim_core::Vehicle`, the SAB carries
one snapshot, and remote rigs render only multiplayer ghosts.

The first useful AI slice is an opponent that starts from the baked race grid,
follows the layout centreline, uses the same vehicle model and collider as the
player, and is visible in the Drive viewer. Car-to-car contact, netcode, and
race standings are later work.

## Decision

1. **Simulate AI in the existing physics worker.** The worker owns the player
   and up to three AI vehicles, stepped at the same fixed 500 Hz rate. AI
   vehicles use `sim_core::Vehicle` and the track collider, not a kinematic
   visual-only follower.

2. **Keep the controller deterministic and layout-driven.** Add
   `sim_core::AiDriver`, which consumes `VehicleState` and a layout sample from
   `RaceTracker`, and returns `Controls`. It must not depend on the visual
   racing line or on rendering state.

3. **Start AI from the race grid.** AI `n` starts at `race.grid[n + 1]`;
   the player remains at the track spawn/start-finish. The first AI therefore
   starts beside or behind the player rather than on the same point. If the
   pack has no race data or fewer than two grid slots, no AI is created.

4. **Reuse the existing snapshot ABI.** An AI frame is the existing 64-float
   `SNAPSHOT_STATE` payload. The worker posts `{ type: "ai_state", slot, frame }`
   over the control channel at about 62.5 Hz (every eight physics ticks). Slot
   `n` is the renderer remote slot `n + 1`. No SAB layout or schema change is
   required.

5. **Render through the existing remote-car path.** The web client creates one
   hidden remote rig per AI and applies frames with
   `DriveViewer.set_remote_state(slot, frame)`. AI brake lights and particles
   use the same remote rig path as multiplayer.

6. **No car-to-car contact in this batch.** AI cars collide with the track and
   follow the layout, but player and AI bodies do not collide. Contact is part
   of the online-contact batch and needs its own semantics.

7. **No new versioned contract.** `RaceData` and `RaceState` are unchanged.
   The worker interface is an internal cross-domain interface documented in
   `docs/features/ai-opponents.md`.

## Consequences

### Positive

- AI uses the same physics, tires, collisions, and tuning as the player.
- Race progress and AI path following share one layout projection.
- The renderer and remote-rig code are reused instead of forked.
- The batch is testable on Race Loop without netcode or new asset work.

### Negative / risks

- The physics worker now steps four vehicles in the worst case; catch-up cost
  and worker CPU need measurement.
- AI frames arrive at 62.5 Hz, not 500 Hz, so the render pose is sampled.
- Without contact, cars can overlap; this is an explicit prototype limitation.
- The main-thread physics fallback has no AI path; AI requires the worker.

## Alternatives considered

- **Main-thread AI in `DriveViewer`.** Rejected: it moves physics off the
  worker and makes the fallback the fastest AI path.
- **Kinematic AI moved along the racing line.** Rejected: it does not exercise
  the vehicle model and produces a visually disconnected opponent.
- **A separate AI worker per car.** Rejected for the first slice: more worker
  lifecycle, SAB, and scheduling complexity for little benefit.
- **Extend the SAB with AI snapshots.** Deferred: the control channel is
  sufficient for one to three AI cars at 62.5 Hz and avoids a layout version
  change.
- **Add contact now.** Deferred to the online-contact batch per ADR-0013.

## Follow-up

- Measure worker cost with one and three AI cars; record debt if catch-up
  rises.
- Add car-to-car contact and standings in the online-contact batch.
- Add AI difficulty, multiple controllers, and racing-line quality later.
- Replace the 62.5 Hz control-channel frame stream with a SAB region if the
  sample rate becomes a visible or CPU problem.
