# 0021. Ten-car AI grid with nine AI opponents

- Status: accepted
- Date: 2026-11-24
- Deciders: architect, @joegaffey
- Related: ADR-0013, ADR-0014, ADR-0015, ADR-0019,
  `docs/features/ai-field.md`, `docs/features/online-contact.md`,
  `docs/features/track-race-mvp.md`

## Context

The AI field was deliberately capped at three opponents: `MAX_AI = 3` in the
worker, `?ai=0..3` in Drive, seven renderer remote rigs, and an 8-slot track-ed
grid. ADR-0019 recorded that the contact solver and race-state path are
count-agnostic and that raising the field is a limits-and-profiling change, not
a solver rewrite.

The product owner wants nine AI opponents. Nine AI plus the player is a ten-car
field, so the shipped grid must grow from eight slots to ten. The default field
remains three AI; nine is the new maximum.

## Decision

1. **Ten-slot race grid.** track-ed emits ten staggered logical grid slots, and
   `track-pack` accepts race grids of `1..=10`. Existing smaller grids remain
   valid.
2. **Nine AI maximum.** The physics worker caps `ai_count` at 9 and still
   clamps it to `race.grid.len() - 1`. Drive keeps the default at 3 and accepts
   `?ai=0..9`.
3. **Ten-car render budget.** The Drive viewer supports one local car plus nine
   remote rigs. The existing garage/scene path already supports ten bays.
4. **Effective field follows the grid.** The web layer clamps the requested AI
   count to the available grid so legacy 8-slot packs still produce a
   consistent standings view.
5. **No versioned contract change.** `RaceData`, `RaceState`, the SAB snapshot
   ABI, the net protocol, and the car setup schema are unchanged. The race grid
   is still a variable-length `Vec`/array; only its validated maximum changes.
6. **Regenerate race fixtures.** The Race Loop and Loop Spline Point Terrain
   fixtures are re-exported from track-ed and re-baked so both carry ten slots.
7. **Keep multiplayer separate.** Multiplayer remains a four-player relay field
   with three remote slots and zero AI.

## Consequences

### Positive

- A ten-car AI field works without changing the contact solver, race state,
  standings, results, or SAB layout.
- The default three-AI behavior and existing tests remain the common case.
- Runtime, renderer, compiler, and editor limits now agree.

### Negative / risks

- Contact work grows from 6 to 45 unordered pairs. Nine AI also add nine
  vehicle steps and a larger render/particle field.
- AI has no avoidance policy; a larger grid can produce more start crowding.
- Track-ed's exporter and the parent submodule pin must stay in sync.
- Legacy 8-slot packs remain valid but can only run seven AI; Drive clamps the
  effective field rather than showing phantom opponents.

## Alternatives considered

- **Keep eight slots and allow overlapping grid positions.** Rejected: it
  produces invalid starts and misleading spacing.
- **Raise only the runtime caps.** Rejected: shipped race tracks would still be
  limited to seven AI.
- **Jump directly to a 20+ car grid.** Deferred: it needs wider authoring,
  renderer, audio, and profiling work; nine AI is the requested step.

## Follow-up

- Measure 3 vs 9 AI on Race Loop: step p50/p99, dropped ticks, catch-up, FPS,
  and draw cost.
- File debt if the larger field materially reduces the 500 Hz physics budget or
  causes sustained catch-up.
- Keep TD-0021 open; grid heading remains a tangent workaround.
