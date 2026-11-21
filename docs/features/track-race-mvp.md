# Track Race MVP — Layout as Race Route and the Race Chunk

- **Status:** in progress; runtime exposure complete, track-ed export pending
- **Date:** 2026-11-21
- **Owner:** architect
- **Domains:** architect, assets, editor, render, web
- **Related:** ADR-0012, ADR-0011, ADR-0008, ADR-0006, `docs/track-ed-integration.md`,
  `docs/multiplayer.md`, `agents/contracts.md`

## 1. Context

Lapster is currently a solo hotlap simulator. The runtime track pack carries
visuals, collision, metadata, spawn, surfaces, and sectors, but no usable lap
route. Track-ed bakes a visual racing line per section, but it has no
start/finish or layout concept. The physics worker runs one vehicle, and the
relay multiplayer prototype treats remote cars as ghosts.

AI racing and online contact need a common track-side foundation:

- an ordered, closed course for AI and lap progress;
- a start/finish and a deterministic starting grid;
- sectors or checkpoints for lap validation and race positions;
- a track identity that all online clients can compare;
- collision/surface data that is identical across clients.

This plan defines only that foundation. AI driving, race management,
car-to-car collision, and contact netcode are follow-up batches.

## 2. Goal

Make the track pack the shared source of truth for racing:

> A compiled track pack can carry a validated race layout: an ordered centerline
> path plus the start/finish, grid, and sector metadata that race code needs.
> AI and online race code consume that contract instead of reinterpreting track
> geometry.

## 3. Model

A layout **is** the race route. The layout is the course; the baked path is its
runtime representation. They are not two separate domain objects.

The useful ownership boundaries are:

| Concept | What it is | Owner |
|---|---|---|
| Track section | Physical road geometry, kerbs, walls | track-ed bake |
| Start/finish marker | Physical/timing line on a section | track section |
| Grid markings | Painted grid boxes on a section | track section |
| **Layout / race route** | Ordered course + direction, with a baked centerline path and width | layout selection |
| Grid slots | Logical race-start positions derived from the start/finish and road width | race setup |
| Sectors | Timing metadata along the course | race setup |
| Racing line | Preferred AI/visual line inside the road | AI/render |

Consequences:

- `start_finish`, grid markings, and sectors are **not** properties of the
  layout path.
- The start/finish marker is anchored to a section as
  `section_id + fraction`. A layout selects which marker it uses.
- Painted grid boxes are section decorations. Logical grid slots are race setup
  and may be derived.
- A layout may be rotated to start at the start/finish for runtime convenience,
  but the line's source of truth is the track section.
- The visual racing line remains separate from the layout path. The racing line
  is an AI/visual preference, not the lap reference.

For the MVP, the layout is implicit:

- Use the spawn's closed centerline as a one-leg layout.
- Use the spawn's section and fraction as the section-local start/finish anchor.
- Derive the grid slots from that anchor and the road width.
- Generate four equal sectors.

No track-ed project schema change is required for this MVP.

### 3.1 Current implementation state

