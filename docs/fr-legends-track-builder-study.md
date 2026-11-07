# FR Legends Track Builder Study

Status: reference / backlog
Date: 2026-11-06
Source: Google AI Mode conversation "are fr legends graphic textured or shaded" and the
follow-up procedural track session.
Related code: `crates/sim-render/src/track_editor.rs`,
`crates/sim-render/src/track.wgsl`, `web/track-editor/`, `web/src/ui/dev-menu.ts`.

## 1. What the source chat concluded

- FR Legends uses a cel-shaded / toon-shaded base look: flat colours, bold ink outlines,
  and simplified lighting rather than detailed photorealistic textures.
- Terrain in the base game is flat-shaded: solid grass, asphalt, concrete, and mountain
  colours with minimal gradients and no tarmac grain, grass blades, or gravel texture.
- The modding community adds HD asphalt, grass, real-world banners, and photographic
  skyboxes, but those are mods, not the base art direction.
- The recommended web implementation is low-poly geometry, flat untextured colours, hard
  stepped lighting, and inverted-hull outlines.

## 2. What the chat's track demo built (Three.js)

- Catmull-Rom spline centreline with a Frenet-style frame and extruded track ribbon.
- Initially separate asphalt, dirt, and grass strips with duplicated boundary vertices to
  keep hard edges and avoid normal smoothing.
- Triangulated centre infield; later replaced by a simpler centre-fan/grid infill.
- Y offset and a dirt "skirt" to stop the track sinking under the grass.
- Curvature-based red/white kerbs with alternating vertex colours and a visible lip.
- Dirt blend experiments: multi-band geometry rings, then a screen-space 4x4 Bayer
  dither matrix to keep the edge crisp without linear alpha blending.
- Concrete barriers, tyre stacks, and low-poly five-sided trees with black inverted-hull
  outlines.
- Editor controls: track complexity, tree density, and a random track generator.

## 3. What Lapster's track builder does (Rust + wgpu)

- Catmull-Rom sampling with 120 samples and iterative curvature relaxation to enforce a
  minimum turn radius (`track_editor.rs`).
- One continuous ribbon with dirt / asphalt / dirt lateral bands; edge transitions are
  handled in the fragment shader rather than with separate strips.
- Full terrain grid with a hole under the track, plus a mountain backdrop. The infield is
  therefore already filled without ear clipping.
- Surface height offset avoids z-fighting; kerbs use curvature-driven spans, alternating
  red/white meshes, and riser triangles at jagged span ends.
- Props: concrete blocks and tyre stacks with inverted-hull outlines, placed along
  editable prop lines.
- Toon ramp with four steps, rim light, distance-scaled outlines, and collision surfaces
  exported with the visual pack for test drive.
- Trees are an editable prop kind alongside blocks and tyres: low-poly trunk cylinders
  with layered cone canopies, inverted-hull outlines, deterministic variety, and
  trunk-only collision. Near and far prop guide lines on each side let trees sit behind
  the barrier lines.
- Props are merged into vertex buffers; `TrackVisual.instances` is still empty.

## 4. Comparison summary

| Area | Chat prototype | Lapster track builder |
| --- | --- | --- |
| Engine | Three.js demo with wgpu/WGSL as theory | Native Rust `wgpu` + WGSL, WASM in browser |
| Track topology | Separate strips, duplicated boundary vertices | One continuous ribbon, shared vertices |
| Dirt/grass edge | Multi-band rings and/or Bayer dither | Bayer dither, now switchable with soft and hard modes |
| Lighting | Two-step toon | Four-step toon ramp |
| Kerbs | Curvature-based, red/white, raised | Curvature-based, separate red/white meshes, risers |
| Trees | Low-poly trunk + cones with outlines | Editable tree prop lines with low-poly trunk + cone canopies and outlines |
| Barriers/tyres | Recommended instancing | Merged prop geometry, no instancing yet |
| Scope | Visual demo | Visuals, collision surfaces, spawn, cameras, pack export, test drive |

## 5. Implemented during this study

- Added a dev-only track edge blend mode so the dithered shipped look can be compared
  against a soft gradient and a hard reference cut.
- Mode values are carried through `model.alpha.w` in the track shader: `1` dither,
  `2` soft, `3` hard cut.
- New `TrackViewer::set_track_blend_mode` / `track_blend_mode` API, preserved across
  editor rebuilds.
- Track editor exposes the control in the developer drawer, enabled with `?dev=1`
  (`web/src/ui/dev-menu.ts` `setupTrackDevMenu`).
- Playwright coverage in `web/tests/dev-menu.spec.ts` for dither, soft, hard, and
  persistence across a rebuild.

## 6. Backlog to pick up later

1. **Density-driven tree placement.** Trees are now paintable prop lines with low-poly
   trunk cylinders, cone canopies, and inverted-hull outlines. A later pass can place them
   automatically from curvature or density while keeping them off the drivable surface.
2. **More track-side props.** Clipping-point cones, banners/signs, and a start/finish
   gantry. Extend `PropKind` and the prop guide editor rather than adding a new placement
   system.
3. **Randomize and complexity controls.** Add a random control-point generator in
   `web/track-editor/main.ts` that respects the existing minimum-radius and offset-crossing
   constraints. A complexity slider can drive the number of control points.
4. **Extend the dev menu into a style lab.** Reuse `setupTrackDevMenu` for:
   two-step vs four-step toon ramp, prop outline on/off or thickness, dither scale / soft
   blend width, and a flat-colour vs textured-surface comparison.
5. **Instance barriers and tyres.** Worth doing only when prop counts grow; currently the
   merged buffers are small and draw calls are acceptable.
6. **Outer dirt skirt or bevel.** Low priority. The existing surface offset prevents
   z-fighting; a skirt would mainly help extreme camera angles or steep terrain.

## 7. Chat ideas not worth porting

- Ear-clipping the infield: the terrain grid with a track hole already solves it.
- Pipeline depth bias: the surface height offset already handles layering.
- Post-process Sobel edge detection: the project deliberately uses inverted-hull geometry.

## 8. How to resume this work

```sh
cargo fmt --check
cargo test -p sim-render --lib
cd web
npm run build:viewer
bash scripts/test-viewer.sh tests/dev-menu.spec.ts tests/track-editor.spec.ts
```

Open the comparison modes at:

```text
http://127.0.0.1:8123/web/track-editor/?dev=1
```

Deploy the current branch to the dev preview:

```sh
gh workflow run pages.yml -R joegaffey/lapster-dev -f ref=dev
```
