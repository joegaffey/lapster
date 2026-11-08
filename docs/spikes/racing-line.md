# Spike: racing-line

## Objective

Prototype a spline-based, subtly darker racing line on the generated
(procedural/editor) track surface. The line should be curvature-biased toward
the inside of corners, blended softly into the asphalt with vertex colours, and
visible on the drive page first and the track editor as a consequence of the
shared generator.

Textures, physics, collision, asset-schema, and other contract changes are out
of scope.

> Process note: the spike agent reached its step limit after committing the
> implementation and running verification, but before writing this report. The
> architect completed the report from the verified handoff, re-ran the relevant
> checks, and performed local browser validation.

## Interfaces tested

- `crates/sim-render/src/track_editor.rs` — procedural track generation,
  `MeshBuffers`, materials, mesh submission, editor pack export.
- `crates/sim-render/src/lib.rs` — `TrackViewer` / `DriveViewer` mesh lookup and
  visibility toggles, editor rebuild path.
- `web/src/ui/dev-menu.ts` — optional drive and track-editor visibility toggle.
- No contract files or pack-schema types were changed.

## Proposed changes

- Branch: `spike/racing-line`
- Commits: `50da39045abca10979e99a8d96a5aa03a429fb27` (implementation), plus the
  report commit that contains this file.
- Files:
  - `crates/sim-render/src/track_editor.rs`
  - `crates/sim-render/src/lib.rs`
  - `web/src/ui/dev-menu.ts`
  - `docs/spikes/racing-line.md`
- Summary:
  - Added deterministic `racing_line_offsets(points)`: signed turn from
    consecutive tangents, curvature-scaled offset toward the inside, clamped to
    `+/-4.1 m`, then circular 1-2-1 smoothing (16 passes).
  - Added `racing_line_ribbon(points, offsets)`: a 3.2 m wide, 7-vertex
    cross-section ribbon with a wide dark core (`[0.016, 0.016, 0.021]`)
    feathered to asphalt (`[0.045, 0.045, 0.05]`) over the outer edges. It sits
    `0.02 m` above the interpolated asphalt plane to avoid z-fighting.
  - Appended a `racing_line` material (index 12) and visual-only `racing_line`
    mesh; collision generation is untouched.
  - Added `racing_line_visible()` / `set_racing_line_visible(bool)` to
    `TrackViewer` and `DriveViewer`, preserved across editor rebuilds.
  - Added optional “Racing line” checkboxes to the drive and track-editor dev
    menus.
  - Added four focused Rust tests: offset clamp, corner-inside/straight-centre
    bias, mesh-above-asphalt/within-lane placement, and mesh/material presence.
- Follow-up revision from user feedback (“a lot wider and unbroken”):
  - Line width increased from 1.0 m to 3.2 m, with a ~1.9 m dark core and a
    feathered edge on each side.
  - Lift increased from `0.006 m` to `0.02 m` to remove distance z-fighting.
  - Offset clamp derived from the asphalt width and line half-width
    (`6.0 - 1.6 - 0.3 = 4.1 m`) so the ribbon cannot overhang the asphalt.
  - Smoothing increased from 10 to 16 passes, and the cross-section from 5 to 7
    vertices, for a smoother, continuous band.

## Verification

| Command | Passed |
|---|---|
| `cargo fmt --all --check` | yes |
| `cargo test -p sim-render` (42 tests) | yes |
| `cargo test --workspace` | yes |
| `cargo clippy --workspace --all-targets -- -D warnings` | yes |
| `cargo check -p sim-types -p sim-asset -p sim-core -p sim-render -p sim-worker --target wasm32-unknown-unknown` | yes |
| `npm test` (37 tests) | yes |
| `npm run build:viewer` | yes |
| Local browser validation: track editor preview | yes |
| Local browser validation: drive page via Test drive (`?track=editor`) | yes |

Local validation notes:

- The editor scene reports `racing_line` with 1440 triangles / 847 vertices,
  material 12, and 14 draw calls total.
- The line appears as a wide, continuous darker band, biased to the inside of
  corners and near the centre on straights.
- The toggle hides and restores the line in both viewers.
- No console errors beyond the existing Windows `powerPreference` warning.
- Playwright screenshots are not suitable for this check here: the existing
  cyan tire barrier ring and editor guide overlay can be mistaken for the line
  at low zoom; toggling the line is the reliable A/B check.

## Findings

- The race line is generated in the shared editor generator, so it appears in
  the editor preview and in the drive page’s editor track pack without separate
  wiring.
- Offsets stay within `+/-4.1 m`; with the `1.6 m` line half-width this leaves
  at least `0.3 m` to the `+/-6 m` asphalt edge.
- The ribbon follows the rendered asphalt plane via edge-height interpolation
  rather than raw terrain height, avoiding the flat-quad mismatch on curved
  terrain.
- The feature is visual-only: no collision surfaces, surface IDs, physics, or
  asset-schema changes.
- Cost is small: one extra draw call and 1.4k triangles.
- Imported asset packs are unaffected; this spike generates a line only for
  procedural/editor tracks.

## Risks