- Branch: `feat/track-race-mvp`, created from `dev`.
- Documentation commit: `6fea5dd` (`docs: add track race MVP plan and ADR`).
- `origin/dev` is merged in as `6e94c69` (PR #43, 67GP Guthrie Eagle car);
  there were no conflicts with this branch or the foundation work.
- The contract/compiler foundation is committed on this branch.
  - Canonical contract: `crates/sim-types/src/race.rs`,
    `crates/sim-types/src/asset.rs`, `crates/sim-types/src/lib.rs`.
  - Runtime accessor: `crates/sim-asset/src/lib.rs`
    (`Pack::track_race_owned`).
  - Compiler: `crates/track-pack/src/lib.rs` accepts
    `sim-track-glb-v1`/`v2`, validates optional race data, and writes the
    Race chunk and entrypoint.
  - Mechanical fixes: `crates/asset-compiler` and
    `crates/sim-render/src/track_editor.rs` set `race: None`.
  - Web loader: `web/src/assets/loader.ts` accepts the optional Race
    entrypoint.
  - TS mirror: `web/src/contracts/race.ts`.
  - Runtime exposure: `DriveViewer.race()` plus `raceAvailable`, `layoutId`,
    and `trackSourceHash` in telemetry.
  - Web identity: `trackIdentity()` in the pack loader, `RaceView` /
    `parseRaceView`, and contract tests.
  - Architect role card/adapter: `web/src/contracts/**` is architect-owned;
    the generic permission was confirmed after the server restart.
- After the dev merge and the runtime exposure batch, the full gate passed:
  `cargo fmt --check`, `cargo test --workspace`, workspace clippy, wasm check,
  `npm --prefix web test`, `CI=1 npm --prefix web run test:viewer`
  (105 passed, 1 skipped), the WebGL2 fallback tests, and
  `python3 scripts/check-agent-sync.py`.
- Resolved: the TS mirror exists, the generic contract-root permission works,
  and the grid-inside-road and start/finish-fraction validations are in place.
- Next: the track-ed implicit-layout export and the race-ready demo fixture.
  Track-ed's own `AGENTS.md` currently lists routes/laps as out of scope, so
  that batch must record the scope extension in the track-ed repository first.
- The race route is available to the runtime, but no shipped track produces
  one yet, so `race()` reports unavailable on existing tracks.

## 4. Contract

### 4.1 Pack data

Add an optional `Race` chunk carrying `RaceData`:

```rust
pub const RACE_DATA_VERSION: u16 = 1;

pub struct RaceData {
    pub version: u16,
    pub layout: Layout,
    pub start_finish: StartFinish,
    pub grid: Vec<SpawnPoint>,
    pub sectors: Vec<SectorSpec>,
}

pub struct Layout {
    pub layout_id: String,
    pub closed: bool,
    pub direction: LayoutDirection,
    pub points: Vec<[f32; 3]>,
    pub half_width: f32,
}

#[serde(rename_all = "lowercase")]
pub enum LayoutDirection {
    Forward,
    Reverse,
}

pub struct StartFinish {
    pub section_id: String,
    pub fraction: f32,
    pub position: [f32; 3],
    pub rotation_xyzw: [f32; 4],
}
```

Semantics:

- `version` must equal `RACE_DATA_VERSION`. Unknown versions are rejected or
  reported as race-unavailable.
- `layout.layout_id` is the stable course selector. In the MVP it is the spawn
  centerline ID. Online identity uses this plus the pack hash.
- `layout.closed` is true for the MVP. The field allows a future
  point-to-point extension.
- `layout.direction` is the direction of travel for this course. The baked
  points are already ordered in travel direction.
- `layout.points` is the baked centerline reference, ordered in travel
  direction, with index 0 at the start/finish. Points include world-space Y
  sampled from the drivable surface. The last point is not duplicated.
- `layout.half_width` is the drivable road half-width. It matches
  `project.road.halfWidth`.
- `start_finish` is a baked copy/reference of a section feature. Its
  `section_id` and `fraction` preserve provenance. Its position must match
  `layout.points[0]` within tolerance. Its rotation is normalized.
- `grid` is pole-first and contains up to 8 logical slots. Slots are derived
  race setup, not track geometry. The renderer supports one local car plus
  seven remote rigs.
- `sectors` uses the existing `SectorSpec` type and covers `[0, 1]` exactly
  once, in order.

`start_finish`, `grid`, and `sectors` are siblings of `layout`, not children of
it.

### 4.2 Pack plumbing

- Add `ChunkKind::Race` in `crates/sim-types/src/asset.rs`.
- Add an optional `race` entry to `EntryPoints`. Make it
  `#[serde(default)]` so old packs still deserialize.
- Add `Pack::track_race_owned()` in `crates/sim-asset/src/lib.rs`:
  - return `Ok(None)` when the entrypoint is absent;
  - otherwise deserialize and return `RaceData`.
- Keep the global `SCHEMA_VERSION` at 3. It is shared by car and track packs,
  and committed car packs cannot all be rebuilt from repository sources. Use
  `RaceData.version` for evolution.
- Mirror `ChunkKind`, `EntryPoints.race`, and `RaceData` in
  `web/src/assets/types.ts` and `web/src/contracts/race.ts`.
- Add `"race"` to `chunkKindForEntrypoint` in `web/src/assets/loader.ts`; it is
  not a required entrypoint for tracks.
- Update every `EntryPoints` constructor with `race: None`; source and AC
  tracks do not emit race data in this batch.

Manifest shape when a layout is present:

```json
{
  "entrypoints": {
    "physics": null,
    "visual": 0,
    "collision": 1,
    "metadata": 2,
    "audio": null,
    "texture": null,
    "race": 3
  },
  "chunks": [
    { "kind": "Visual", "...": "..." },
    { "kind": "Collision", "...": "..." },
    { "kind": "Metadata", "...": "..." },
    { "kind": "Race", "...": "..." }
  ]
}
```

### 4.3 Track-ed source manifest

Track-ed exports `sim-track-glb-v2` with an optional `race` block:

```json
{
  "schema": "sim-track-glb-v2",
  "kind": "track",
  "name": "Loop Fixture",
  "length": 360.0,
  "spawn": { "position": [0, 0, 0], "rotation_xyzw": [0, 0, 0, 1] },
  "race": {
    "version": 1,
    "layout": {
      "layout_id": "main",
      "closed": true,
      "direction": "forward",
      "points": [[0, 0, 0]],
      "half_width": 5.0
    },
    "start_finish": {
      "section_id": "main",
      "fraction": 0.0,
      "position": [0, 0, 0],
      "rotation_xyzw": [0, 0, 0, 1]
    },
    "grid": [],
    "sectors": []
  },
  "visual": { "...": "..." },
  "collision": { "...": "..." },
  "surfaces": []
}
```

`race` is optional. A v1 export without it remains valid.

### 4.4 Identity

The track identity for the later online-contact handshake is:

```text
id + source_hash + layout_id
```

`source_hash` already covers the track JSON, including race data, and the GLB.
Expose a hex representation through `DriveViewer.info()` and a web loader
helper. Do not change the relay protocol in this batch.

### 4.5 Contract registry and agent process

The asset schema row in `AGENTS.md` and `agents/contracts.md` becomes:

| Contract | Canonical | Mirror | Consumers | Owner |
|---|---|---|---|---|
| Asset schema v3 / pack format (optional Race chunk) | `crates/sim-types/src/asset.rs`, `crates/sim-types/src/race.rs` | `web/src/assets/types.ts`, `web/src/contracts/**` | `sim-asset`, `asset-compiler`, `track-pack`, `track-pack-wasm`, `sim-render`, `web/src/assets` | architect |

`SCHEMA_VERSION` remains 3. The additive chunk carries `RaceData.version`; the
exception is recorded in ADR-0011. Existing packs remain readable, but old
application builds cannot read packs that contain `ChunkKind::Race`.

The architect updates canonical sources and mirrors first. Workers treat these
files as read-only. If a worker needs a further contract change, it returns
`BLOCKED` with:

```json
{
  "contract_change": {
    "canonical": "crates/sim-types/src/asset.rs",
    "mirrors": [
      "web/src/assets/types.ts",
      "web/src/contracts/race.ts"
    ],
    "reason": "Add optional Race chunk/entrypoint and RaceData",
    "breaking": false
  }
}
```

`breaking: false` covers existing packs; the one-way reader impact is recorded
in ADR-0011. A full two-way compatibility change would require a global
`SCHEMA_VERSION` bump and a complete pack rebuild.

Contract ownership is directory-based under ADR-0012. Adding a mirror under
`web/src/contracts/**` does not require an agent-config change or a server
restart; the architect adapter already allows the root.

## 5. Data flow

```text
track-ed project (spawn.centerline closed)
  -> implicit one-leg layout
  -> baked centerline layout path
  -> track.json (sim-track-glb-v2)
  -> track-pack -> LSIM pack with optional Race chunk + entrypoint
  -> sim-asset -> RaceData
       -> sim-worker (future AI and lap logic)
       -> DriveViewer.race() (future race manager)
       -> identity (id + source_hash + layout_id) for online
```

## 6. Producer: track-ed

Track-ed currently has no start/finish or lap route. The project model has
centerlines, junctions, a spawn (`centerline`, `fraction`, `lateral`), road
width, and bakes a per-section visual racing line.

### 6.1 Derivation

If `project.spawn.centerline` is closed:

1. Sample the centerline with the existing spline sampler.
2. Resample to approximately 2 m spacing.
3. Sample asphalt height for each point with the existing surface height
   helper.
4. Rotate the array so the point nearest `spawn.fraction` becomes index 0.
5. Keep the centerline point order as the travel direction. Verify the spawn
   rotation agrees with the route tangent; reverse only if the data is
   inconsistent.
6. Build the section-local start/finish anchor from the spawn centerline,
   fraction, and tangent. Do not use the spawn's lateral offset for the line.
7. Build 8 staggered logical grid slots behind the line, clamped inside
   `road.halfWidth`.
8. Build 4 equal sectors.
9. Set `layout_id` to the spawn centerline ID.

If the spawn centerline is open, omit `race` and emit a warning. Ordinary
export must still succeed.

### 6.2 Validation

- Points are finite and the route is closed.
- No duplicate consecutive points; spacing is within a sane range.
- `half_width` is positive and wider than the car.
- `start_finish.position` matches `points[0]`.
- Every route point lies on a drivable surface, preferably asphalt.
- Grid slots are finite, behind the start line, inside the road, and do not
  overlap.
- Sectors are ordered, contiguous, and cover `[0, 1]`.
- The route does not jump between disconnected sections.

### 6.3 Section features

The start/finish line and grid markings are section features. In the MVP they
are not separately authored:

- the spawn supplies the implicit start/finish station;
- grid slots are derived;
- no painted grid markings are exported.

A later `track-ed/project@6` can add optional section-level `start_finish` and
grid markings, and allow a layout to select from multiple markers.

### 6.4 Optional start/finish presentation

Bake a visible `start_finish` mesh into the GLB as a section decoration and
include it in `visual.include`; exclude it from collision. This is useful but
not a blocker for the data contract.

### 6.5 Release

Track-ed is a submodule. Land the changes in the track-ed repository, run its
tests, then bump `web/track-ed-src` in this repository.

## 7. Compiler: track-pack

Files: `crates/track-pack/src/lib.rs`, `crates/track-pack/tests/byte_compile.rs`.

- Accept `sim-track-glb-v1` and `sim-track-glb-v2`.
- Parse the optional `race` block.
- Validate the layout, start/finish, grid, and sectors.
- Archive `RaceData` and append a Race chunk.
- Set `entrypoints.race` when the chunk exists.
- Keep the output deterministic. The existing source hash covers the race data.
- Reject invalid race data with actionable errors.

Tests:

- v1 source without race compiles and has no Race entrypoint;
- v2 source with a valid race compiles and round-trips;
- invalid data is rejected (non-closed layout, too few points, non-finite
  values, out-of-range sectors, grid outside the road, start/finish not at
  route point 0);
- identical inputs produce identical bytes;
- changing race data changes the source hash.

## 8. Runtime and web

### 8.1 `sim-render`

- Read the optional `RaceData` in `DriveViewer.create`.
- Store it on `DriveViewer`.
- Add `DriveViewer.race()` returning JSON:
  - `{"available": false}` when absent;
  - otherwise `layout`, `start_finish`, `grid`, and `sectors`.
- Extend `info()` or telemetry with:
  - `raceAvailable`
  - `layoutId`
  - `trackSourceHash` (hex)
- Do not change physics or the SAB layout.

### 8.2 Web

- Add `web/src/contracts/race.ts` with the `RaceData` mirror and a JSON validator.
- Extend `web/src/assets/types.ts` for `ChunkKind` and `EntryPoints.race`.
- Add `trackIdentity()` for the future room handshake.
- Optionally add a small nearest-point/progress helper for the later race
  manager and recovery logic.
- Note: old local track-ed packs may lack race data. Re-exporting refreshes
  them. If stale local packs are a concern, bump
  `TRACK_ED_TRACK_CACHE_KEY` to `lapster.track-ed.v2`.

## 9. Fixtures and demo

- Add a committed race-loop fixture generated from track-ed's loop project.
  Use it for compiler tests, runtime tests, and the later AI/online batches.
- Keep `fixtures/tracks/minimal` as the “race unavailable” case.
- Default Skidpad has no race layout and must report unavailable.
- Follow-up: parse Assetto Corsa `ai/fast_lane.ai` or add a race block manually
  so Skidpad and real AC tracks can become race-ready.

## 10. Work breakdown

| Phase | Domain | Deliverable | Verify |
|---|---|---|---|
| 0 | architect | ADR-0011, ADR-0012, contract docs, scope record | doc review, `git diff --check` |
| 1 | architect | `RaceData`, `ChunkKind::Race`, entrypoint, accessor, mirrors | `cargo test -p sim-types -p sim-asset`, web type tests |
| 2 | assets | track-pack parse/validate/write, races in packs | `cargo test -p track-pack`, `cargo test -p asset-compiler` |
| 3 | editor | track-ed implicit layout, centerline path, grid, sectors, export v2 | `npm --prefix web/track-ed-src test` and build |
| 4 | render/web | `DriveViewer.race()`, info/identity, loader types | `cargo test -p sim-render`, wasm check, `npm --prefix web test` |
| 5 | assets/web | race-loop fixture, no-race fixture, Playwright smoke | `npm --prefix web run test:viewer` |
| 6 | architect | docs, submodule pin, full gate, PR | full gate below |

Dependencies:

- Phase 1 can start immediately.
- Phase 2 can be tested with a hand-authored `track.json` before track-ed
  produces one.
- Phase 3 runs in the track-ed repository in parallel.
- Phase 4 depends on phase 1 and phase 2.
- The submodule pin bump follows the track-ed merge.

## 11. Testing and acceptance

### Automated

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p sim-types -p sim-asset -p sim-core -p sim-render -p sim-worker \
  -p track-pack-wasm --target wasm32-unknown-unknown

npm --prefix web test
npm --prefix web run test:viewer
npm --prefix web/track-ed-src test
npm --prefix web/track-ed-src run build
npm --prefix web run build:viewer
```

Specific new tests:

- `sim-types`: `RaceData` archive round trip, version rejection, `EntryPoints`
  backward compatibility.
- `sim-asset`: pack without race returns `None`; pack with race returns the
  data; malformed race archive rejected.
- `track-pack`: v1/v2 acceptance, validation failures, deterministic bytes,
  source-hash changes.
- track-ed: closed/open spawn section, start/finish anchor, grid placement,
  sector coverage, layout-on-asphalt, deterministic export.
- web: `ChunkKind`/`EntryPoints` mirror, `race()` JSON shape, identity helper,
  race-unavailable behavior.
- Playwright: race-ready pack reports an available layout; no-race pack reports
  unavailable.

### Manual

- Open a track-ed closed-loop project, Test drive, and confirm `race()` returns
  the expected layout, start/finish, grid, and sectors.
- Open a no-race track and confirm the page still works and reports no race.
- Confirm the visual racing line is unchanged.
- Confirm pack recompilation is deterministic.

### Definition of done

- A closed track-ed project exports and compiles a pack with valid `RaceData`.
- A track without race data loads normally and reports unavailable.
- Layout, start/finish, grid, sectors, and identity are available to the later
  AI and online batches.
- Existing packs, car packs, and track-ed projects continue to load.
- All gate checks pass.
- Docs and the contract registry are updated.

## 12. Compatibility and rollout

- Existing packs load unchanged; they simply have no race entrypoint.
- New packs are not readable by older app builds. Static deployment is atomic,
  so this is acceptable.
- The track-ed source schema bumps to v2, but `track-pack` accepts v1 for old
  exports.
- The global pack schema stays at 3 to avoid invalidating committed car packs.
- Local track-ed packs may need re-export; optionally bump the local cache key.

## 13. Risks

| Risk | Mitigation |
|---|---|
| track-ed submodule coordination delays the pin | Land contract and compiler first; use synthetic track.json tests |
| Only closed spawn sections are supported | Document clearly; layouts/route legs are a follow-up |
| Default demo tracks have no race layout | Add a race-loop fixture; follow up with AC `fast_lane.ai` |
| Centerline layout is not the ideal AI line | Keep the racing line separate; add a `RacingLine` chunk later |
| Additive chunk breaks older readers | Atomic static deployment; existing packs remain readable |
| Stale local track-ed packs lack race data | Re-export; optionally bump cache key |
| Route/grid validation too permissive | Validate in track-ed, track-pack, and runtime tests |
| Start/finish and grid authoring deferred | Section-level fields are a follow-up; MVP uses spawn + derived grid |

## 14. Out of scope

- Full layout authoring, multi-layout export, route legs through sections.
- Section-level start/finish and grid-marking authoring.
- AI controller, multi-car worker, race manager, HUD, countdown.
- Car-to-car collision, contact netcode, server-authoritative lap validation.
- Pit lane, rolling starts, reverse layouts.
- AC `fast_lane.ai` import.
- Per-layout cameras and race configuration.

## 15. Follow-ups

1. **Section features**: add optional `start_finish` and grid markings to
   track-ed sections (`track-ed/project@6`), with multiple markers per section
   if layouts need different lines.
2. **AI racing**: multi-vehicle worker, shared collider, AI controller using
   `Layout` and grid slots.
3. **Race manager**: lap/sector timing, positions, results, start countdown.
4. **Online contact**: car-car collision, contact netcode, room handshake on
   `id + source_hash + layout_id`.
5. **Layouts**: route legs over sections, per-layout start/finish, sectors,
   spawns, cameras.
6. **Racing line**: optional `RacingLine` chunk or runtime derivation for AI.
7. **AC tracks**: `fast_lane.ai` import to make imported circuits race-ready.

## 16. References

- `GOALS.md`
- `SPEC.md`
- `PLAN.md`
- `docs/adr/0012-contract-roots-and-agent-permissions.md`
- `docs/adr/0011-race-layout-race-chunk.md`
- `docs/adr/0008-defer-layouts-as-route-layer.md`
- `docs/adr/0006-track-ed-hosting-and-track-pack.md`
- `docs/track-ed-integration.md`
- `docs/multiplayer.md`
- `agents/contracts.md`
- `crates/sim-types/src/asset.rs`
- `crates/sim-asset/src/lib.rs`
- `crates/track-pack/src/lib.rs`
- `crates/sim-render/src/lib.rs`
- `web/track-ed-src/src/kernel/bake.ts`
- `web/track-ed-src/src/export/lapster/manifest.ts`
- `web/src/assets/types.ts`
- `web/src/contracts/race.ts`
- `web/src/assets/loader.ts`
