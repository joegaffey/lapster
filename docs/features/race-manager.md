# Race Manager MVP — Lap Progress and HUD

- **Status:** complete; full gate passed; parent PR ready
- **Date:** 2026-11-21
- **Owner:** architect
- **Domains:** architect, physics, render, web
- **Related:** ADR-0013, ADR-0011, `GOALS.md`,
  `docs/features/track-race-mvp.md`, `agents/contracts.md`

## 1. Goal

Turn the race layout data from ADR-0011 into visible, deterministic race state
for the local car:

> A race-ready track reports lap, sector, progress, wrong-way state, and
> current/last/best lap time. The Drive viewer shows a small race HUD. A track
> without race data remains race-unavailable.

This is the first batch after the race layout foundation. It does not add AI
opponents, contact, or netcode.

## 2. Contract

Add `RaceState` to `crates/sim-types/src/race.rs` and mirror it in
`web/src/contracts/race-state.ts`:

```rust
pub struct RaceState {
    pub lap: u32,
    pub sector: u32,
    pub progress: f32,
    pub distance_m: f32,
    pub wrong_way: bool,
    pub started: bool,
    pub current_lap_time: f64,
    pub last_lap_time: Option<f64>,
    pub best_lap_time: Option<f64>,
}
```

Runtime JSON view:

```json
{
  "available": true,
  "lap": 1,
  "sector": 1,
  "progress": 0.12,
  "distance_m": 44.2,
  "wrong_way": false,
  "started": true,
  "current_lap_time": 12.3,
  "last_lap_time": null,
  "best_lap_time": null
}
```

Unavailable view:

```json
{ "available": false }
```

Semantics:

- `lap` and `sector` are 1-based.
- `progress` is normalized distance along the layout in `[0, 1]`.
- `distance_m` is the projected along-track distance.
- `started` becomes true on the first forward movement; the lap timer starts
  then, not at page load.
- `current_lap_time`, `last_lap_time`, and `best_lap_time` are seconds.
- A forward crossing of the start/finish increments `lap` and records the last
  and best lap.
- A backward crossing does not increment `lap`; it sets `wrong_way`.
- `sector` is derived from `progress`; this batch does not add separate sector
  split times.
- Resetting the car resets the current lap/sector timer but preserves the best
  lap.

## 3. Core algorithm

Add `sim-core::race::RaceTracker`:

- Precompute layout points as `Vec3`, segment lengths, total length, and
  cumulative distances.
- On each update, project the car position onto every layout segment and choose
  the nearest point. At roughly 180 points this is cheap for the local car.
- Derive normalized progress and sector from the projected distance.
- Track the previous progress to detect start/finish crossings:
  - forward wrap: `previous > 0.8 && current < 0.2`;
  - backward wrap: `previous < 0.2 && current > 0.8`.
- Use the layout tangent and position delta for wrong-way hysteresis.
- Use `FIXED_DT` and the vehicle tick for lap timing.
- Keep the tracker independent of the visual racing line.

## 3.1 Track-ed spawn direction

The Race Loop HUD exposed that track-ed exports did not pre-flip the track
metadata spawn for Lapster's runtime drive flip. The car spawned facing
opposite the race layout. Track-ed PR #7 fixed it; the parent fixture was
regenerated, the pack re-baked, and the parent submodule pin moved to the merged
track-ed `main` commit (1b22f5a).

## 4. Runtime integration

- `DriveViewer` owns an optional `RaceTracker` when `RaceData` exists.
- Update the tracker after every local state update:
  - main-thread `update`/`simulate`;
  - worker-driven `render_remote`;
  - `reset`.
- Add `DriveViewer.race_state()` returning the contract JSON.
- Add a `raceState` object to telemetry.
- Keep `DriveViewer.race()` unchanged.

## 5. Web HUD

- Add a compact Race panel to the Drive HUD:
  - lap;
  - sector;
  - current time;
  - last lap;
  - best lap;
  - wrong-way indicator.
- Hide the panel when `raceState.available` is false.
- Parse telemetry with `RaceStateView`; do not infer race state from layout data
  in the web layer.

## 6. Tests

- `sim-types`: `RaceState` serde round trip and stable field names.
- `sim-core`: straight-line/closed-loop projection, sector changes, forward lap
  crossing, backward crossing sets `wrong_way`, timer starts on movement,
  reset preserves best, non-finite positions do not corrupt state.
- `sim-render`: `race_state()` unavailable shape; available shape from a
  synthetic `RaceData`; telemetry includes `raceState`.
- `web`: contract validator accepts/rejects the union; HUD hidden on Skidpad;
  HUD shows lap/sector/time on Race Loop; progress increases after driving
  forward; wrong-way indicator has a testable selector.
- `web` browser: Race Loop shows the HUD, Skidpad does not.

## 7. Work breakdown

| Phase | Domain | Deliverable | Verify |
|---|---|---|---|
| 0 | architect | `RaceState`, TS mirror, registry, ADR-0013, plan | doc review, `git diff --check` |
| 1 | physics | `sim-core` race tracker and tests | `cargo test -p sim-core`, clippy |
| 2 | render | `DriveViewer.race_state()`, telemetry, tracker updates | `cargo test -p sim-render`, wasm check |
| 3 | web | contract tests, HUD, browser coverage | `npm --prefix web test`, viewer tests |
| 4 | architect | docs, full gate, PR | full gate |

Dependencies:

- Phase 1 depends on the `RaceState` contract.
- Phase 2 depends on phase 1.
- Phase 3 depends on phase 2's JSON shape.

## 8. Out of scope

- AI opponents, multi-vehicle worker, and race positions.
- Car-to-car contact and relay protocol changes.
- Countdown, start lights, results, and sector split times.
- Multiple layouts, reverse layouts, and pit lanes.
- Server-authoritative lap validation.

## 9. Acceptance

- Race Loop reports lap 1, sector 1, and advancing progress after the car moves.
- Crossing sector boundaries updates the sector.
- Crossing start/finish forward increments the lap and records last/best time.
- Reversing across the line sets wrong-way and does not increment the lap.
- Default Skidpad and `fixtures/tracks/minimal` report race unavailable and show
  no race HUD.
- Existing drive, physics, and multiplayer behavior is unchanged.
