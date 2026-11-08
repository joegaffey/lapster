# Spike: track-textures

## Objective

First texture pass for the generated (procedural/editor) track surface: add a
deterministic, deliberately subtle procedural tarmac texture, subtle grass and
dirt detail, and make the racing line use the same texture and UV mapping so it
blends into the road instead of looking like a flat overlay.

Imported-track changes, physics, collision, and asset-schema changes are out of
scope for this pass.

## Interfaces tested

- `crates/sim-render/src/track_editor.rs` — procedural texture generation,
  `TrackVisual.textures`, `MaterialRef.base_color_texture`, road ribbon UVs,
  racing-line ribbon UVs/colours, manifest stats.
- `crates/sim-render/src/track.wgsl` — asphalt band sampling and blend-edge UV
  handling.
- No contract files, pack schema, collision, or physics changes.

## Proposed changes

- Branch: `spike/track-textures`
- Commits: `472f38c` (tarmac + racing-line blend), `a5c3008` (subtle grass
  and dirt), `d6cfd05` (subtler tarmac), plus the report commits.
- Files:
  - `crates/sim-render/src/track_editor.rs`
  - `crates/sim-render/src/track.wgsl`
  - `docs/spikes/track-textures.md`
- Summary:
  - Added a deterministic, tileable 256x256 tarmac RGBA texture
    (`tarmac_procedural`) using wrapped value noise plus fine grain and low
    frequency wear patches. The contrast is deliberately narrow (roughly sRGB
    0.62–0.79) so the asphalt detail is subtle.
  - Added it as `TrackVisual.textures[1]` and assigned it to the road
    (`dither_shoulder`) and `racing_line` materials.
  - Added a deterministic, tileable 256x256 grass RGBA texture
    (`grass_procedural`), tuned to the previous dark grass base colour, and
    assigned it to the `grass` material at texture index 2. Terrain UVs tile at
    roughly 8 m so the grass detail is visible without obvious repetition.
  - Dirt shoulders now receive a subtle 25% tarmac-texture modulation in the
    road blend path instead of a completely flat colour.
  - Road UVs now tile by physical distance: whole-number tiles along the loop
    (`TARMAC_TILE_METERS = 7.0`) and `TARMAC_UV_ACROSS = 4.0` across the ribbon,
    keeping the start/finish seam texture-continuous.
  - Shader asphalt band is now
    `vec3(0.045, 0.045, 0.05) * (TARMAC_GAIN * texel.rgb)`, with the blend edge
    computed from `uv.y / TARMAC_UV_ACROSS`.
  - Racing-line UVs now use the same road-aligned mapping, and its vertex
    colours are pre-multiplied by `TARMAC_GAIN`. At the feathered edges the line
    therefore resolves to exactly the same textured colour as the surrounding
    asphalt, fixing the previous “pasted-on” edge.
  - Corrected `AssetStats.textures` from the old hard-coded `0` to the actual
    texture count.
  - Added tests for texture determinism/opacity/subtle contrast, noise
    tile-edge wrapping, texture/material assignment (road, racing line, grass),
    and scaled road/terrain UVs.

## Verification

| Command | Passed |
|---|---|
| `cargo fmt --all` | yes |
| `cargo test -p sim-render` (46 tests) | yes |
| `cargo clippy -p sim-render --all-targets -- -D warnings` | yes |
| `cargo check -p sim-render --target wasm32-unknown-unknown` | yes |
| Viewer build (`npm run build:viewer`, via `test:viewer`) | yes |
| Local Chromium: track editor preview | yes |
| Local Chromium: drive page after Test drive | yes |
| `npm run test:viewer -- tests/dev-menu.spec.ts` | build passed; Playwright web server could not start because port 8123 was already in use |

Visual notes:

- Tarmac detail is now subtle: a mostly uniform dark surface with gentle
  mottling rather than a strong noisy pattern.
- Grass keeps the previous dark green tone with fine mottling. Dirt shoulders
  are lightly textured rather than flat.
- The racing line still reads as a darker band and shares the road texture and
  edge colour, so it blends into the surface.
- Re-run “Test drive” after a rebuild to regenerate the cached editor track
  pack with the new textures.

## Findings

- A shared texture + shared UV mapping is what fixes the blend: the line’s
  feathered edge resolves to the same `asphalt * TARMAC_GAIN * texel` value as
  the adjacent road.
- Using whole-number tiles along the loop avoids a start/finish texture seam.
- Three procedural textures are embedded: the tree-card atlas, a 256 KiB
  tarmac texture, and a 256 KiB grass texture. Editor track packs grow by
  roughly 512 KiB; no schema change was needed.
- The grass texture is tuned to the previous `GRASS_GROUND_COLOR`, so the
  terrain tone is preserved while gaining subtle mottling. Dirt uses the tarmac
  texture at 25% strength, which keeps it subtle.
- Draw calls are unchanged. The road and racing line share the tarmac texture;
  the added cost is texture memory and sampling.
- The generated track is still self-contained and deterministic; no external
  texture assets or licensing dependencies were introduced.

## Risks

- `TARMAC_GAIN` and `TARMAC_UV_ACROSS` are duplicated in Rust and WGSL and must
  stay in sync.
- The pure-product asphalt formula is tuned for this texture’s brightness;
  changing the generated texture values requires retuning the gain.
- Tarmac is deliberately low-contrast now and may read as too subtle on some
  displays.
