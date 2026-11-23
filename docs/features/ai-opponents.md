# AI Opponents — Race-Grid Path Following

- **Status:** complete; full gate passed; PR pending
- **Date:** 2026-11-22
- **Owner:** architect
- **Domains:** architect, physics, web
- **Branch:** `feat/ai-opponents`
- **Related:** ADR-0013, ADR-0014, ADR-0011, `GOALS.md`,
  `docs/features/race-manager.md`, `agents/contracts.md`

## 1. Goal

Add the first visible AI opponent to the race-ready Drive slice:

> On a race-ready track, one AI car starts from the baked race grid, follows the
> race layout using the same vehicle model and collider as the player, and is
> rendered as a moving opponent. Tracks without race data remain unchanged.

This batch proves the multi-vehicle path and the layout-following controller. It
does not add car-to-car contact, results, standings, or netcode.

## 2. Scope

In scope:

- one AI opponent by default on race-ready tracks;
- up to three AI vehicles supported by the worker, with one selected in Drive;
- AI start from race grid slots 1..N, leaving the player at the track spawn;
- AI grid heading derived from the nearest layout tangent, because the Race
  Loop grid rotations do not currently match the layout direction (TD-0021);
- AI controller using layout samples and `VehicleState`;
- AI simulation in the physics worker at 500 Hz;
- AI frames over the worker control channel at about 62.5 Hz;
- AI rendering through the existing remote-car rig path;
- tests for controller behavior, worker frames, and the Drive browser slice.

Out of scope:

- car-to-car contact, collision response, or contact messages;
- AI in the main-thread `?physics=main` fallback;
- AI count or difficulty UI;
- race standings, position HUD, countdown, start lights, results;
- racing-line import or optimal lap-time tuning;
- AI audio voices;
- relay protocol changes;
- multiple AI controllers or machine learning.

## 3. Contracts and interfaces

No versioned contract change. Reuse:

- `crates/sim-types/src/race.rs` (`RaceData`, `RaceState`);
- `crates/sim-types/src/abi.rs` `SNAPSHOT_STATE` / 64-float snapshot payload;
- `web/src/contracts/race.ts` and `race-state.ts`.

New internal interfaces:

```rust
// sim-worker wasm API
Simulation::new(track_pack_bytes: &[u8], car_pack_bytes: &[u8], ai_count: u32)
Simulation::ai_count() -> u32
Simulation::ai_snapshot(index: u32) -> Vec<f32> // 64-float SNAPSHOT_STATE
```

```text
// worker -> main thread, every 8 physics ticks
{ "type": "ai_state", "slot": 1, "frame": <64-float snapshot> }
```

```ts
// web client
PhysicsClient.create(sab, trackBytes, carBytes, aiCount = 0)
PhysicsClient.onAiFrame: ((slot: number, frame: Float32Array) => void) | null
```

Slot `n` maps directly to `DriveViewer.set_remote_state(n, frame)`; slot 0 is
the local player.

## 4. Core controller

Add `sim-core::AiDriver` and a layout sampling method on `RaceTracker`:

```rust
pub struct LayoutSample {
    pub point: Vec3,
    pub tangent: Vec3,
    pub curvature: f32,
}

impl RaceTracker {
    pub fn total_length(&self) -> f32;
    pub fn sample(&self, distance_m: f32) -> Option<LayoutSample>;
}

impl AiDriver {
    pub fn controls(&mut self, state: &VehicleState, tracker: &RaceTracker) -> Controls;
}
```

Controller behavior:

- Look ahead along the layout, with distance increasing with speed:
  `lookahead = clamp(8.0 + speed * 0.35, 8.0, 24.0)`.
- Steer toward the look-ahead point using the car's local forward axis.
- Estimate curvature from layout samples ahead and choose a target speed from a
  lateral-acceleration limit, clamped to a conservative maximum.
- Throttle when below target speed, brake when clearly above it.
- Upshift/downshift from engine RPM with a short cooldown; never select reverse.
- Return finite controls for degenerate layouts and non-finite state.
- Do not read the visual racing line.

Determinism:

- The controller is a pure function of `VehicleState`, the tracker, and its own
  shift cooldown.
- The integration tick is the worker tick; no wall-clock or random input.

## 5. Worker integration

Each AI car owns:

- a `Vehicle` spawned at `race.grid[index + 1]` and oriented to the nearest
  layout tangent (falling back to the grid rotation plus the runtime `PI` flip
  if the layout sample is unavailable);
