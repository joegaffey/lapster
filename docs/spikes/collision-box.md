# Spike: collision-box

## Objective

Prototype one planar collision box per car, derived from the baked car visual
AABB, and use it for **both** car-to-car contact and track/scenery wall contact.
Add a `?collisionDebug=1` wireframe overlay so the product owner can confirm the
box matches the rendered body before deciding whether to promote.

No pack-schema, SAB ABI, or contract-file changes are in scope. The existing
wheelbase/track heuristic remains the fallback for callers that do not install a
box.

## Interfaces tested

- `crates/sim-core/src/collision_box.rs` (new) — `CollisionBox` derived from
  `sim_types::Aabb`, plus the legacy physics fallback.
- `crates/sim-core/src/vehicle.rs` — box installation API and dynamic
  wall-probe placement.
- `crates/sim-core/src/contact.rs` — car-to-car `BodyBox` extents and centre.
- `crates/sim-worker/src/lib.rs` — worker path installs the box for the player
  and every AI from the car pack manifest.
- `crates/sim-render/src/lib.rs` — `DriveViewer::new` installs the box on the
  main-thread path and draws the debug overlay.
- `crates/sim-render/src/renderer.rs` — reusable dynamic debug-line buffer.
- `web/drive/main.ts` — `?collisionDebug=1` query flag.

No contract files were edited. The pack still exposes the same
`AssetManifest.bounds`; the box is derived at load time.

## Proposed changes

- Branch: `spike/collision-box`
- Implementation commit: `28fbbbc` (plus the report commit that contains this
  file).
- Files:
  - `crates/sim-core/src/collision_box.rs` (new)
  - `crates/sim-core/src/contact.rs`
  - `crates/sim-core/src/lib.rs`
  - `crates/sim-core/src/vehicle.rs`
  - `crates/sim-render/src/lib.rs`
  - `crates/sim-render/src/renderer.rs`
  - `crates/sim-worker/src/lib.rs`
  - `web/drive/main.ts`
  - `docs/spikes/collision-box.md`
- Summary:
  - Added `CollisionBox { half_width, half_length, center_x, center_z }`.
    `CollisionBox::from_aabb` uses the visual AABB XZ extents and centre and
    adds a `COLLISION_SKIN` of `0.02 m`. `CollisionBox::from_car_physics`
    reproduces the previous wheelbase/track heuristic for callers that never
    install a box.
  - `Vehicle` stores the box, exposes `set_collision_box` / `collision_box`,
    and `contact_half_extents` now reads the installed box.
  - Wall/scenery probes are generated from the box via `wall_probe_offsets()`:
    eight perimeter probes inset by `WALL_PROBE_RADIUS` (`0.45 m`) so the swept
    circle edge matches the box, plus three interior probes. The box centre
    offset is included, so an asymmetric overhang shifts the whole probe set.
  - Car-to-car `BodyBox` uses the same box extents and rotates its local XZ
    centre into world space.
  - The worker computes the box once from `car_pack.manifest().bounds` and
    installs it on the player and all AI. `DriveViewer::new` does the same from
    the loaded car pack on the main-thread path.
  - Added an on-demand dynamic debug-line buffer to the renderer and
    `set_collision_debug(bool)` on `DriveViewer`. It draws the 12 edges of the
    box at local `y = 0.1 m` and `y = 1.2 m` for the local player and every
    visible AI/remote state.
  - `web/drive/main.ts` reads `?collisionDebug=1`; the overlay is off by
    default and inert when disabled.

## Build result

`npm --prefix web run build:viewer` passed in the worktree. The final run after
formatting and warning cleanup compiled `sim-render`, `sim-worker`,
`track-pack-wasm`, and all esbuild bundles with no warnings.

No tests were added or run, per the spike brief.

## Visual-debug instructions

Serve the worktree (the architect serves it; this spike does not start a
server) and open:

```text
/web/drive/?track=race-loop&ai=3&collisionDebug=1
/web/drive/?track=race-loop&ai=3&car=sc-deckard-conejo&collisionDebug=1
/web/drive/?track=race-loop&ai=3&car=67gp-guthrie-eagle&collisionDebug=1
/web/drive/?track=race-loop&ai=3&spectate=1&collisionDebug=1
```

Dev-preview equivalents are the same paths under
`https://joegaffey.com/lapster-dev/`, for example
`https://joegaffey.com/lapster-dev/web/drive/?track=race-loop&ai=3&collisionDebug=1`.

