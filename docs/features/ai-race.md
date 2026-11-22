# AI Race — Grid Start, Countdown, Standings, and Results

- **Status:** complete; merged to `dev` as PR #52
- **Date:** 2026-11-22
- **Owner:** architect
- **Domains:** architect, physics, render, web
- **Branch:** `feat/ai-race`
- **Related:** ADR-0015, ADR-0013, ADR-0014, `docs/features/ai-field.md`,
  `docs/features/race-manager.md`, `docs/tech-debt.md`

## 1. Goal

Turn the AI field into a short local race:

> The player and three AI start from the grid. A 3-2-1 countdown starts a
> three-lap race. The HUD shows the player's position, and finishing shows a
> simple results order. No contact or netcode is added.

## 2. Scope

In scope:

- player at race grid slot 0, AI at slots 1..3;
- runtime grid heading from the nearest layout tangent for player and AI;
- grid-start semantics so the first start/finish crossing begins lap 1;
- 3-second countdown with controls and AI held neutral;
- three laps by default;
- live player position (`P1/4`);
- finish detection for player and AI;
- simple results overlay when the player finishes;
- `?laps=1..5` validation override, default 3;
- tests for grid start, countdown, positions, and finish ordering.

Out of scope:

- car-to-car contact or avoidance;
- online/relay changes;
- championship points, persistence, or leaderboards;
- sector split times or telemetry analysis;
- pit stops, damage, weather, or flags;
- a full race calendar or multiple race layouts;
- fixing track-ed grid rotations (TD-0021 remains separate).

## 3. Contracts and interfaces

No versioned contract change. `RaceState` and the 64-float snapshot stay the
same.

`sim-core`:

```rust
impl RaceTracker {
    /// Arms the first forward start/finish crossing to begin lap 1 without
    /// incrementing the lap count.
    pub fn arm_grid_start(&mut self);
}
```

`sim-worker` wasm API:

```rust
Simulation::place_on_race_grid(slot: u32)
Simulation::start_race(laps: u32)
Simulation::ai_race_state(index: u32) -> String // RaceState JSON
```

Worker `ai_state` message:

```json
{
  "type": "ai_state",
  "slot": 1,
  "frame": "<64-float vehicle snapshot>",
  "race": { "available": true, "lap": 1, "sector": 4, "...": "..." }
}
```

`render`:

```rust
DriveViewer::place_on_race_grid(slot: u32)
```

The web owns the race director and view model. It consumes the local
`RaceState` and each AI `RaceState`; no new Rust contract is introduced.

## 4. Grid start

- `place_on_race_grid(0)` places the player at `race.grid[0]`; AI use
  `race.grid[1..]`.
- Both use the nearest layout tangent for heading. This avoids TD-0021 while
  it remains open.
- Trackers are armed for a grid start after placement.
- The first forward crossing of start/finish:
  - sets `started = true`;
  - resets the current lap timer;
  - keeps `lap = 1`;
  - clears wrong-way.
- Later forward crossings increment the lap normally.

Countdown:

- The web shows `3`, `2`, `1`, `GO`.
- While counting down, the worker is paused and main-thread input is gated.
- At GO, the worker resumes and the race director starts its clock.

## 5. Race director

Add a pure web `RaceDirector`:

```ts
interface RaceCarState {
  slot: number;
  isPlayer: boolean;
  lap: number;
  sector: number;
  progress: number;
  distanceM: number;
  lastLapTime: number | null;
  bestLapTime: number | null;
  finished: boolean;
  finishTime: number | null;
}

interface RaceStanding extends RaceCarState {
  position: number;
  totalDistanceM: number;
}

interface RaceView {
  totalLaps: number;
  elapsed: number;
  cars: RaceStanding[];
  playerPosition: number;
  playerFinished: boolean;
}
```

Ordering:

- completed laps = `max(0, lap - 1)`;
- total distance = `completed * layoutLength + distanceM`;
- finished cars sort by finish time, then position;
- unfinished cars sort by total distance descending;
- ties fall back to slot order.

