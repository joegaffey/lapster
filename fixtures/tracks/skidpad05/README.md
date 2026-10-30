# Skidpad 0.5 — Temporary Default Demo Track

- **Source:** `skidpad05.zip`, downloaded 2026-10-23.
- **License:** no license or readme was included in the archive.
- **Status:** temporary demo fixture. Replace or confirm redistribution rights before a
  public/commercial release.
- **File:** `pack.sima`, baked from the local Assetto Corsa mod files at original scale
  (`1.0`, 150 m UI length). The real-scale car is unchanged.

The raw `.kn5` files are not checked in. The baked pack contains:

- 76,543 visual triangles, 29 draw calls, 10 base-colour textures at a maximum of 512 px.
- 43,885 collision triangles from the central pad, ASP surfaces, outer banked asphalt ring,
  and grass.
- One spawn derived from `AC_START_0`.
- Base-colour DDS textures are decoded and imported; normal and detail maps are ignored.
  The grass and dirt/transition materials use textures from the Drift Tracks fixture,
  configured by `texture-overrides.json`. Their UVs are projected from world X/Z (`uvMode:
  "worldXz"`) to avoid stretching on the large ground meshes; the grass tile uses twice the
  previous world size, and the dirt tile scale is tuned separately for the Skidpad's larger
  ground surface. Mountains/terrain, metal
  barriers, and tent fabric keep approximated material colours instead of textures. The
  AC logo mesh is omitted. The multilayer `tarmac` material is approximated with
  `tracknew.DDS`, and terrain meshes are lowered 5 cm to avoid z-fighting with the banked
  rings. Distant background terrain uses a blue-grey color. Concrete apron UVs are scaled to 0.25 so the texture does not look excessively
  fine. Baked pack is about 11 MB.
- No tree/fence meshes and no wall/fence collision; ground collision includes the outer ring
  and grass.
- Grass friction is capped at 0.35 to keep it significantly lower-grip than asphalt.
- The concrete apron is offset 1.5 cm below the tarmac and thin road overlays 5 mm above it
  to avoid coplanar z-fighting.

The UI name is “Skidpad 0.5”.

Re-bake with:

```bash
cargo run -p asset-compiler -- bake-ac-track fixtures/real/tracks/skidpad05 \
  fixtures/tracks/skidpad05/pack.sima \
  --texture-overrides fixtures/tracks/skidpad05/texture-overrides.json
```