- its own `RaceTracker`;
- an `AiDriver`;
- the last throttle/brake values for the snapshot.

`Simulation`:

- loads optional `RaceData` from the track pack;
- clamps `ai_count` to `0..=min(3, grid.len() - 1)`;
- settles the player and AI vehicles on creation/reset;
- steps the player, then each AI, in the same fixed-step loop;
- resets AI with the player and reapplies setup/tuning/assists to all vehicles;
- returns a 64-float snapshot for each AI via `ai_snapshot`.

Worker publishing:

- publish AI frames every eight ticks, about 62.5 Hz;
- publish immediately after reset, place-on-grid, simulate, and tuning changes;
- do not publish when `ai_count == 0`.

## 6. Web integration

- Detect race availability from the loaded track pack's optional `race`
  entrypoint.
- On a race-ready track in single-player, request one AI.
- Create the Drive viewer with `remoteSlots = mpMode ? 3 : aiCount`.
- Pass `aiCount` to `PhysicsClient.create`.
- Route `onAiFrame` to `viewer.set_remote_state(slot, frame)`.
- Keep the main-thread fallback working without AI.
- No new HUD is required; the existing race HUD remains the local player's
  state. The AI is visible in the scene and in `telemetry.remoteCars`.

## 7. Tests

Physics:

- `RaceTracker::sample` returns points on straight and curved layouts, wraps
  distances, and reports higher curvature in a corner.
- `AiDriver` produces throttle on a straight and a steering sign matching a
  corner.
- Target speed drops for a tighter corner.
- Gear selection upshifts and downshifts with RPM and respects cooldown.
- Degenerate layouts and non-finite state produce finite controls.
- Worker compiles for wasm with the new API.

Web:

- Race Loop creates one AI and receives `remoteCars` entries.
- After driving or waiting, the AI position changes and remains finite.
- Skidpad creates no AI and keeps `remoteSlots` at zero in single-player.
- No page or console errors.
- Existing race HUD and player race-state tests remain green.

## 8. Work breakdown

| Phase | Domain | Deliverable | Verify |
|---|---|---|---|
| 0 | architect | ADR-0014, plan, progress checkpoint | `git diff --check`, agent sync |
| 1 | physics | `AiDriver`, `RaceTracker::sample`, worker multi-vehicle path | `cargo test -p sim-core -p sim-worker`, clippy, wasm |
| 2 | web | `ai_state` handling, Drive integration, browser coverage | `npm --prefix web test`, viewer |
| 3 | architect | combined review, full gate, preview, PR | full gate |

Dependencies:

- Physics defines the wasm API and snapshot behavior.
- Web consumes that API and the existing remote-rig contract.
- Both phases share the fixed 64-float snapshot shape.

## 9. Acceptance

- On Race Loop in the normal worker path, one AI car appears and moves forward
  from its grid slot.
- AI progress and position change over time; no NaNs or console errors.
- The player's race HUD and lap state behave as before.
- Default Skidpad has no AI and no extra remote rigs.
- The AI can follow the Race Loop layout without leaving the road for the
  checked window.
- Car-to-car overlap is allowed and documented as deferred.
- Full gate passes.

## 10. Risks and follow-up

- Worker catch-up cost rises with AI count; measure one and three AI cars and
  record debt if needed.
- 62.5 Hz frames can look sampled on high-refresh displays; evaluate
  interpolation or a SAB region later.
- No contact means cars can overlap; online-contact batch owns contact
  semantics.
- AI pace and line quality are intentionally conservative; racing-line work is
  a later batch.

## 11. Implementation notes

- One AI is requested by default on a race-ready track; the worker supports up
  to three and clamps to the available grid slots. The player remains at the
  track spawn.
- Race Loop grid-slot rotations do not match the layout direction, so AI heading
  comes from the nearest layout tangent (TD-0021). This is a workaround for the
  current export, not the intended long-term source of grid heading.
- AI frames are published at 62.5 Hz and rendered through the existing remote
  rigs without interpolation. There is no car-to-car contact in this batch.
- Manual Race Loop probe after the heading fix: the AI stayed within 3.62 m of
  the centreline (5 m half-width), completed a lap in about 40 s, and produced
  no page errors.
- Gate: cargo fmt, workspace clippy, 213 workspace tests, wasm check; 57 web
  tests; 106 viewer tests passed / 1 skipped; WebGL2 fallback 2/2; agent sync.
