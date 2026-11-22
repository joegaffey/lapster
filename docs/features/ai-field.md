# AI Field — Three Opponents, Pace, and Performance

- **Status:** in progress
- **Date:** 2026-11-22
- **Owner:** architect
- **Domains:** architect, physics, web
- **Branch:** `feat/ai-field`
- **Related:** ADR-0013, ADR-0014, `docs/features/ai-opponents.md`,
  `docs/tech-debt.md`

## 1. Goal

Turn the single AI opponent into a small race field:

> A race-ready track starts three AI opponents by default. AI pace is
> selectable without changing the car or track contract. The Drive viewer can
> request zero to three AI for testing, and worker timing remains visible.

This batch is about field size, pace, and performance. It does not add race
standings, countdown, results, car-to-car contact, or netcode.

## 2. Scope

In scope:

- three AI opponents by default on race-ready tracks;
- `?ai=0..3` to request a field size for testing;
- `?aiPace=chill|club|fast` to select a pace preset, default `club`;
- AI pace scaling for target speed and cornering budget;
- worker diagnostics that report the active AI count;
- browser coverage for 3 AI, 1 AI, and 0 AI;
- a manual performance comparison for 1 vs 3 AI using worker step timing.

Out of scope:

- race positions/standings, countdown, results, lap limit;
- player start on the race grid;
- car-to-car contact or avoidance;
- AI difficulty UI beyond URL parameters;
- per-AI pace overrides;
- racing-line optimisation or lap-time tuning;
- online contact/netcode.

## 3. Interfaces

No versioned contract change. Reuse `RaceData`, `RaceState`, `SNAPSHOT_STATE`,
and the existing `ai_state` message.

Internal interfaces:

```rust
// sim-core
pub enum AiPace {
    Chill,
    Club,
    Fast,
}

impl AiPace {
    pub fn from_index(index: u32) -> AiPace; // unknown -> Club
}

// sim-worker wasm API
Simulation::new(track_pack_bytes, car_pack_bytes, ai_count: u32, ai_pace: u32)
```

```text
// worker init
{ "type": "init", "aiCount": 3, "aiPace": 1, ... }
```

```ts
// web client
PhysicsClient.create(sab, trackBytes, carBytes, aiCount = 0, aiPace = 1)
PhysicsDiagnostics.aiCount: number
```

Pace indices are `0 = chill`, `1 = club`, `2 = fast`. The worker clamps the AI
count to `0..=min(3, grid.len() - 1)` and maps unknown pace indices to `club`.

## 4. Pace behavior

`AiDriver` receives an `AiPace` and scales its existing conservative limits:

| Pace | Target speed | Lateral accel |
|---|---|---|
| Chill | 0.85x | 0.75x |
| Club | 1.00x | 1.00x |
| Fast | 1.12x | 1.22x |

The controller still uses layout curvature and the same look-ahead. Pace must
not change the car physics, collider, setup, or assists. Fast AI must remain on
the road for the tested Race Loop window.

## 5. Web behavior

- Single-player race-ready tracks default to `ai=3` and `aiPace=club`.
- `?ai=0..3` overrides the field size; invalid or out-of-range values clamp.
- `?aiPace=chill|club|fast` overrides pace; invalid values fall back to `club`.
- Skidpad and other race-unavailable tracks always request zero AI.
- Multiplayer keeps its existing three remote slots and requests zero AI.
- The race panel shows a compact `AI N` field when AI is active.
- `telemetry.remoteSlots` remains the field-size source for tests.

## 6. Tests

Physics:

- `AiPace::from_index` maps known indices and falls back to `Club`.
- Chill/Club/Fast change target speed monotonically on the same layout/state.
- Pace does not introduce non-finite controls.
- Worker compiles for wasm with the four-argument constructor.

Web:

- Race Loop defaults to three AI cars and `remoteSlots = 3`.
- All three AI cars move and stay within `half_width + 1.0` m of the layout.
- `?ai=1` shows one AI; `?ai=0` shows none.
- `?aiPace=fast` still keeps the AI on the layout for the tested window.
- Skidpad remains at zero AI and zero remote slots.
- Worker diagnostics include `aiCount = 3` on the default race field.
- No page errors.

## 7. Performance

Measure the normal worker path on Race Loop:

- `?ai=1&aiPace=club`
- `?ai=3&aiPace=club`

Record `stepP50Ms`, `stepP99Ms`, dropped ticks, and catch-up events after a
warm-up. The target is not a hard threshold in this batch; record the result
and file debt if three AI materially reduce the physics budget or cause
sustained catch-up.

## 8. Work breakdown

| Phase | Domain | Deliverable | Verify |
|---|---|---|---|
| 0 | architect | plan and progress checkpoint | `git diff --check`, agent sync |
| 1 | physics | `AiPace`, driver scaling, four-argument worker constructor | `cargo test -p sim-core -p sim-worker`, clippy, wasm |
| 2 | web | field size/pace URL handling, diagnostics, HUD count, browser tests | `npm --prefix web test`, viewer |
| 3 | architect | performance measurement, full gate, preview, PR | full gate |

## 9. Acceptance

- Race Loop starts three AI at Club pace by default.
- The three AI visibly move and remain on the road for the tested window.
- `?ai=1`, `?ai=0`, and `?aiPace=fast` behave as specified.
- Skidpad remains AI-free.
- Worker diagnostics expose the AI count.
- Player race state and existing AI behavior remain unchanged.
- Full gate passes.

## 10. Risks

- Three AI increase worker catch-up pressure; measure before adding a fourth.
- Identical pace can keep the field bunched without contact; later race work
  should add pace/difficulty selection and standings.
- The AI still uses the layout tangent workaround for TD-0021.
