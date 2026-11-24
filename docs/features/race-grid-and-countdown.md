# Race Grid and Pre-Countdown Visibility

- **Status:** planned
- **Date:** 2026-11-23
- **Owner:** architect
- **Domains:** architect, physics, render, web
- **Branch:** `feat/online-contact`
- **Related:** ADR-0019, ADR-0015, `docs/features/ai-race.md`,
  `docs/features/online-contact.md`, `docs/tech-debt.md`

## 1. Goal

Make the local race start read correctly:

> The four cars sit in narrow, on-track grid columns; the player starts at the
> back of the active field; and all AI cars are visible on the grid during the
> 3-2-1 countdown, not only after GO.

## 2. Scope

In scope:

- clamp race-grid lateral offsets toward the layout centreline;
- start the player in the last active grid slot and fill AI ahead;
- publish and render AI frames before the countdown starts;
- keep the HUD position consistent with the grid order before GO;
- tests for placement, countdown visibility, and start order.

Out of scope:

- changing the exported grid slots in track-ed (TD-0021 remains the long-term
  fix);
- raising the AI field size (still 3);
- car-to-car avoidance;
- multiplayer start behavior;
- new contracts or schema changes.

## 3. Contracts and interfaces

No versioned contract change. `RaceData`, `RaceState`, the 64-float snapshot,
and the net protocol are unchanged.

`sim-core` gains an additive grid-placement helper:

```rust
pub const GRID_MAX_LATERAL: f32 = 1.8;

impl RaceTracker {
    /// Projects a grid slot onto the layout, clamps its lateral offset, and
    /// returns the placement position plus the layout tangent.
    pub fn grid_spawn(&self, base: &SpawnPoint) -> Option<(Vec3, Vec3)>;
}
```

The worker's `place_on_race_grid(slot)` keeps its wasm signature. `slot` is the
player's grid index; AI fill the lowest grid slots in order, skipping the
player's slot. Car slot ids stay `0 = player`, `1..=n = AI`.

`DriveViewer::place_on_race_grid(slot)` keeps its signature and uses the same
clamp through `sim-core`.

Web race-director options gain an optional start order:

```ts
interface RaceDirectorOptions {
  // ...
  startOrder?: number[]; // car slots in grid order; default [0, ...aiSlots]
}
```

Ties between unfinished cars use this order, so a last-place grid start shows
`P4/4` before GO.

## 4. Grid placement

- Project each grid slot onto the nearest layout point.
- Clamp the lateral offset to `GRID_MAX_LATERAL` (1.8 m) on either side of the
  centreline; preserve the slot's longitudinal position and y.
- Keep the runtime layout tangent as the heading (TD-0021 workaround).
- Player grid index = `min(ai_count, grid.len() - 1)`; with three AI the player
  is at slot 3 and AI at slots 0, 1, 2.
- `ai_count == 0` keeps the player at slot 0.

## 5. Countdown visibility

Current flow resets AI buffers and ignores AI frames while placing the grid, so
the placement frames are dropped and the paused worker does not publish again
until GO.

New flow:

1. Pause the worker.
2. Reset AI buffers.
3. Place the player locally and send `place_on_race_grid` to the worker; the
   worker publishes fresh AI frames from the grid.
4. Accept those frames and start the 3-2-1 countdown.
5. At GO, start the race and resume the worker.

The AI cars must be visible on the grid while `3`, `2`, and `1` are shown.

## 6. Tests

Physics:

- a wide grid slot at +/-4.5 m clamps to +/-1.8 m and keeps the tangent heading;
- player slot 3 with three AI places AI on grid 0,1,2 and the player on 3;
- player slot 0 keeps the old AI 1,2,3 behavior;
- existing grid-start and contact tests stay green.

Render:

- local `place_on_race_grid(3)` uses the clamped grid position and tangent.

Web:

- before GO, `telemetry().remoteCars` already contains the three AI cars;
- the grid HUD shows the player last (`P4/4`) with three AI;
- the countdown still holds movement until GO;
- existing Race Loop, AI field, AI race, and motion-smoothing tests stay green.

## 7. Work breakdown

| Phase | Domain | Deliverable | Verify |
|---|---|---|---|
| 1 | physics | `grid_spawn` clamp, player-last placement, tests | `cargo test -p sim-core -p sim-worker`, clippy, wasm |
| 2 | render | use the shared clamp for local grid placement | `cargo test -p sim-render`, clippy, wasm |
| 3 | web | player grid slot, pre-countdown AI frames, start order, tests | `npm --prefix web test`, viewer |
| 4 | architect | full gate, preview, PR | full gate |

## 8. Acceptance

- Grid columns are on the track and closer to the centreline.
- The player starts behind the AI on the grid.
- All three AI cars are visible during the countdown.
- The HUD shows the player last before GO, then updates from live progress.
- Full gate passes.

## 9. Risks

- The runtime clamp masks wide grid export; TD-0021 still needs the track-ed
  fix and a fixture regeneration.
- The player-last grid changes initial positions, so AI spacing and the start
  shove behavior differ from the previous batch.
- AI frames published before the countdown must not leak stale frames from the
  previous run; reset buffers after pausing.
