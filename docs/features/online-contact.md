# Online Contact — Co-Simulated Contact and Relay Observation

- **Status:** 3A in progress; 3B proposed (ADR-0020)
- **Date:** 2026-11-23
- **Owner:** architect
- **Domains:** architect, physics, render, web (3A); multiplayer, web (3B)
- **Branch:** `feat/online-contact`
- **Related:** ADR-0019, ADR-0020, ADR-0013, ADR-0014, ADR-0015, `GOALS.md`,
  `docs/features/ai-race.md`, `docs/multiplayer.md`, `docs/tech-debt.md`

## 1. Goal

Close the last item of ADR-0013 in two stages:

> **3A:** the player and up to three AI collide with each other in the physics
> worker, with bounded contact events and local feedback.
>
> **3B:** online clients can observe car-to-car contact through the relay
> without making the relay authoritative.

This batch implements 3A first. 3B keeps its design here but waits for the
product decision in ADR-0020.

## 2. Delivery staging

| Stage | Scope | Decision |
|---|---|---|
| 3A | player/AI contact solver, worker events, render effect, web feedback | ADR-0019 accepted |
| 3B | local-vs-ghost detection, relay `contact` messages, observer feedback | ADR-0020 proposed |

The field stays at three AI for 3A. A larger grid (20+) is a later product
direction; the solver is written to be field-size-agnostic so that change is a
cap, export, and profiling batch rather than a rewrite.

## 3. Scope

3A in scope:

- deterministic pairwise contact between the player and up to three AI;
- equal-and-opposite impulses, positional correction, restitution and friction
  clamps, and fixed resolution order;
- bounded contact events on the worker control channel;
- Drive feedback for contact (audio and a small visual effect);
- tests for the solver, worker events, and web feedback.

3A out of scope:

- relay or online changes;
- physical response to remote ghosts;
- damage, deformation, penalties, or race-state changes from contact;
- AI avoidance or contact-aware driving;
- main-thread `?physics=main` contact (the fallback keeps single-vehicle
  behaviour);
- raising the AI cap or fixing TD-0021.

3B in scope (later):

- client-side detection between the local car and interpolated remote ghosts;
- additive relay `contact` messages with stamping, validation, and rate
  limiting;
- observer feedback for remote contact.

3B out of scope:

- relay-authoritative physics or server correction;
- physical response to remote ghosts;
- anti-cheat, authentication, or competitive ranking.

## 4. Contracts and interfaces

### 4.1 No versioned contract change for 3A

The net protocol, asset schema, setup schema, 64-float snapshot, `RaceData`, and
`RaceState` are unchanged. The new `sim-core` contact API is an additive core
interface owned by physics.

### 4.2 Core contact API (physics-owned)

`sim-core` gains a deterministic pairwise resolver, roughly:

```rust
pub struct CarContact {
    pub a: u32,            // 0 = player, 1..=n = AI slot
    pub b: u32,
    pub point: Vec3,
    pub normal: Vec3,
    pub relative_speed: f32,
    pub tick: u64,
}

pub fn resolve_car_contact(a: &mut Vehicle, b: &mut Vehicle) -> Option<CarContact>;
```

The exact accessors are physics' choice. The resolver must not expose
`Vehicle` internals publicly, must keep the existing track contact path intact,
and must not hard-code the number of AI cars.

### 4.3 Worker control channel (internal interface)

```text
// worker -> main thread, alongside ai_states
{ "type": "contacts", "contacts": [
  { "a": 0, "b": 1, "point": [x, y, z], "normal": [x, y, z],
    "speed": 3.2, "tick": 1234 }
] }
```

- drain and publish at the `ai_states` cadence (62.5 Hz) and after
  reset/place/start;
- at most one event per pair per 100 ms;
- omit the message when the drained list is empty.

```ts
// web client
PhysicsClient.onContact: ((contact: ContactEvent) => void) | null;
interface ContactEvent {
  a: number;
  b: number;
  point: [number, number, number];
  normal: [number, number, number];
  speed: number;
  tick: number;
}
```

### 4.4 Render (render-owned)

A small API to show a contact at a world point, reusing the existing particle
machinery where practical:

```rust
DriveViewer::contact_effect(x: f32, y: f32, z: f32, speed: f32);
```

It must be cheap, bounded, and a no-op when the existing Tyre effects setting
is `Off`. No snapshot or asset change.

### 4.5 3B net protocol (follow-up, not this batch)

Canonical `crates/relay-server/src/protocol.rs`, mirror
`web/src/net/protocol.ts`. Add one client and one server variant:

