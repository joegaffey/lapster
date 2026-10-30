# Drift Tracks

- **Source:** https://sketchfab.com/3d-models/drift-tracks-90a78772873b462aab3ed5fc5fc0c65e
- **Author:** The-Third-Dimension (https://sketchfab.com/muneeb.changezi1)
- **License:** CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/)
- **Source file:** `drift_tracks.glb` (7.1k triangles, 5.5k vertices, embedded textures)
- **Status:** imported fixture; mesh selectors, textures, and collision are defined in the GLB
  and `track.json`. Baked pack: 6,940 visual triangles, 5,160 collision triangles, 10
  textures, 10 materials.
- `textures/grass.png` and `textures/dirt.png` are extracted from the GLB so other track
  fixtures can reuse them through texture overrides. `grass.png` is a square crop of the
  grass-edge texture, avoiding the broad mowing bands in the ground texture. They remain
  under CC BY 4.0.

The source model contains the drift course, kerbs, grass, fences, checkpoints, start
marker, and scenery. Only drivable surfaces and barriers are used for collision. Checkpoint
flags, loose props (including the metal box), and the tree-wall billboard are excluded from
the visual import. The dockyard entrance posts and light pole are retained. GLB base-colour textures are imported and referenced by the baked materials,
and unused materials/textures are pruned from the pack. Kerbs use `"mountable": true`, which
keeps their top faces for wheel support but removes vertical faces so they do not act as
walls.

Re-bake with:

```bash
cargo run -p asset-compiler -- bake-glb-track fixtures/tracks/drift-sketchfab
```