- `RACING_LINE_MATERIAL` is a hard-coded index (`12`); reordering the material
  list would silently point the mesh at the wrong material.
- Opaque scene meshes may cast static shadows by default; the line’s shadow
  contribution should be checked in the dev preview.
- The 3.2 m width may be too broad for tight tracks with narrow asphalt; width
  could be made track-scale aware.
- The dev-menu additions were not exercised by the node test suite or an
  automated browser test.
- No dev-preview / rig validation yet; the local site does not exercise the
  Pages COOP/COEP path.

## Debt

- New debt introduced by the spike:
  - Material lookup by constant index instead of by name.
  - Racing-line geometry lives in `track_editor.rs`; if it is promoted, consider
    a small reusable track-line module and a name-based material lookup.
  - Visibility is in-memory only; no persisted user preference yet.

## Recommendation

`test`

Validate on the dev preview (drive page with an editor track and the track
editor) before deciding whether to promote. If accepted, promote through the
strict workflow: name-based material lookup, an explicit shadow-caster decision,
and width tuning for narrow tracks.

## Open questions

- Does the line read as wide and continuous in the dev preview on the dev
  preview target browser/GPU?
- Should the line be hidden from static shadow casting?
- Should the line width/contrast vary with speed, camera distance, or track
  scale?
- Should the toggle persist as a user setting across sessions?
- Should imported asset packs eventually get a generated line when centerline or
  sector data is available?

## Sign-off

Spike sign-off

- Spike: `spike/racing-line`
- Interfaces tested: `track_editor.rs`, `lib.rs`, `dev-menu.ts`
- Decision: `promote` — local validation passed and the user approved testing
  on `dev`; the changes were re-applied to `dev` (the spike branch was not
  merged directly).
- Verification: workspace tests, clippy, wasm check, `npm test`, viewer build,
  and local Chromium validation all passed. See the table above.
- Architect: completed report and local validation; reviewed the diff and the
  four new tests.

```json
{
  "spike": "racing-line",
  "status": "COMPLETE",
  "objective": "Prototype a spline-based, wide, darker racing line on the generated procedural track surface using curvature-driven vertex-colour blending, with no texture, physics, collision, or contract changes.",
  "interfaces_tested": [
    "crates/sim-render/src/track_editor.rs",
    "crates/sim-render/src/lib.rs",
    "web/src/ui/dev-menu.ts"
  ],
  "changes": {
    "branch": "spike/racing-line",
    "commits": [
      "50da39045abca10979e99a8d96a5aa03a429fb27"
    ],
    "files": [
      "crates/sim-render/src/track_editor.rs",
      "crates/sim-render/src/lib.rs",
      "web/src/ui/dev-menu.ts",
      "docs/spikes/racing-line.md"
    ],
    "summary": "Added deterministic curvature-biased racing-line offsets and a wide (3.2 m) soft-edged vertex-coloured ribbon above the asphalt, a racing_line material/mesh, focused tests, and viewer/dev-menu visibility toggles."
  },
  "verification": [
    {"cmd": "cargo fmt --all --check", "passed": true},
    {"cmd": "cargo test -p sim-render", "passed": true},
    {"cmd": "cargo test --workspace", "passed": true},
    {"cmd": "cargo clippy --workspace --all-targets -- -D warnings", "passed": true},
    {"cmd": "cargo check -p sim-types -p sim-asset -p sim-core -p sim-render -p sim-worker --target wasm32-unknown-unknown", "passed": true},
    {"cmd": "npm test", "passed": true},
    {"cmd": "npm run build:viewer", "passed": true},
    {"cmd": "local Chromium: track editor + drive page", "passed": true}
  ],
  "findings": [
    "The racing line is generated by the shared editor track generator, so it appears in both the editor preview and drive-page editor track packs.",
    "Offsets stay within +/-4.1 m, leaving at least 0.3 m to the +/-6 m asphalt edge with a 1.6 m line half-width.",
    "The 3.2 m ribbon sits 0.02 m above the interpolated asphalt plane, stays within the lane, and is visual-only.",
    "Vertex colours keep a wide dark core [0.016,0.016,0.021] and feather to asphalt [0.045,0.045,0.05] at the edges.",
    "Visibility toggles work in TrackViewer and DriveViewer and are present in the dev menus.",
    "Cost is one draw call and 1440 triangles."
  ],
  "risks": [
    "Material index 12 is hard-coded; reordering materials would break the reference.",
    "The line may still cast a static shadow because opaque scene meshes are shadow casters by default.",
    "The 3.2 m width may be too broad for tight tracks with narrow asphalt.",
    "Dev-menu UI additions are not covered by automated tests.",
    "No dev-preview or rig validation yet."
  ],
  "recommendation": "test",
  "open_questions": [
    "Does the line read as wide and continuous in the dev preview on the target browser/GPU?",
    "Should the line be hidden from static shadow casting?",
    "Should the line width/contrast scale with speed, camera distance, or track scale?",
    "Should the visibility toggle persist as a user setting?",
    "Should imported asset packs get a generated line when centerline data is available?"
  ]
}
```