Finish:

- a car is finished when `lap > totalLaps`;
- the race director records the current race elapsed time at the first
  observation of finish;
- when the player finishes, results are shown immediately using the current
  order. Unfinished AI are shown by distance, not as penalties.

## 6. HUD and overlays

Race panel additions:

- `P1/4` position;
- lap shown as `1/3`;
- existing current/last/best and wrong-way;
- existing `AI 3` field.

Overlays:

- countdown: large center number, hidden after GO;
- results: ordered rows with position, `You`/`AI n`, laps, and total time;
- results include a `Restart` action that re-grids and replays the countdown.

## 7. Tests

Core:

- grid-start arming: first forward crossing keeps lap 1 and starts the timer;
- subsequent crossing increments;
- reset/re-arm preserves best lap;
- backward crossing on grid sets wrong-way and does not increment.

Worker:

- `start_race` holds AI neutral before start;
- AI race state JSON is available and serializes all `RaceState` fields;
- AI stops racing after the lap limit;
- wasm build remains green.

Web:

- Race Loop starts at grid slot 0 with a visible countdown;
- player and AI do not move during countdown;
- after GO, AI move and player input is accepted;
- HUD shows `P?/4` and `Lap 1/3`;
- `RaceDirector` unit tests cover ordering, finished cars, and current-order
  results with unfinished AI;
- `?laps=1` validation path reaches results with a short race;
- existing Race Loop, AI field, and Skidpad tests remain green.

## 8. Work breakdown

| Phase | Domain | Deliverable | Verify |
|---|---|---|---|
| 0 | architect | ADR-0015, plan, progress checkpoint | `git diff --check`, agent sync |
| 1 | physics | grid-start arming, worker race placement/start/laps, AI race state | `cargo test -p sim-core -p sim-worker`, clippy, wasm |
| 2 | render | `DriveViewer::place_on_race_grid`, local tracker arming | `cargo test -p sim-render`, wasm |
| 3 | web | race director, countdown, HUD position, results, tests | `npm --prefix web test`, viewer |
| 4 | architect | full gate, preview, PR | full gate |

Dependencies:

- physics defines the worker race API and AI race state shape;
- render exposes local grid placement;
- web consumes both and owns the race state machine.

## 9. Acceptance

- Race Loop starts the player at `race.grid[0]` and three AI at 1..3.
- Countdown prevents movement until GO.
- The first start crossing starts lap 1, not lap 2.
- The HUD shows player position and `1/3`.
- Three laps finish the player race and show results.
- AI positions update from real lap/progress state.
- Skidpad remains AI-free and unchanged.
- Full gate passes.

## 10. Implementation notes

- `RaceTracker::arm_grid_start` keeps lap 1 on the first forward start crossing;
  `Vehicle::set_spawn` supports race-grid resets.
- The worker exposes race-grid placement, explicit start with a lap limit, AI
  stop-after-limit behavior, and AI `RaceState` JSON in `ai_state`.
- `DriveViewer::place_on_race_grid` places and arms the local tracker.
- The web adds a pure `RaceDirector`, 3-2-1-GO countdown, live `P?/4` and
  `Lap ?/3`, results overlay, restart flow, and `?laps=1..5`.
- Gate for the AI race changes before the track-option merge: cargo fmt,
  workspace clippy, 228 workspace tests, wasm check; 68 web tests; 114 viewer
  tests passed / 1 skipped; WebGL2 fallback 2/2; agent sync. The combined branch
  PR CI covers the merged track-option fixture and launcher tests.
- Browser coverage covers grid start, countdown, AI options, fast pace, reset,
  results-overlay/restart, and main-thread fallback. Full-race auto-finish is
  covered by `RaceDirector`/results unit tests rather than a long browser race.

## 11. Risks

- The player and AI race states update at different rates; finish times are
  sampled.
- Results show current order if AI have not finished.
- TD-0021 remains open; runtime tangent heading is still a workaround.
- The race director adds web state that must reset cleanly on restart.
