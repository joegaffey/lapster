# AI Motion Smoothing and Player Steering Rate

- **Status:** complete; full gate passed; PR pending
- **Date:** 2026-11-22
- **Owner:** architect
- **Domains:** architect, physics, web
- **Branch:** `feat/ai-motion-smoothing`
- **Related:** `docs/features/ai-race.md`, `docs/features/ai-field.md`,
  `docs/tech-debt.md`

## 1. Goal

Reduce visible AI jerk and make player steering response configurable.

The AI is simulated at 500 Hz but rendered from 62.5 Hz snapshots with no
interpolation. Player keyboard steering uses a frame-rate-dependent exponential
filter; wheel steering is direct.

## 2. Scope

- Add an AI steering-rate limit, default **3.0 units/s**.
- Publish AI frames at **125 Hz** (every 4 physics ticks).
- Interpolate AI frames in the web with a **40 ms** buffer.
- Add a player **Steering rate** setting, default **6.0 units/s**; `0` disables.
- Keep physical wheel steering direct.
- Add tests for the steering-rate helper and AI controller limiting.

Out of scope:

- AI throttle/brake hysteresis (revisit if still twitchy).
- Changes to `RaceState` or the 64-float snapshot.
- Car setup schema changes.
- Contact/netcode.

## 3. Interfaces

No versioned contract change.

```rust
impl AiDriver {
    pub fn set_steering_rate(&mut self, rate: f32);
    pub fn with_steering_rate(self, rate: f32) -> Self;
}
```

`steering_rate` is in normalized steering units per second. `0` disables the
limit. At 500 Hz the maximum change per tick is `rate / 500`.

Web input setting:

```ts
interface AssistSettings {
  // ...
  steeringRate: number; // units/s, 0 = off, default 6
}
```

```ts
rateLimitSteering(current, target, maxRate, dt): number
```

## 4. AI smoothing

- `AiDriver` stores the last steering command and moves it toward the desired
  command by at most `steering_rate * FIXED_DT` per physics tick.
- Reset clears the last steering command.
- The worker publishes AI frames every 4 ticks (`125 Hz`).
- The web keeps one `RemoteStateBuffer(40)` per AI slot and feeds it with the
  frame tick. The render loop samples the buffer each animation frame before
  `render_remote`.
- AI race state is still applied immediately for standings.

## 5. Player steering

- Replace the exponential keyboard/touch filter with a time-based rate limit.
- Use `rateLimitSteering(steer, target, playerSteeringRate, delta)`.
- Default 6 units/s; 0 means direct target.
- Physical wheel input remains direct.
- Persist the setting in the existing settings storage; add a Gameplay slider
  with a `0–20 units/s` range.

## 6. Tests

Physics:

- AI steering rate limits a large desired step.
- The second tick continues ramping toward the target.
- Rate `0` disables the limit.
- Reset clears the steering state.

Web:

- Unit test for `rateLimitSteering`: normal, zero rate, overshoot, and dt
  clamping.
- Existing AI field/race browser tests still pass.
- Race Loop still renders moving AI with no page errors.
- Settings slider writes and reloads the value.

## 7. Work breakdown

| Phase | Domain | Deliverable | Verify |
|---|---|---|---|
| 0 | architect | plan and progress checkpoint | `git diff --check`, agent sync |
| 1 | physics | `AiDriver` steering rate + tests | `cargo test -p sim-core`, clippy, wasm |
| 2 | web | 125 Hz AI frames, interpolation, player setting, tests | `npm --prefix web test`, viewer |
| 3 | architect | full gate, preview, PR | full gate |

## 8. Acceptance

- AI steering commands ramp at no more than 3 units/s.
- AI frames are published at 125 Hz and interpolated at 40 ms.
- Player keyboard/touch steering uses the configured rate; wheel is direct.
- Steering rate setting persists and defaults to 6 units/s.
- AI cars look smoother on Race Loop.
- Full gate passes.