The overlay draws bright-green 3D wireframes for the local player and all three
AI cars. `track=race-loop` is the intended baseline; use the `car=` queries
above to compare RX-7, Deckard, and Eagle, and the `spectate=1` query to follow
an AI car.

Suggested checks:

- RX-7: box footprint should visually match the body, and wall/scenery feel
  should stay close to the current hard-coded footprint.
- Deckard: the box should shift backward with the rear overhang rather than
  staying centred on the vehicle origin.
- Eagle: box width should reflect the wider body in both car-to-car and wall
  contact.
- Toggle the query off and confirm the overlay and normal rendering are
  unchanged.

## Prototype shortcuts

- The same local car box is used for the player, AI, and every remote slot. In
  solo AI that is correct (one car pack); in multiplayer a remote car with a
  different pack would still use the local box.
- The box is derived from the whole visual AABB, so mirrors, spoilers, or other
  widest visual features can widen the physical footprint.
- The `0.02 m` skin is folded into the box itself, so both car-to-car and wall
  contact sit slightly outside the rendered body.
- Wall contact keeps the existing circle-probe resolver and fixed `0.45 m`
  probe radius; only the probe centres derive from the box. Very small boxes
  collapse the inset probes toward the centre.
- The debug overlay is a visual extrusion (local `y = 0.1..1.2 m`) of a planar
  box; the vertical extent is not part of collision.
- The overlay is refreshed from `DriveViewer::update_visuals`. Intermediate
  `set_remote_state` renders in the worker path may draw the previous frame's
  lines until the local `render_remote` pass, which is the frame that is
  displayed.

## Findings

- No pack schema or SAB ABI change is needed: `AssetManifest.bounds` already
  gives the visual AABB in the vehicle-local frame.
- With the RX-7 manifest AABB, the inset-perimeter construction reproduces the
  previous effective wall footprint closely, while adding the local Z centre
  offset for asymmetric cars.
- The same box is now the single source of truth for wall probes and car-to-car
  extents, so future tuning only has to happen in one place.
- The debug buffer is reused across frames and only uploaded when enabled, so
  the overlay is cheap and does not rebuild scene meshes.
- Wall probes remain the wall-contact mechanism; the box only places/insets
  them. Probe count is unchanged (8 perimeter + 3 interior = 11 per car per
  tick), so runtime cost is unchanged; the debug overlay costs only when
  enabled.
- The fallback box preserves the old car-to-car heuristic for tests and any
  caller that never installs a box.

## Risks

- Wall/scenery feel can change for cars whose visual AABB is larger than the
  old `0.45 + 0.45` / `1.65 + 0.45` footprint, especially around tight walls
  and posts.
- The visual AABB may include mirrors or trim that make the physical box wider
  than the product owner expects; this is exactly what the wireframe should
  reveal.
- The centre offset is applied in both wall probes and car-to-car contact. A
  bad manifest origin would shift both systems consistently but incorrectly.
- Multiplayer remote cars with different packs still use the local box; visual
  overlap is possible until per-remote boxes are supplied.
- No automated tests were added or run; this is a visual-first prototype.
- The debug flag is query-string only (no dev-menu toggle) and is not persisted.

## Recommendation

`test`

Validate visually on the dev preview / served worktree before promoting. If the
box shape is accepted, promote through the strict workflow with tests for AABB
derivation, centre-offset contact, and wall probes, and consider per-remote
boxes for multiplayer.

## Open questions

- Should the collision box use a tighter body-only AABB instead of the whole
  visual AABB (excluding mirrors/spoilers)?
- Should the wall probe radius scale with box size instead of staying at
  `0.45 m`?
- Should the `0.02 m` skin stay inside `CollisionBox` or move to each contact
  resolver?
- Should multiplayer remote cars carry their own collision box in the state
  protocol, and is that a contract change?
- Is the current wall/scenery feel on RX-7 acceptable after switching from the
  hard-coded footprint to the manifest AABB?
- Should the overlay become a dev-menu toggle (and persist) after promotion?

## Architect sign-off

- Architect reviewed `28fbbbc` (AABB derivation, worker/main-thread wiring, shared car/wall box, debug overlay).
- `npm --prefix web run build:viewer` re-run by the architect: clean.
- `cargo fmt --check` and `git diff --check` pass.
- Product owner accepted the prototype direction after local RX-7/Deckard visual checks.
- Wall probes stay: they are the wall-contact mechanism; the box only places/insets them. Same 11 probes per car per tick, so cost is unchanged.
- Recommendation: `test` — push `spike/collision-box` and deploy the dev preview with `ref=spike/collision-box`.