- Grass texture repetition (8 m tiles) may become visible from high camera
  angles or on very large tracks.
- Dirt shoulders reuse the tarmac pattern at reduced strength rather than a
  dedicated dirt texture.
- No automated visual regression covers the track surface; screenshot baselines
  are not available in this environment.
- The racing line still sits `0.02 m` above the asphalt to avoid z-fighting, so a
  close low-angle view may show a thin geometric edge even when the colour
  matches.
- Mobile texture/fill cost has not been measured.

## Debt

- New debt introduced by the spike:
  - Procedural texture generation lives in `track_editor.rs`; if promoted,
    consider moving reusable procedural textures into a small module.
  - Material and texture indices are still hard-coded constants.
  - Rust/WGSL constants are duplicated rather than generated or shared.
  - Dirt detail reuses the tarmac texture; a dedicated dirt texture and
    shared grass/dirt tiling constants are future work.

## Recommendation

`promote`

Reviewed on the dev/local preview and promoted through the strict workflow.
Tune `TARMAC_GAIN`, `TARMAC_TILE_METERS`, `TARMAC_UV_ACROSS`,
`TERRAIN_TILE_METERS`, and the dirt modulation strength as follow-ups. A
dedicated dirt texture and grass anti-repetition are the natural next steps.

## Open questions

- Is the asphalt now too subtle, or right at chase-camera speed and on mobile?
- Is the 8 m grass tiling noticeable from the chase or high editor cameras?
- Should dirt get its own texture rather than reusing the tarmac pattern?
- Should the road use a dedicated asphalt material instead of
  `dither_shoulder`?
- Should the racing line sit slightly lower, or use polygon offset, to reduce
  the close-range lift edge?
- Should texture constants live in one shared place instead of Rust + WGSL?

## Sign-off

Spike sign-off

- Spike: `spike/track-textures`
- Interfaces tested: `track_editor.rs`, `track.wgsl`
- Decision: `promote` — architect reviewed the diff; no contract, collision,
  physics, or schema changes. Changes were re-applied on `feat/track-textures`
  and promoted through `dev`; the `dev` → `main` PR is handed to @joegaffey.
- Verification: `cargo fmt --all --check`, workspace clippy, workspace tests,
  wasm check, and the viewer build passed. Web tests are delegated to CI; the
  local Playwright server port was occupied and screenshot baselines are
  environment-specific.
- Architect: reviewed the diff and gate; local visual validation passed on the
  editor and drive preview.

```json
{
  "spike": "track-textures",
  "status": "COMPLETE",
  "objective": "Add deterministic, subtle procedural tarmac, grass, and dirt detail to the generated track surface and blend the racing line into the same texture/UV mapping.",
  "interfaces_tested": [
    "crates/sim-render/src/track_editor.rs",
    "crates/sim-render/src/track.wgsl"
  ],
  "changes": {
    "branch": "spike/track-textures",
    "commits": [
      "472f38c",
      "a5c3008",
      "d6cfd05"
    ],
    "files": [
      "crates/sim-render/src/track_editor.rs",
      "crates/sim-render/src/track.wgsl",
      "docs/spikes/track-textures.md"
    ],
    "summary": "Generated subtle tileable tarmac and grass textures, added light dirt-shoulder modulation, aligned the racing-line UVs and colours with the textured asphalt, and added texture/material/UV tests."
  },
  "verification": [
    {"cmd": "cargo fmt --all", "passed": true},
    {"cmd": "cargo test -p sim-render", "passed": true},
    {"cmd": "cargo clippy -p sim-render --all-targets -- -D warnings", "passed": true},
    {"cmd": "cargo check -p sim-render --target wasm32-unknown-unknown", "passed": true},
    {"cmd": "viewer build via npm run test:viewer", "passed": true},
    {"cmd": "local Chromium track editor + drive preview", "passed": true}
  ],
  "findings": [
    "Tarmac detail is now subtle: a mostly uniform dark surface with gentle mottling.",
    "Grass keeps the previous dark green tone with fine mottling; dirt shoulders are lightly textured.",
    "Sharing the texture and road-aligned UVs makes the racing-line feather match the textured asphalt at the edges.",
    "Whole-number tiles along the loop avoid a start/finish texture seam.",
    "The tarmac and grass textures are embedded in editor track packs with no schema change; draw calls are unchanged.",
    "Editor packs grow by roughly 512 KiB from the two new 256x256 RGBA textures."
  ],
  "risks": [
    "TARMAC_GAIN and TARMAC_UV_ACROSS are duplicated in Rust and WGSL.",
    "The product-based asphalt formula is tuned for the current texture brightness.",
    "Tarmac may now be too subtle on some displays.",
    "Grass texture repetition may be visible from high camera angles.",
    "Dirt reuses the tarmac pattern instead of a dedicated dirt texture.",
    "No automated visual regression covers the surface.",
    "The racing line's 0.02 m lift can still show a close-range geometric edge.",
    "Mobile texture/fill cost is unmeasured."
  ],
  "recommendation": "test",
  "open_questions": [
    "Is the asphalt now too subtle, or right at chase-camera speed and on mobile?",
    "Is the 8 m grass tiling noticeable from chase or high editor cameras?",
    "Should dirt get its own texture rather than reusing the tarmac pattern?",
    "Should the road use a dedicated asphalt material?",
    "Should the racing line use a lower lift or polygon offset?",
    "Should texture constants be shared between Rust and WGSL?"
  ]
}
```
