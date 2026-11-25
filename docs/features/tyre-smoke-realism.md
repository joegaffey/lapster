# Tyre Smoke Realism

- **Status:** planned
- **Date:** 2026-11-25
- **Owner:** architect
- **Domain:** render
- **Branch:** `feat/tyre-smoke-realism` (to be created)
- **Related:** `crates/sim-render/src/particles.rs`,
  `crates/sim-render/src/tuning.rs`, `crates/sim-core/src/vehicle.rs`

## 1. Goal

Make tyre smoke appear only when the tyres are genuinely sliding, and keep
heavy smoke for real drifts, lockups and wheelspin. The current effect produces
too much smoke during ordinary hard cornering and side slip.

## 2. Diagnosis

Smoke is emitted from `crates/sim-render/src/particles.rs:138-152` using
defaults in `crates/sim-render/src/tuning.rs:318-442`.

- Lateral onset is `0.09 rad` (~5°), **below** the tyre peak-grip slip angle
  (`0.12 rad` in `crates/sim-core/src/vehicle.rs:1203`). Ordinary hard
  cornering therefore crosses it.
- `rate_scale = 2000`, `rate_cap = 210/s`, `max_spawn_per_step = 8` per wheel.
  Four wheels quickly saturate the 1024-particle cap.
- Puffs grow to 1.5-3.1 m quads, live 1.1-1.7 s, alpha `0.30`, so the smoke
  stacks and lingers.
- The only front-wheel gate is speed-based (`8 + 10 m/s`), so at track speed
  all four wheels emit equally.

## 3. Proposed Changes

### 3.1 Smooth, physical onset (`particles.rs`)

- Extract a pure helper, e.g. `wheel_smoke_rate(...)`, so the emission curve can
  be unit-tested.
- Compute excess beyond threshold, then shape it:
  - `excess = max(0, |slip| - threshold)`
  - `severity = excess.powf(onset_power)`
- Add a front-wheel weight (`front_rate_scale`) so understeer smoke is lighter
  than rear/driven-wheel smoke.
- Keep the accumulator and rolling-dust paths unchanged.

### 3.2 Retune defaults (`tuning.rs`)

| Setting | Now | Proposed starting point |
|---|---:|---:|
| `lateral_threshold` | 0.09 | **0.17 rad** (~10°, above peak grip) |
| `lateral_scale` | 1.5 | 1.1 |
| `longitudinal_threshold` | 0.14 | 0.18 |
| `onset_power` (new) | - | 2.0 |
| `front_rate_scale` (new) | - | 0.5 |
| `max_spawn_per_step` | 8 | 3 |
| asphalt `rate_scale` | 2000 | 5000 (higher because severity is squared) |
| asphalt `rate_cap` | 210 | 110 |
| asphalt start size | 0.10-0.17 | 0.08-0.14 |
| asphalt end size | 0.75-1.55 | 0.55-1.05 |
| asphalt life | 1.10-1.70 | 0.90-1.35 |
| asphalt alpha | 0.30 | 0.18 |
| `fade_out_start` | 0.55 | 0.45 |
| `grow_power` | 2.0 | 2.5 |

Outcome: near-threshold cornering emits almost nothing; smoke ramps up sharply
only for genuine slides, lockups and wheelspin. Loose-surface rolling dust
(grass/dirt/kerb) should be re-checked visually and only adjusted if the squared
onset makes it too weak.

### 3.3 Dev schema

- Add `particles.onsetPower` and `particles.frontRateScale` to
  `schema_fields()` so they can be swept live.
- No storage migration is needed: `TUNING_STORAGE_KEY` is written but not
  currently restored, so defaults apply on reload.

## 4. Tests

- Update `emits_smoke_only_above_skid_threshold`: normal-cornering slip
  (`~0.12`) emits **zero**; heavy slip (`~0.32`) still emits.
- Add a monotonic-onset test: moderate slide emits fewer particles than a heavy
  slide over the same time.
- Add a front-vs-rear test if `front_rate_scale` is added.
- Keep existing front low-speed gate, dirt/grass shape, cap, lifecycle and
  determinism tests.

## 5. Verification

- `cargo test -p sim-render`
- `cargo clippy --workspace --all-targets -- -D warnings`
- wasm check for `sim-render`
- Full gate once for the batch (`cargo fmt --check`, workspace tests,
  `npm --prefix web test`, viewer tests), since `crates/sim-render/**` is
  full-gate.
- Visual check on the dev preview skidpad: normal cornering -> no smoke;
  handbrake/drift -> strong smoke; brake lockup, grass, dirt and kerb unchanged.
  Sweep `lateralThreshold`, `onsetPower` and `asphalt.rateScale` in the dev menu
  before locking the defaults.

## 6. Ownership

Render worker owns `crates/sim-render/**`; no contract or web changes are
required. The architect reviews the manifest, runs the gate and commits.
