# 0019. Co-simulated car-to-car contact in the physics worker

- Status: accepted
- Date: 2026-11-23
- Deciders: architect, @joegaffey
- Related: ADR-0013, ADR-0014, ADR-0015, `GOALS.md`,
  `docs/features/online-contact.md`, `docs/features/ai-race.md`

## Context

ADR-0013 set online contact as the third batch after the race manager and AI
racing. The AI batches are merged: the physics worker steps the player and up
to three AI at 500 Hz, and all four are physical vehicles with a track collider.
They still do not collide with each other; ADR-0014 decision 6 deferred that
to this batch.

The worker owns every car in this field, so contact between them can be a real
deterministic part of the fixed step. That is separate from multiplayer, where
clients own different cars and the relay is not a physics authority; the online
observation layer is ADR-0020.

The current field is deliberately small: `MAX_AI = 3`, matching the four-player
remote-slot budget. Larger grids are a later product direction; this ADR keeps
the solver count-agnostic so a larger field is a cap and profiling change, not
a redesign.

## Decision

1. **Resolve co-simulated contact in `sim-core`, inside the worker.** Add a
   deterministic pairwise contact pass for the player and the AI. It runs after
   the vehicles step, in a fixed pair order, and applies equal and opposite
   impulses plus positional correction. All cars share the same footprint and
   mass properties as their `Vehicle`.

2. **Keep contact bounded and stable.** Fixed iteration count, restitution and
   tangential friction clamps, penetration slop, and one event per pair per
   cooldown window. No randomness and no wall-clock input. Track and wall
   collision stays per vehicle and is resolved as it is today.

3. **Keep the pair pass field-size-agnostic.** The current four-car field has
   six pairs; the loop is `O(n²)` over the simulated cars and must not hard-code
   three AI. Raising the cap to a 20+ car grid is a later change that adds
   profiling, grid-slot export, and renderer/web limits, not a new solver.

4. **Publish bounded contact events on the existing worker control channel.**
   Publish at most one event per pair per 100 ms alongside `ai_states`. The
   64-float snapshot, SAB ABI, `RaceData`, and `RaceState` are unchanged.

5. **Never feed contact into race state.** Contact changes vehicle motion, but
   observations must not change laps, sectors, positions, finish order, or
   results.

6. **Do not add relay or online behavior in this decision.** Online contact
   observation, its additive protocol messages, and the ghost limitation are
   ADR-0020.

7. **No versioned contract change.** The net protocol, asset schema, and setup
   schema are untouched. The new `sim-core` contact API is an additive public
   core interface owned by physics.

## Consequences

### Positive

- The player and AI physically collide on the same deterministic fixed step.
- Race Loop stops being an interpenetrating field without touching netcode.
- No SAB, snapshot, pack, or relay change is needed for this batch.
- Contact cost is small: six pairs at four cars, and the pass scales with the
  square of the field.

### Negative / risks

- Impulses can inject energy or jitter if restitution, friction, and
  penetration handling are not clamped; this needs focused tests.
- AI has no avoidance policy, so contact can cause spins or pile-ups at the
  start. That is expected for this batch; avoidance is later work.
- Raised grids will increase pairs, worker catch-up, renderer rigs, and audio
  voices; none of that is measured here.
- The main-thread `?physics=main` fallback keeps single-vehicle behaviour.

## Alternatives considered

- **Visual-only contact for the player and AI.** Rejected: it would leave
  obvious interpenetration and undermine the AI race.
- **Kinematic separation with no impulses.** Rejected: it would not exercise the
  vehicle model and would feel weightless.
- **A full rigid-body engine for car contact.** Rejected for now: the custom
  vehicle model and collider are the project constraint, and pairwise impulse
  contact is sufficient for four cars.
- **Reuse the snapshot wheel-contact flags.** Rejected: those flags describe
  terrain wheel contact, not car-to-car contact, and the snapshot should stay
  unchanged.
- **Do online contact in the same batch.** Deferred to ADR-0020 so the AI
  contact path can be proven first.

## Follow-up

- Implement 3A in `docs/features/online-contact.md`: solver, worker pass,
  event drain, render effect, and web feedback.
- Measure worst-case worker cost with three AI and contact enabled.
- Keep TD-0021 open; grid heading still affects start crowding.
- Revisit the field cap after 3A; a 20+ grid needs a profiling and limits
  batch, not a solver rewrite.
- Implement online observation under ADR-0020 once 3A is accepted.
