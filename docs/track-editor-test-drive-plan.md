# Track Editor → Test Drive Plan

Status: proposed  
Branch at time of writing: `feature/multiplayer-relay`  
Base commit: `5e23d8e Add continuous kerb and prop guide editing`  
Target effort: 5–8 focused hours for the browser-generated-pack path.

## 1. Goal

Add a **Test drive** action to the track editor that lets the current procedural track
be loaded directly by Drive, with full physics including kerb collision, without an
offline export/compile/import step.

Recommended approach: generate the `.sima` pack **in the browser** using the existing
Rust/rkyv code compiled to WASM. Reuse the same pack writer as `asset-compiler` rather
than reimplementing the format in TypeScript.

## 2. Non-goals

- No new offline track-source format.
- No `asset-compiler` CLI workflow for editor tracks.
- No sharing, publishing, or multiplayer support for editor tracks.
- No texture authoring or GLB import.
- No vehicle-solver changes. The physics engine already raycasts arbitrary triangle
  collision.

## 3. Current state

- `crates/sim-render/src/track_editor.rs` builds `EditorTrack`, containing
  `visual`, `metadata`, `collision`, `manifest`, `kerb_spans`, and `prop_lines`.
- Procedural collision currently includes asphalt, both shoulders, and terrain only.
  Kerb meshes are visual-only. See `track_editor.rs:340`.
- Procedural metadata has asphalt, grass, and dirt surfaces, but no kerb surface.
- `crates/asset-compiler/src/bake.rs` has a private `write_pack` plus chunk
  archiving/hashing helpers. It is the authoritative pack writer.
- `crates/sim-asset/src/lib.rs` can parse and validate packs but cannot write them.
- `web/drive/main.ts` loads track and car packs by URL, then passes the bytes to both
  `DriveViewer.create` and the physics worker.
- `web/src/assets/cache.ts` already provides `IndexedDbPackCache`.
- The driver, suspension, tyre, and surface-friction code already consume
  `TrackCollider` raycasts and `SurfaceSpec`.
- Existing `fixtures/tracks/drift-sketchfab` already includes kerb collision and is
  useful for a quick physics sanity check before implementation.

## 4. Target flow

```text
editor state
  -> TrackViewer.editor_track_pack()          (Rust/WASM)
  -> validated .sima bytes                    (shared sim-asset writer)
  -> IndexedDbPackCache key "editor-track"
  -> navigate to /web/drive/?track=editor
  -> loadPackFromBytes
  -> DriveViewer.create(trackBytes, carBytes, ...)
  -> sim-worker receives same trackBytes
  -> wheel raycasts hit kerb triangles with KERB surface
```

## 5. Implementation phases

### Phase 1: Add procedural kerb collision

Estimate: 30–60 minutes.

Files:

- `crates/sim-render/src/track_editor.rs`
- possibly `crates/sim-core/src/collider.rs` tests only

Tasks:

1. Add `const KERB_SURFACE: u16 = 3;`.
2. Add a `SurfaceSpec` named `"kerb"` with friction around `0.9` and roughness
   around `0.2`. Tune after driving.
3. Append the two kerb mesh buffers to `collision`:
   - `visual.mesh_buffers[4]` is `kerb_red`.
   - `visual.mesh_buffers[5]` is `kerb_white`.
   - Use `KERB_SURFACE`.
4. Confirm mesh ordering before indexing; it is currently:
   `asphalt`, `outer shoulder`, `inner shoulder`, `terrain`, `kerb_red`,
   `kerb_white`, `barriers`, `tires_blue`, `tires_white`.
5. Keep kerbs out of wall surfaces. `TrackCollider` only treats surfaces named
   `"wall"` as walls, so kerbs remain mountable.
6. If kerb side faces cause harsh raycast behavior, restrict collision to the kerb
   top quad or filter near-vertical triangles. Start with full kerb geometry and
   only filter if testing shows a problem.

Tests:

- In `track_editor.rs`, build a track with a known kerb span and create a
  `TrackCollider`.
- Raycast downward over the kerb top, ideally around lateral radius `6.4`.
- Assert:
  - the hit surface is `KERB_SURFACE`;
  - friction matches the kerb surface;
  - the hit height is above the asphalt/terrain and follows the ramped top;
  - the leading edge remains near asphalt height.