```json
// client -> server
{ "type": "contact", "seq": 7, "tick": 812345, "other_player_id": 2,
  "point": [12.5, 0.4, -3.1], "normal": [0.2, 0.0, 0.98], "speed": 4.1 }

// server -> other clients
{ "type": "contact", "player_id": 1, "other_player_id": 2, "seq": 7,
  "tick": 812345, "point": [12.5, 0.4, -3.1],
  "normal": [0.2, 0.0, 0.98], "speed": 4.1 }
```

The server stamps the sender identity and never trusts a client-supplied
`player_id`. Additive variants only. Old web clients ignore unknown server
messages; the relay rejects unknown client messages, so relay and client ship
together.

## 5. Co-simulated contact model (3A)

### 5.1 Detection

- Reuse the environment/wall collision pattern instead of a single SAT
  normal: each car keeps the same eight body probe points used for wall
  collision (front/rear centre, four corners, two side midpoints), placed on
  its own per-car contact box.
- The box extents come from the car's `CarPhysics`, not fixed constants:
  half-length from the wheelbase plus a front/rear overhang, and half-width
  from the widest track width plus a tire/mirror margin. Clamp both to sane
  car-sized ranges and add a small contact skin.
- For each pair, test A's probes against B's box and B's probes against A's
  box in a fixed order. Each probe produces a closest point, a normal, and a
  penetration; a small probe radius keeps the hull just clear of the other car.
- A vertical gate of about 1.0 m keeps cars on different terrain heights from
  colliding.
- Skip pairs that are clearly apart with a cheap radius check first.

### 5.2 Response

- Apply a normal impulse at each probe contact using the combined inverse mass
  and inverse inertia, only when the relative normal velocity is closing.
- Scale the angular part of the contact impulse by a tire-yaw resistance factor
  (default 0.5) so a side hit at the front still pushes the car but does not
  spin it like a free rigid body. The linear impulse is unchanged; the angular
  scale is a feel constant, not a physical tire model.
- Restitution clamp around 0.05, tangential friction clamp around 0.4, and a
  small penetration slop.
- Accumulate positional correction from the deepest contacts and split it by
  inverse mass, with a per-tick cap so multiple probes cannot over-correct.
- Fixed probe order, pair order, and at most two passes per tick so the result
  is deterministic for a given input history.
- Keep the existing per-vehicle track and wall resolution unchanged; it runs
  after the pairwise pass.

### 5.3 Worker integration

- After stepping the player and AI each tick, resolve all pairs in slot order.
  With four cars that is `(0,1)`, `(0,2)`, `(0,3)`, `(1,2)`, `(1,3)`, `(2,3)`.
- Reset, place-on-grid, and race start clear pending events and cooldowns.
- `ai_count == 0` skips the pass entirely.
- `simulate()` uses the same pass so tests and replays agree with `step()`.

## 6. Web feedback (3A)

- `PhysicsClient` parses `contacts` and calls `onContact`.
- Drive shows the render effect at the point, plays a short impact sound, and
  keeps a session contact count for debugging.
- No HUD requirement beyond the existing race panel; the effect and audio are
  the observable feedback.
- `?physics=main` and Skidpad are unchanged.

## 7. Online observation (3B)

- Only when `mp=1` and the relay is connected.
- Each animation frame, after the local snapshot and each remote buffer frame
  are available, test the local car against every remote car with the same XZ
  footprint test.
- Debounce per remote player (about 250 ms) and ignore contacts below a small
  relative-speed threshold.
- Send a `contact` message with the local tick and the remote frame tick.
- On receiving a `contact` from another player, render `contact_effect` at the
  point, play a short impact sound, and count it in telemetry.
- No remote state is modified and no contact changes race state.

## 8. Tests

3A physics:

- Head-on overlap produces a separating impulse and finite state.
- Side and rear contacts choose a sensible normal and separate the boxes.
- Angled corner and offset contacts produce separating normals at the probe
  points rather than one shove direction.
- A front-side hit produces a bounded yaw response; the hit car must not spin
  like a free rigid body.
- A resting overlap settles without jitter or energy growth.
- Pair resolution is symmetric in magnitude and deterministic in order.
- Separated cars or a large vertical offset produce no contact.
- Existing track and wall contact tests still pass.

3A worker:

- `?ai=3` player-to-AI and AI-to-AI contacts emit bounded events.
- Events respect the per-pair cooldown.
- `ai_count == 0` emits no contact events.
- Reset/place/start clear events; wasm builds.

