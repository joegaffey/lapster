# 0015. AI race start, standings, and results

- Status: accepted
- Date: 2026-11-22
- Deciders: architect, @joegaffey
- Related: ADR-0013, ADR-0014, ADR-0011, `GOALS.md`,
  `docs/features/ai-field.md`, `docs/features/ai-race.md`

## Context

The AI field batch gives Race Loop three AI opponents, selectable pace, and a
working remote-rig path. It still starts the player at the track spawn, has no
race countdown, no live positions, and no finish condition.

The product goal is a small, local, non-authoritative race: player and AI start
from the grid, complete a fixed number of laps, see positions, and finish.
Online contact and netcode remain separate.

Race Loop grid rotations do not match the layout direction (TD-0021). The AI
already derives its heading from the nearest layout tangent. The player should
use the same runtime rule for this batch.

## Decision

1. **Player starts on the race grid.** The player uses `race.grid[0]`; AI uses
   `race.grid[1..]`. Both use the nearest layout tangent as their heading until
   track-ed grid rotations are fixed.

2. **Add grid-start semantics to `RaceTracker`.** A tracker can be armed for a
   grid start. The first forward start/finish crossing only begins lap 1 and
   does not increment the lap. Later forward crossings increment normally. The
   `RaceState` contract is unchanged.

3. **Race start is explicit.** The worker exposes `place_on_race_grid(slot)`
   and `start_race(laps)`. Before `start_race`, player input and AI controls
   are held neutral. After it, the selected lap count is active and AI cars
   stop racing after completing it.

4. **AI race state travels with the existing AI stream.** The `ai_state`
   message carries the existing 64-float vehicle frame plus the AI
   `RaceState` JSON. No SAB or snapshot layout change.

5. **The web owns the race director.** Countdown, race clock, standings,
   position HUD, and results are a web view model built from the local
   `RaceState` and AI `RaceState` values. Positions are ordered by completed
   distance and finish time. The worker remains the source of vehicle and AI
   lap data.

6. **Three laps by default.** The default race is three laps. A query override
   may be added for validation, but 3 is the product default.

7. **No contact or online authority.** Player and AI still do not collide.
   The relay is not involved. Contact remains a separate batch.

## Consequences

### Positive

- The race uses the existing layout, race manager, and AI field.
- Standings remain deterministic from lap/progress data.
- No new versioned contract or SAB layout revision is needed.
- The grid-start rule is reusable by future online spawns.

### Negative / risks

- The web now owns a race state machine in addition to the local tracker.
- The player and AI race states have different update rates; finish times are
  sampled.
- TD-0021 remains open; runtime tangent heading is a workaround.
- Results while AI are still running need a clear policy.

## Alternatives considered

- **Server-authoritative standings.** Rejected for this phase; no relay
  authority.
- **Move all race tracking into the worker.** Rejected for now: it would
  duplicate or replace the existing `DriveViewer` race-state contract.
- **Use grid rotations as exported.** Rejected: they do not match the layout on
  Race Loop; runtime tangent is consistent with AI.
- **Increment lap on the first grid crossing.** Rejected: it would show lap 2
  before the player completes a lap.
- **Race until all AI finish.** Rejected for the first slice: results appear
  when the player finishes, with current order for unfinished cars.

## Follow-up

- Fix track-ed grid-slot rotations and remove the runtime tangent workaround
  (TD-0021).
- Add a final results policy for unfinished AI and replay/restart.
- Add contact and netcode in the later online-contact batch.
