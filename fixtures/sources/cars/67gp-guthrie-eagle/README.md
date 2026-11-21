# Guthrie Eagle 67GP — Speed Dreams source

This directory holds the tracked source files for Lapster's Speed Dreams 67GP
car import. The baked runtime pack is `fixtures/cars/67gp-guthrie-eagle/`.

## Attribution

- **Title:** Guthrie Eagle 67GP
- **Author:** Copyright 2018 CHAD (Chad Phillips). The XML header carries the
  earlier copyright notice `(C) 2017 by C Phillips`.
- **Source:** https://forge.a-lec.org/speed-dreams/67gp-guthrie-eagle
- **Source commit:** `01c82922ba16697e1a7d4766013ebadea26bd72c`
- **License:** Free Art License 1.3 (FAL 1.3)
- **License URL:** https://artlibre.org/licence/lal/en/
- **Local license copy:** `LICENSE-FAL-1.3.txt`
- **Downloaded:** 2026-11-21

Required credit line:

> Guthrie Eagle 67GP by CHAD (Chad Phillips), Free Art License 1.3, via Speed
> Dreams.

FAL requires modified copies to remain under FAL or a compatible license and to
identify the modifications. Lapster's modifications are listed below.

This attribution covers copyright only. Eagle and related marks belong to their
respective owners. Lapster does not imply manufacturer affiliation.

## Files

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `body/67gp-guthrie-eagle.acc` | Speed Dreams body source | `8d5d0327aec15bf5980ee51a8ccfd4005b66a21e4d281f98048d61276bdcee22` |
| `body/67gp-guthrie-eagle.xml` | Car specification, physics, sounds, lights | `eaef74289d6641b9d3e2296c3bf57398f5abc96ba88462083a3c435e2813ee01` |
| `body/67gp-guthrie-eagle.png` | Exterior skin texture | `fd39802ee8a002a7acab0470537c6063b86dfe272767baf07a484b8c34ce6240` |
| `body/readme.txt` | Original author/license note | `f5dd407abe35e3d6d2e4a97728f0f7a16f9935df4c20b7c06eb9aa70586dcde7` |
| `wheel/wheel0.acc` | Speed Dreams front-right wheel source | `45a7e10ac61d87a88d2da59bb30191f9ed07b8fde158ee170e1d2b16964b7015` |
| `wheel/wheel3d.png` | Wheel texture | `0043975c9fffaf7ed1051f4fb417bd2a7ccc68f72726e28641b8ad67f1deed5a` |
| `LICENSE-FAL-1.3.txt` | Free Art License text | `0b1e1674f969b4389ecf9429bde45768ce91d0ecb4af35f360e87faceafe6879` |

`import.json` is Lapster-authored build configuration; it is not part of the
upstream source package. Auxiliary Speed Dreams files that are not needed by the
current import — alternate skins, driver models, steering-wheel models, gauge
textures, shadow texture, and sound samples — remain available in the upstream
repository at the commit above.

## Modifications by Lapster

- Exported the `.acc` body and wheel to glTF with Assimp 6.0.4.
- Rotated the body from Speed Dreams `+X` forward into Lapster `-Z` forward.
- Raised the body by 0.104 m as the initial suspension ride height.
- Assigned the transparent `Glass` material to `WIGLASS*` nodes.
- Added four wheel nodes from the per-car `import.json` axle, track, radius, and
  width values.
- Used a single wheel model (`wheel0`) for all four corners; the front-right
  and rear-right wheels get the 180-degree Y rotation.
- Imported the body and wheel textures.
- Mapped the source XML into runtime `CarPhysics` mass, engine, gearbox, brakes,
  steering, suspension, tire mu, and aero.
- Assimp 6.0.4 triangulates the assembled pack to 19,328 triangles; the earlier
  Assimp 5.3.x pipeline produced a different triangle count. Bounds, materials,
  and physics are unaffected.

For the conversion and remaining work, see
[`docs/speed-dreams-pipeline.md`](../../../../docs/speed-dreams-pipeline.md).