3A web:

- `PhysicsClient` parses `contacts` and ignores malformed entries.
- A contact event drives the viewer effect and impact sound path.
- No page errors on Race Loop; existing AI race/field/motion tests stay green.

3B relay (later):

- A valid contact is forwarded to other room members with the stamped sender.
- Self-target, unknown target, non-finite, oversized, and repeated events are
  rejected or dropped by the rate limit.
- `state`, `welcome`, and roster behaviour are unchanged.

3B web (later):

- Protocol parse accepts `contact` and still ignores unknown types.
- Local-vs-remote detection produces the expected normal sign and respects the
  debounce.
- Contact events do not change `RaceDirector` state or results.
- Two-browser manual probe shows feedback on both clients without correction.

## 9. Work breakdown

3A:

| Phase | Domain | Deliverable | Verify |
|---|---|---|---|
| 1 | physics | `sim-core` contact solver, worker pairwise pass + `contacts` drain, tests | `cargo test -p sim-core -p sim-worker`, clippy, wasm |
| 2 | render | `contact_effect` API and effect budget | `cargo test -p sim-render`, clippy, wasm |
| 3 | web | `contacts` handling, feedback, tests | `npm --prefix web test`, viewer |
| 4 | architect | full gate, preview validation, PR | full gate |

3B:

| Phase | Domain | Deliverable | Verify |
|---|---|---|---|
| 5 | architect | ADR-0020 decision, net-protocol canonical + mirror | contract gate |
| 6 | multiplayer | relay `contact` validation, stamping, rate limit, forwarding, tests | `cargo test -p relay-server`, clippy |
| 7 | web | online detection, protocol handling, telemetry, tests | `npm --prefix web test`, viewer |
| 8 | architect | full gate, preview validation, PR | full gate |

Dependencies:

- 3A physics and render are independent.
- 3A web depends on the worker message and the render API.
- 3B waits for 3A acceptance and the ADR-0020 decision.

## 10. Acceptance

3A:

- On Race Loop, the player can bump AI cars and both cars react; cars no longer
  pass through each other.
- AI cars collide with each other.
- Contact is stable: no explosions, no sustained jitter, and states stay
  finite.
- Worker contact events are bounded and reach the web.
- Contact never changes laps, positions, finish order, or results.
- `?ai=0` and `?physics=main` are unchanged; Skidpad stays AI-free.
- Full gate passes.

3B:

- In multiplayer, contact between the local car and a remote ghost is observed
  on both clients with visual and audio feedback.
- The relay stamps identity, validates, and rate-limits contact messages.
- No remote car is corrected and race state is unchanged.
- Full gate passes.

## 11. Measured performance

To fill in after 3A:

- three AI with contact on Race Loop: `stepP50Ms`, `stepP99Ms`, dropped ticks,
  catch-up events after warm-up;
- contact pass time alone.

After 3B:

- web contact detection cost per frame with three remote ghosts.

## 12. Future grid size

The product direction is a larger grid later (20+ cars). This batch keeps the
solver count-agnostic but does not raise the current caps:

| Limit | Current | Needed for 20+ |
|---|---|---|
| worker `MAX_AI` | 3 | derive from grid slots |
| web AI field | `?ai=0..3` | widened URL/UI and count-agnostic results |
| renderer remote rigs | `min(7)` | remote-slot and budget work |
| track-ed grid export | 8 slots | larger grid authoring/export |
| relay clients | 4 default | slot and bandwidth work |

A future grid batch also needs profiling for physics catch-up, draw calls,
particles, and audio voices. It is not blocked by the contact solver design.

## 13. Risks and open questions

- **3B product decision:** is observation-only online contact acceptable, or is
  a local physical response to remote cars required? ADR-0020 proposes
  observation-only and defers proxies.
- Online observations can disagree; accepted while the relay is
  non-authoritative.
- Web detection uses interpolated 30 Hz snapshots; grazes can be missed and
  slow overlaps can repeat.
- AI contact can cause pile-ups because there is no avoidance policy; tune
  impulses after the first green run, not before.
- Impulse tuning can cause jitter or energy injection; keep clamps and add
  regression tests before tuning feel.
- TD-0021 still makes the grid heading a workaround.
- Raising the grid is a separate profiling and limits batch.

## 14. Documentation updates on completion

- `docs/multiplayer.md` after 3B: contact message, validation, and the
  observation limitation.
- `PROGRESS.md`: accepted result, measured cost, and next step.
- `docs/tech-debt.md`: any deferred tuning or detection gap.