### Phase 2: Move the pack writer into `sim-asset`

Estimate: 1–2 hours.

Files:

- `crates/sim-asset/src/lib.rs`
- `crates/asset-compiler/src/bake.rs`

Tasks:

1. Move the reusable pack-writing logic from `bake.rs` into `sim-asset`:
   - `ChunkSpec { kind: ChunkKind, bytes: Vec<u8> }`
   - `write_pack(manifest, chunks) -> Result<Vec<u8>, PackError>`
   - generic `archive<T>(value) -> Result<Vec<u8>, PackError>`
   - `sha256`, `align_up`, and header writing helpers.
2. Add the minimum `PackError` variants for build/serialize failures.
3. Keep the same alignment, manifest stabilization loop, chunk hashing, and
   round-trip validation behavior.
4. Update `asset-compiler/src/bake.rs` to call the shared writer and remove its
   duplicate implementation.
5. Do not reimplement rkyv serialization in TypeScript.

Tests:

- Add a `sim-asset` unit test that writes a tiny pack and parses it back.
- Existing `asset-compiler/tests/round_trip.rs` must pass unchanged.
- `cargo test -p sim-asset -p asset-compiler`

### Phase 3: Expose editor pack export through WASM

Estimate: 30–90 minutes.

Files:

- `crates/sim-render/src/lib.rs`
- `crates/sim-render/src/track_editor.rs`

Tasks:

1. Add a `TrackViewer` method:

   ```rust
   pub fn editor_track_pack(&self) -> Result<Vec<u8>, JsValue>
   ```

   `Vec<u8>` is exposed to JS as `Uint8Array` by wasm-bindgen.

2. Regenerate the current `EditorTrack` from the stored editor state:

   ```rust
   track_from_control_points_with_spans(
       &self.editor_control_points,
       Some(&self.editor_kerb_spans),
       Some(&self.editor_prop_lines),
   )
   ```

3. Replace the placeholder manifest entrypoints with:
   - visual = 0
   - collision = 1
   - metadata = 2
4. Archive `TrackVisual`, `TrackCollision`, and `TrackMetadata` with the shared
   `sim_asset::archive` helper.
5. Set a stable `source_hash` from the editor-state JSON if practical. At minimum,
   ensure the manifest validates and the `id` remains `"procedural-drift-loop"`.
6. Keep the track `id` stable so Drive/telemetry and tests have a predictable name.

Tests:

- Add a Rust test that exports an editor pack and parses it with
  `sim_asset::Pack::from_bytes`.
- Assert:
  - kind is `Track`;
  - visual, collision, and metadata entrypoints exist;
  - metadata contains a surface named `"kerb"`;
  - collision contains triangles using the kerb surface id;
  - the exported pack round-trips through the parser.

### Phase 4: Add the editor Test drive handoff

Estimate: 30–90 minutes.

Files:

- `web/track-editor/index.html`
- `web/track-editor/main.ts`
- new `web/src/assets/editor-track.ts`

Tasks:

1. Add a shared module for editor-track storage:

   ```ts
   export const EDITOR_TRACK_ID = "editor";
   export const EDITOR_TRACK_CACHE_KEY = "lapster.editor-track.v1";
   ```

2. Add a `Test drive` button near `Reset camera` / `Hide overlay`.
3. Disable it until the editor is ready; re-enable after a successful rebuild.
4. On click:
   - call `editorViewer.editor_track_pack()`;
   - write the bytes with `IndexedDbPackCache.put(EDITOR_TRACK_CACHE_KEY, bytes)`;
   - navigate to `../drive/?track=editor`;
   - show errors in the editor status if export or storage fails.
5. Do not put the pack in the URL or localStorage. Use IndexedDB.
6. Preserve the selected car by omitting `car=` unless an explicit car override is
   already present.

### Phase 5: Load `track=editor` in Drive

Estimate: 30–90 minutes.

Files:

- `web/drive/main.ts`
- `web/src/assets/loader.ts` if a small helper is useful
- `web/src/assets/editor-track.ts`

Tasks:

1. Treat `trackParam === "editor"` as a special case before normal URL resolution.
2. Add `loadEditorTrackPack()`:
   - read `EDITOR_TRACK_CACHE_KEY` through `IndexedDbPackCache`;
   - fail with a clear error if absent or stale;
   - parse with `loadPackFromBytes` so hashes are verified.
3. Load the car pack normally.
4. Pass the same track bytes to `DriveViewer.create` and `PhysicsClient.create`.
5. Ensure the physics-worker path receives editor bytes; do not silently fall back
   to a different track.
6. Show a useful error if the user opens `?track=editor` without first exporting.
7. Optionally expose the editor package version/time in the Drive asset label.

### Phase 6: Verification and regression tests

Estimate: 1–2 hours.

Rust:

- `cargo fmt --all -- --check`
- `cargo clippy -p sim-asset -p sim-render -p asset-compiler --all-targets -- -D warnings`
- `cargo test -p sim-asset`
- `cargo test -p asset-compiler`
- `cargo test -p sim-render track_editor`

Browser:

- Rebuild everything:

  ```bash
  bash web/scripts/build-viewer.sh
  ```

- Add `web/tests/editor-drive.spec.ts`:
  1. open `/web/track-editor/`;
  2. wait for `__trackEditorReady`;
  3. click `Test drive`;
  4. wait for `/web/drive/?track=editor` and `__driveViewerReady`;
  5. assert no page errors;
  6. assert telemetry identifies the procedural track;
  7. assert `debug_info()` reports a kerb surface and kerb collision triangles;
  8. assert the car has four wheel contacts after settling.
- Keep the existing `web/tests/track-editor.spec.ts` green.
- Run:

  ```bash
  bash web/scripts/test-viewer.sh tests/track-editor.spec.ts tests/editor-drive.spec.ts
  ```

Manual acceptance:

- Open the editor, modify control points and kerb spans, click Test drive.
- Drive over kerbs on both sides.
- Confirm the car rides up the kerb ramp and rumbles, rather than hitting a vertical
  wall or launching.
- Confirm the physics worker path also works (`physics` not forced to `main`).

### Phase 7: Optional pack download

Estimate: 20–30 minutes.

- Add a secondary `Download .sima` action that saves the exact same exported bytes.
- Useful for sharing, fixtures, CI, and reproducing a bug outside IndexedDB.
- Do not make this the primary workflow.

## 6. Acceptance criteria

- The editor can send the current track to Drive with one click.
- No offline export, CLI command, or server-side processing is required.
- The generated pack validates through `sim_asset::Pack::from_bytes`.
- Procedural metadata includes a kerb surface.
- Procedural collision includes the kerb top geometry with that surface.
- Drive loads the editor pack in both main-thread and worker physics modes.
- A car can drive over both kerbs with suspension/contact physics and visible rumble.
- Existing track editor, pack loader, drive, and asset-compiler tests still pass.
- The editor bundle and WASM rebuild cleanly through `build-viewer.sh`.

## 7. Risks and notes

- **Duplicate writer:** do not keep separate pack writers. Move the authoritative
  writer into `sim-asset` and use it from both `asset-compiler` and WASM.
- **WASM size:** adding the writer uses code already present in the dependency graph;
  measure bundle growth after Phase 2.
- **Stale IndexedDB data:** use a versioned cache key and show a clear error if Drive
  is opened without a recent export.
- **Kerb side faces:** full kerb geometry is fine for raycasts, but if side faces
  produce bad contacts, generate collision from the top surface only.
- **Performance:** kerbs add triangles along the track. Check `collisionTriangles`
  and worker step times before adding every possible detail.
- **Generated files:** always run `build-viewer.sh`; do not hand-edit
  `web/track-editor/main.js` or `web/drive/main.js`.
- **Current commit:** the working tree was clean at `5e23d8e` when this plan was
  written. Check for newer commits before starting.

## 8. Fresh-session prompt

Use this to start a new session:

> Read `docs/track-editor-test-drive-plan.md`. Implement it phase by phase, starting
> with Phase 1. Reuse the Rust pack writer by moving it into `sim-asset`; do not write
> a TypeScript `.sima` writer. After each phase, run the listed tests. Keep changes
> uncommitted until I ask to push/deploy.
