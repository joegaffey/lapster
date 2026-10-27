# Skidpad 0.5 — Temporary Default Demo Track

- **Source:** `skidpad05.zip`, downloaded 2026-10-23.
- **License:** no license or readme was included in the archive.
- **Status:** temporary demo fixture. Replace or confirm redistribution rights before a
  public/commercial release.
- **File:** `pack.sima`, baked from the local Assetto Corsa mod files at scale `0.70`
  (30% track reduction). The real-scale car is unchanged; the UI length is 105 m.

The raw `.kn5` files are not checked in. The baked pack contains:

- 76,545 visual triangles, 30 draw calls.
- 43,885 collision triangles from the central pad, ASP surfaces, outer banked asphalt ring,
  and grass.
- One spawn derived from `AC_START_0`.
- Approximated material colours; KN5 textures are not imported.
- No tree/fence meshes and no wall/fence collision; ground collision includes the outer ring
  and grass.
- Grass friction is capped at 0.35 to keep it significantly lower-grip than asphalt.
- The concrete apron is offset 1.5 cm below the tarmac and thin road overlays 5 mm above it
  to avoid coplanar z-fighting.

The UI name is “Skidpad 0.5”.
